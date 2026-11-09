//! Remote repair of a fit by distance and over time.

use rufa_data::Info;
use rufa_data::catalog::group;

use super::apply;
use super::cycles::{self, Cycles, Spool};
use super::dealers::{SourceData, module_falloff, module_optimal, source_data};
use super::model::{Quantity, Request};
use super::sample;
use super::{Ctx, InputKey};
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, Slot, State};

pub const TIME_LIMIT: f64 = 2500.0;
const SAMPLES: usize = 50;
const DEPTH: u32 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Repper {
    pub index: usize,
    /// HP of one cycle at zero spool, shield then armor then hull.
    pub amount: [f64; 3],
    /// Seconds after the cycle starts that the repair lands.
    pub delay: f64,
    pub cycles: Option<Cycles>,
    pub spool: (f64, f64),
    pub optimal: f64,
    pub falloff: f64,
    pub drone: bool,
    /// Charge multiplier and shots of an ancillary armor repairer.
    pub paste: Option<(f64, u32)>,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn group_of(info: &impl Info, type_id: i32) -> i32 {
    info.get_type(type_id).map(|t| t.group_id()).unwrap_or(0)
}

/// Every active remote repairer of the fit. Ancillary repairers reload by
/// `ancillary`, the rest by `reload`.
pub fn reppers<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    calc: &Calculation,
    reload: bool,
    ancillary: bool,
) -> Vec<Repper> {
    let ids = ctx.ids;
    let mut out = Vec::new();
    for (index, (item, result)) in fit.items.iter().zip(&calc.items).enumerate() {
        if result.state < State::Active {
            continue;
        }
        match item.slot {
            Slot::DroneBay => {
                let n = f64::from(item.quantity);
                let amount = [
                    attr(result, ids.shield_bonus) * n,
                    attr(result, ids.armor_damage_amount) * n,
                    attr(result, ids.structure_damage_amount) * n,
                ];
                if amount.iter().sum::<f64>() <= 0.0 || item.quantity == 0 {
                    continue;
                }
                let Some(cycle) = cycles::drone_cycle(ids, result) else {
                    continue;
                };
                out.push(Repper {
                    index,
                    amount,
                    delay: if amount[0] > 0.0 { 0.0 } else { cycle },
                    cycles: Some(Cycles::forever(cycle, 0.0)),
                    spool: (0.0, 0.0),
                    optimal: f64::INFINITY,
                    falloff: 0.0,
                    drone: true,
                    paste: None,
                });
            }
            slot if Rack::of_slot(slot).is_some() => {
                let kind = group_of(ctx.info, item.type_id);
                let cycle = cycles::raw_cycle(ids, result);
                if cycle <= 0.0 {
                    continue;
                }
                let is_ancillary = matches!(
                    kind,
                    group::ANCILLARY_REMOTE_ARMOR_REPAIRER | group::ANCILLARY_REMOTE_SHIELD_BOOSTER
                );
                let (amount, delay, paste) = match kind {
                    group::REMOTE_SHIELD_BOOSTER | group::ANCILLARY_REMOTE_SHIELD_BOOSTER => {
                        ([attr(result, ids.shield_bonus), 0.0, 0.0], 0.0, None)
                    }
                    group::REMOTE_ARMOR_REPAIRER | group::MUTADAPTIVE_REMOTE_ARMOR_REPAIRER => (
                        [0.0, attr(result, ids.armor_damage_amount), 0.0],
                        cycle,
                        None,
                    ),
                    group::ANCILLARY_REMOTE_ARMOR_REPAIRER => {
                        let charged = item.charge.is_some();
                        let mult = if charged {
                            attr(result, ids.charged_armor_damage_multiplier).max(1.0)
                        } else {
                            1.0
                        };
                        let shots = cycles::shots(ids, item, result).unwrap_or(0);
                        (
                            [0.0, attr(result, ids.armor_damage_amount) * mult, 0.0],
                            cycle,
                            charged.then_some((mult, shots)),
                        )
                    }
                    group::REMOTE_HULL_REPAIRER => (
                        [0.0, 0.0, attr(result, ids.structure_damage_amount)],
                        cycle,
                        None,
                    ),
                    _ => continue,
                };
                if amount.iter().sum::<f64>() <= 0.0 {
                    continue;
                }
                let reloads = if is_ancillary { ancillary } else { reload };
                out.push(Repper {
                    index,
                    amount,
                    delay,
                    cycles: cycles::module_cycles(ids, item, result, reloads),
                    spool: (
                        attr(result, ids.repair_multiplier_bonus_max),
                        attr(result, ids.repair_multiplier_bonus_per_cycle),
                    ),
                    optimal: module_optimal(ctx, result),
                    falloff: module_falloff(ctx, result),
                    drone: false,
                    paste,
                });
            }
            _ => {}
        }
    }
    out
}

impl Repper {
    fn total(amount: [f64; 3]) -> f64 {
        amount.iter().sum()
    }

    /// Repair per second averaged over the cycle plan at full spool.
    pub fn declared(&self) -> f64 {
        let Some(cycles) = &self.cycles else {
            return 0.0;
        };
        let avg = cycles.average();
        if avg <= 0.0 {
            return 0.0;
        }
        let spool = cycles::spool_bonus(
            self.spool.0,
            self.spool.1,
            cycles.runs[0].active,
            Spool::Scale(1.0),
        );
        Repper::total(self.amount) * (1.0 + spool) / avg
    }
}

/// Repair of one repper laid out in time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Line {
    pub rps: Vec<(f64, f64)>,
    pub events: Vec<(f64, f64)>,
}

fn push_change(list: &mut Vec<(f64, f64)>, t: f64, v: f64) {
    if list.last().is_some_and(|(lt, _)| *lt == t) {
        list.pop();
    }
    if list.last().is_some_and(|(_, lv)| *lv == v) {
        return;
    }
    list.push((t, v));
}

pub fn timeline(reppers: &[Repper], until: f64, ancillary_reload: bool) -> Vec<Line> {
    let mut out = Vec::with_capacity(reppers.len());
    for r in reppers {
        let mut line = Line::default();
        let Some(cycles) = &r.cycles else {
            out.push(line);
            continue;
        };
        let mut segments: Vec<(f64, f64, f64)> = Vec::new();
        let mut events: Vec<(f64, f64)> = Vec::new();
        let mut without_reload = 0u32;
        cycles.walk(until, |t, run, nonstop| {
            without_reload += 1;
            let spool =
                cycles::spool_bonus(r.spool.0, r.spool.1, run.active, Spool::Cycles(nonstop));
            let mut amount = Repper::total(r.amount) * (1.0 + spool);
            if let Some((mult, shots)) = r.paste
                && !ancillary_reload
                && without_reload > shots
                && mult > 0.0
            {
                amount /= mult;
            }
            events.push((t + r.delay, amount));
            if run.active > 0.0 {
                segments.push((t, t + run.active, amount / run.active));
            }
            if run.reload {
                without_reload = 0;
            }
            t <= until
        });
        let mut end = None::<f64>;
        for (start, stop, rps) in segments {
            if let Some(e) = end
                && start > e
            {
                push_change(&mut line.rps, e, 0.0);
            }
            push_change(&mut line.rps, start, rps);
            end = Some(stop);
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut total = 0.0;
        for (t, v) in events {
            total += v;
            line.events.push((t, total));
        }
        out.push(line);
    }
    out
}

impl Line {
    pub fn rps_at(&self, t: f64) -> f64 {
        let i = self.rps.partition_point(|(lt, _)| *lt <= t + 1e-9);
        if i == 0 { 0.0 } else { self.rps[i - 1].1 }
    }

    pub fn total_at(&self, t: f64) -> f64 {
        let i = self.events.partition_point(|(lt, _)| *lt <= t + 1e-9);
        if i == 0 { 0.0 } else { self.events[i - 1].1 }
    }
}

pub struct Prepared {
    src: SourceData,
    reppers: Vec<Repper>,
}

impl Prepared {
    pub fn is_empty(&self) -> bool {
        self.reppers.is_empty()
    }
}

pub fn prepare<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
) -> Prepared {
    Prepared {
        src: source_data(ctx, fit, calc),
        reppers: reppers(
            ctx,
            fit,
            calc,
            req.options.reload,
            req.options.ancillary_reload,
        ),
    }
}

fn application(r: &Repper, src: &SourceData, d: Option<f64>, req: &Request) -> f64 {
    if let Some(d) = d {
        if req.options.lock_range && d > src.max_target_range {
            return 0.0;
        }
        if r.drone && req.options.drone_range && d > src.drone_control_range {
            return 0.0;
        }
    }
    if r.drone {
        return 1.0;
    }
    apply::unerr(apply::range_factor(r.optimal, r.falloff, d, true))
}

fn value(
    p: &Prepared,
    lines: Option<&[Line]>,
    req: &Request,
    d: Option<f64>,
    t: Option<f64>,
) -> f64 {
    let y = req.y_axis().q;
    p.reppers
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let amount = match (t, lines) {
                (Some(t), Some(lines)) => match y {
                    Quantity::Rps => lines[i].rps_at(t),
                    _ => lines[i].total_at(t),
                },
                _ => match y {
                    Quantity::Rps => r.declared(),
                    _ => 0.0,
                },
            };
            amount * application(r, &p.src, d, req)
        })
        .sum()
}

pub fn series(p: &Prepared, req: &Request) -> Option<(Vec<(f64, f64)>, bool)> {
    let x = req.x_axis();
    let (lo, hi) = (req.range.0.min(req.range.1), req.range.0.max(req.range.1));
    let misc_time = req.input(InputKey::Time).map(|t| t.clamp(0.0, TIME_LIMIT));
    match x.q {
        Quantity::Distance => {
            if req.y_axis().q == Quantity::TotalRep && misc_time.is_none() {
                return None;
            }
            let lines = misc_time.map(|t| timeline(&p.reppers, t, req.options.ancillary_reload));
            let pts = sample::sample(lo * 1000.0, hi * 1000.0, SAMPLES, DEPTH, |d| {
                value(p, lines.as_deref(), req, Some(d), misc_time)
            });
            Some((
                pts.into_iter().map(|(d, y)| (d / 1000.0, y)).collect(),
                false,
            ))
        }
        Quantity::Time => {
            let lo = lo.clamp(0.0, TIME_LIMIT);
            let hi = hi.clamp(0.0, TIME_LIMIT);
            let d = req.input(InputKey::Distance).map(|km| km * 1000.0);
            let lines = timeline(&p.reppers, hi, req.options.ancillary_reload);
            let mut times: Vec<f64> = lines
                .iter()
                .flat_map(|l| {
                    if req.y_axis().q == Quantity::Rps {
                        l.rps.iter().map(|(t, _)| *t).collect::<Vec<f64>>()
                    } else {
                        l.events.iter().map(|(t, _)| *t).collect()
                    }
                })
                .filter(|t| *t >= lo && *t <= hi)
                .collect();
            times.push(lo);
            times.push(hi);
            times.sort_by(|a, b| a.total_cmp(b));
            times.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            let points: Vec<(f64, f64)> = times
                .iter()
                .map(|t| (*t, value(p, Some(&lines), req, d, Some(*t))))
                .collect();
            Some((sample::steps(&points, lo, hi), true))
        }
        _ => None,
    }
}

pub fn point(p: &Prepared, req: &Request, x_value: f64) -> Option<f64> {
    let x = req.x_axis();
    let misc_time = req.input(InputKey::Time).map(|t| t.clamp(0.0, TIME_LIMIT));
    match x.q {
        Quantity::Distance => {
            if req.y_axis().q == Quantity::TotalRep && misc_time.is_none() {
                return None;
            }
            let lines = misc_time.map(|t| timeline(&p.reppers, t, req.options.ancillary_reload));
            Some(value(
                p,
                lines.as_deref(),
                req,
                Some(x_value * 1000.0),
                misc_time,
            ))
        }
        Quantity::Time => {
            let t = x_value.clamp(0.0, TIME_LIMIT);
            let d = req.input(InputKey::Distance).map(|km| km * 1000.0);
            let lines = timeline(&p.reppers, t, req.options.ancillary_reload);
            Some(value(p, Some(&lines), req, d, Some(t)))
        }
        _ => None,
    }
}
