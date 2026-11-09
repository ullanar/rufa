//! Electronic warfare strength of a fit by distance.

use rufa_data::Info;

use super::apply;
use super::cycles;
use super::dealers::{SourceData, module_falloff, module_optimal, source_data};
use super::model::{Quantity, Request};
use super::sample;
use super::{Ctx, InputKey};
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, Slot, State};

const SAMPLES: usize = 50;
const DEPTH: u32 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub strength: f64,
    pub optimal: f64,
    pub falloff: f64,
    pub needs_lock: bool,
    pub needs_drone_range: bool,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn max4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    a.max(b).max(c).max(d)
}

/// Every active module, drone and fighter that applies the chosen effect,
/// with its raw strength before range and resistance.
pub fn entries<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    calc: &Calculation,
    y: Quantity,
) -> Vec<Entry> {
    let ids = ctx.ids;
    let fx = ctx.fx;
    let info = ctx.info;
    let mut out = Vec::new();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active {
            continue;
        }
        let effects = fx.of(info, item.type_id);
        let has = |e: Option<i32>| e.is_some_and(|e| effects.contains(&e));
        let cycle = cycles::module_cycles(ids, item, result, false)
            .map(|c| c.average())
            .unwrap_or(0.0);
        let per_second = |amount: f64| if cycle > 0.0 { amount / cycle } else { 0.0 };
        let ecm = max4(
            attr(result, ids.scan_gravimetric_strength_bonus),
            attr(result, ids.scan_ladar_strength_bonus),
            attr(result, ids.scan_magnetometric_strength_bonus),
            attr(result, ids.scan_radar_strength_bonus),
        );
        match item.slot {
            Slot::DroneBay => {
                let strength = match y {
                    Quantity::NeutStr if has(fx.neut_drone) => {
                        let d = attr(result, ids.energy_neutralizer_duration) / 1000.0;
                        if d > 0.0 {
                            attr(result, ids.energy_neutralizer_amount) / d
                        } else {
                            0.0
                        }
                    }
                    Quantity::WebStr if has(fx.web_drone) => attr(result, ids.speed_factor),
                    Quantity::EcmStr if has(fx.ecm_drone) => ecm,
                    Quantity::DampStr if has(fx.damp_drone) => {
                        attr(result, ids.max_target_range_bonus)
                    }
                    Quantity::TdStr if has(fx.weapon_disruptor_drone) => {
                        attr(result, ids.max_range_bonus)
                    }
                    Quantity::TpStr if has(fx.paint_drone) => {
                        attr(result, ids.signature_radius_bonus)
                    }
                    _ => continue,
                };
                for _ in 0..item.quantity {
                    out.push(Entry {
                        strength,
                        optimal: f64::INFINITY,
                        falloff: 0.0,
                        needs_lock: true,
                        needs_drone_range: true,
                    });
                }
            }
            Slot::FighterTube(_) => {
                let amount = f64::from(item.quantity.max(1));
                let strength = match y {
                    Quantity::NeutStr if has(fx.neut_fighter) => {
                        let d = attr(result, ids.fighter_neut_duration) / 1000.0;
                        if d > 0.0 {
                            attr(result, ids.fighter_neut_amount) / d * amount
                        } else {
                            0.0
                        }
                    }
                    Quantity::WebStr if has(fx.web_fighter) => {
                        attr(result, ids.fighter_web_speed_penalty) * amount
                    }
                    Quantity::EcmStr if has(fx.ecm_fighter) => {
                        max4(
                            attr(result, ids.fighter_ecm_gravimetric),
                            attr(result, ids.fighter_ecm_ladar),
                            attr(result, ids.fighter_ecm_magnetometric),
                            attr(result, ids.fighter_ecm_radar),
                        ) * amount
                    }
                    _ => continue,
                };
                out.push(Entry {
                    strength,
                    optimal: f64::INFINITY,
                    falloff: 0.0,
                    needs_lock: true,
                    needs_drone_range: false,
                });
            }
            slot if Rack::of_slot(slot).is_some() => {
                let burst = |e: Option<i32>| has(e);
                let (strength, is_burst) = match y {
                    Quantity::NeutStr => {
                        if has(fx.neut) || has(fx.structure_neut) {
                            (
                                per_second(attr(result, ids.energy_neutralizer_amount)),
                                false,
                            )
                        } else if has(fx.nos) && attr(result, ids.nos_override) > 0.0 {
                            (per_second(attr(result, ids.power_transfer_amount)), false)
                        } else if burst(fx.burst_neut) {
                            (
                                per_second(attr(result, ids.energy_neutralizer_amount)),
                                true,
                            )
                        } else {
                            continue;
                        }
                    }
                    Quantity::WebStr => {
                        if has(fx.web) || has(fx.structure_web) {
                            (attr(result, ids.speed_factor), false)
                        } else if burst(fx.burst_web) {
                            (attr(result, ids.speed_factor), true)
                        } else {
                            continue;
                        }
                    }
                    Quantity::EcmStr => {
                        if has(fx.ecm) || has(fx.structure_ecm) {
                            (ecm, false)
                        } else if burst(fx.burst_ecm) {
                            (ecm, true)
                        } else {
                            continue;
                        }
                    }
                    Quantity::DampStr => {
                        if has(fx.damp) || has(fx.structure_damp) {
                            (attr(result, ids.max_target_range_bonus), false)
                        } else if burst(fx.burst_damp) {
                            (attr(result, ids.max_target_range_bonus), true)
                        } else {
                            continue;
                        }
                    }
                    Quantity::TdStr => {
                        if has(fx.tracking_disruptor) || has(fx.structure_weapon_disruptor) {
                            (attr(result, ids.max_range_bonus), false)
                        } else if burst(fx.burst_track) {
                            (attr(result, ids.max_range_bonus), true)
                        } else {
                            continue;
                        }
                    }
                    Quantity::GdStr => {
                        if has(fx.guidance_disruptor) || has(fx.structure_weapon_disruptor) {
                            (attr(result, ids.missile_velocity_bonus), false)
                        } else if burst(fx.burst_track) {
                            (attr(result, ids.missile_velocity_bonus), true)
                        } else {
                            continue;
                        }
                    }
                    Quantity::TpStr => {
                        if has(fx.paint) || has(fx.structure_paint) {
                            (attr(result, ids.signature_radius_bonus), false)
                        } else if burst(fx.burst_paint) {
                            (attr(result, ids.signature_radius_bonus), true)
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };
                let mut optimal = module_optimal(ctx, result);
                if is_burst {
                    optimal = (optimal + attr(result, ids.doomsday_aoe_range)).max(0.0);
                }
                out.push(Entry {
                    strength,
                    optimal,
                    falloff: module_falloff(ctx, result),
                    needs_lock: !is_burst,
                    needs_drone_range: false,
                });
            }
            _ => {}
        }
    }
    out
}

/// Guidance disruption needs the flight time bonus alongside the velocity one.
pub fn gd_time_entries<I: Info>(ctx: &Ctx<'_, I>, fit: &Fit, calc: &Calculation) -> Vec<Entry> {
    let ids = ctx.ids;
    let fx = ctx.fx;
    let mut out = Vec::new();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active || Rack::of_slot(item.slot).is_none() {
            continue;
        }
        let effects = fx.of(ctx.info, item.type_id);
        let has = |e: Option<i32>| e.is_some_and(|e| effects.contains(&e));
        let burst = has(fx.burst_track);
        if !(has(fx.guidance_disruptor) || has(fx.structure_weapon_disruptor) || burst) {
            continue;
        }
        let mut optimal = module_optimal(ctx, result);
        if burst {
            optimal = (optimal + attr(result, ids.doomsday_aoe_range)).max(0.0);
        }
        out.push(Entry {
            strength: attr(result, ids.explosion_delay_bonus),
            optimal,
            falloff: module_falloff(ctx, result),
            needs_lock: !burst,
            needs_drone_range: false,
        });
    }
    out
}

fn effective(
    entries: &[Entry],
    src: &SourceData,
    d: f64,
    resonance: f64,
    req: &Request,
) -> Vec<f64> {
    entries
        .iter()
        .filter(|e| {
            (!e.needs_lock || !req.options.lock_range || d <= src.max_target_range)
                && (!e.needs_drone_range
                    || !req.options.drone_range
                    || d <= src.drone_control_range)
        })
        .map(|e| {
            let range = if e.optimal.is_infinite() {
                1.0
            } else {
                apply::range_factor(e.optimal, e.falloff, Some(d), true)
            };
            e.strength * resonance * range
        })
        .collect()
}

fn combine(y: Quantity, values: &[f64], time_values: &[f64]) -> f64 {
    match y {
        Quantity::NeutStr | Quantity::EcmStr => values.iter().sum(),
        Quantity::WebStr | Quantity::DampStr | Quantity::TdStr => {
            let mults: Vec<f64> = values.iter().map(|v| 1.0 + v / 100.0).collect();
            (1.0 - apply::penalized(&mults)) * 100.0
        }
        Quantity::GdStr => {
            let mv: Vec<f64> = values.iter().map(|v| 1.0 + v / 100.0).collect();
            let mt: Vec<f64> = time_values.iter().map(|v| 1.0 + v / 100.0).collect();
            (1.0 - apply::penalized(&mv) * apply::penalized(&mt)) * 100.0
        }
        Quantity::TpStr => {
            let mults: Vec<f64> = values.iter().map(|v| 1.0 + v / 100.0).collect();
            (apply::penalized(&mults) - 1.0) * 100.0
        }
        _ => 0.0,
    }
}

pub struct Prepared {
    src: SourceData,
    entries: Vec<Entry>,
    time_entries: Vec<Entry>,
}

impl Prepared {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn prepare<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
) -> Prepared {
    let y = req.y_axis().q;
    Prepared {
        src: source_data(ctx, fit, calc),
        entries: entries(ctx, fit, calc, y),
        time_entries: if y == Quantity::GdStr {
            gd_time_entries(ctx, fit, calc)
        } else {
            Vec::new()
        },
    }
}

fn value(p: &Prepared, req: &Request, d: f64) -> f64 {
    let y = req.y_axis().q;
    let resonance = 1.0 - req.input(InputKey::Resist).unwrap_or(0.0).clamp(0.0, 100.0) / 100.0;
    let values = effective(&p.entries, &p.src, d, resonance, req);
    let times = effective(&p.time_entries, &p.src, d, resonance, req);
    combine(y, &values, &times)
}

pub fn series(p: &Prepared, req: &Request) -> (Vec<(f64, f64)>, bool) {
    let (lo, hi) = (req.range.0.min(req.range.1), req.range.0.max(req.range.1));
    let pts = sample::sample(lo * 1000.0, hi * 1000.0, SAMPLES, DEPTH, |d| {
        value(p, req, d)
    });
    (
        pts.into_iter().map(|(d, y)| (d / 1000.0, y)).collect(),
        false,
    )
}

pub fn point(p: &Prepared, req: &Request, x_value: f64) -> f64 {
    value(p, req, x_value * 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webs_combine_with_the_stacking_penalty_and_paints_grow() {
        let two = combine(Quantity::WebStr, &[-60.0, -60.0], &[]);
        let expected = (1.0 - apply::penalized(&[0.4, 0.4])) * 100.0;
        assert!((two - expected).abs() < 1e-9);
        assert!((combine(Quantity::TpStr, &[30.0], &[]) - 30.0).abs() < 1e-9);
        assert_eq!(combine(Quantity::NeutStr, &[10.0, 5.0], &[]), 15.0);
        let gd = combine(Quantity::GdStr, &[-20.0], &[-10.0]);
        assert!((gd - (1.0 - 0.8 * 0.9) * 100.0).abs() < 1e-9);
    }
}
