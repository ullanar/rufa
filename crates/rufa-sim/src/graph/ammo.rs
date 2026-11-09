//! The application profile: the best charge of the main weapon at every
//! distance, and the damage it lands.

use std::collections::BTreeMap;

use rufa_data::Info;
use rufa_data::catalog::category;

use super::Ctx;
use super::apply::{self, Flight};
use super::cycles;
use super::dealers::{Shot, SourceData, source_data};
use super::model::{AmmoTier, Quantity, Request, Segment};
use super::tackle::{self, Motion, Tackle};
use super::target::{Target, TargetData};
use crate::place::Rack;
use crate::{Calculation, Charge, Fit, ItemResult, State};

const META_T2: i32 = 2;
const META_FACTION: i32 = 4;
const NAVY_PREFIXES: [&str; 4] = [
    "Imperial Navy",
    "Republic Fleet",
    "Caldari Navy",
    "Federation Navy",
];
const XL_PREFIXES: [&str; 3] = ["Sansha", "Arch Angel", "Shadow"];

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn has(item: &ItemResult, id: i32) -> bool {
    item.attributes.contains_key(&id)
}

/// A type's own attribute before any modifier.
fn base(info: &impl Info, type_id: i32, id: i32) -> Option<f64> {
    info.get_dogma_attributes(type_id)
        .into_iter()
        .flatten()
        .find(|a| a.attribute_id() == id)
        .map(|a| f64::from(a.value()))
}

fn base_or(info: &impl Info, type_id: i32, id: i32, default: f64) -> f64 {
    base(info, type_id, id).unwrap_or(default)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub type_id: i32,
    pub name: String,
    /// Damage of one shot before resists, by type.
    pub damage: [f64; 4],
    pub kind: CandidateKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CandidateKind {
    Turret {
        optimal: f64,
        falloff: f64,
        tracking: f64,
    },
    Missile {
        flight: Flight,
        radius: f64,
        velocity: f64,
        drf: f64,
        priority: u8,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Weapon {
    pub count: usize,
    pub cycle: f64,
    pub optimal_sig: f64,
    pub candidates: Vec<Candidate>,
    pub max_range: f64,
    pub transitions: Vec<(f64, Option<usize>)>,
}

fn tier_ok(
    info: &impl Info,
    ids: &crate::attrs::AttrIds,
    type_id: i32,
    name: &str,
    tier: AmmoTier,
) -> bool {
    let meta = base(info, type_id, ids.meta_group_id).map(|m| m as i32);
    match tier {
        AmmoTier::All => true,
        AmmoTier::T1 => meta.is_none_or(|m| m == 1),
        AmmoTier::Navy => match meta {
            None | Some(1) | Some(META_T2) => true,
            Some(META_FACTION) => {
                let prefixes: &[&str] = if name.ends_with(" XL") {
                    &XL_PREFIXES
                } else {
                    &NAVY_PREFIXES
                };
                prefixes.iter().any(|p| name.starts_with(p))
            }
            _ => false,
        },
    }
}

/// Published charges a module type can load, in the given tier.
pub fn valid_charges<I: Info>(
    ctx: &Ctx<'_, I>,
    module_type: i32,
    tier: AmmoTier,
) -> Vec<(i32, String)> {
    let ids = ctx.ids;
    let info = ctx.info;
    let Some(module) = info.get_type(module_type) else {
        return Vec::new();
    };
    let capacity = module.capacity().map(f64::from).unwrap_or(0.0);
    let size = base(info, module_type, ids.charge_size).unwrap_or(0.0);
    let mut all: Vec<(i32, String)> = Vec::new();
    for group_id in crate::place::charge_groups(info, ids, module_type) {
        for charge in ctx.catalog.charges_in_group(group_id) {
            let Some(ty) = info.get_type(charge.id) else {
                continue;
            };
            if !ty.published() || ty.category_id() != category::CHARGE {
                continue;
            }
            let volume = ty.volume().map(f64::from).unwrap_or(0.0);
            if capacity > 0.0 && volume > capacity {
                continue;
            }
            if size > 0.0
                && (base(info, charge.id, ids.charge_size).unwrap_or(size) - size).abs() > 0.5
            {
                continue;
            }
            all.push((charge.id, charge.name.clone()));
        }
    }
    let any_meta = all
        .iter()
        .any(|(id, _)| base(info, *id, ids.meta_group_id).is_some());
    let kept: Vec<(i32, String)> = all
        .iter()
        .filter(|(id, name)| tier_ok(info, ids, *id, name, tier))
        .cloned()
        .collect();
    if kept.is_empty() && !any_meta {
        all
    } else {
        kept
    }
}

/// The loaded or first valid charge's modified attributes on a launcher,
/// from an engine run with that charge in place.
fn loaded<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    index: usize,
    calc: &Calculation,
    first: i32,
) -> Option<BTreeMap<i32, f64>> {
    let snapshot = |r: &ItemResult| r.attributes.iter().map(|(k, a)| (*k, a.value)).collect();
    if fit.items[index].charge.is_some() {
        return calc.items[index].charge.as_deref().map(snapshot);
    }
    let mut with = fit.clone();
    with.items[index].charge = Some(Charge { type_id: first });
    let calc = crate::calculate(ctx.info, &with);
    calc.items[index].charge.as_deref().map(snapshot)
}

fn damage_priority(name: &str) -> u8 {
    if name.contains("Mjolnir") {
        0
    } else if name.contains("Inferno") {
        1
    } else if name.contains("Scourge") {
        2
    } else if name.contains("Nova") {
        3
    } else {
        99
    }
}

/// Builds the weapons of the dominant hardpoint type with every candidate charge.
pub fn weapons<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    calc: &Calculation,
    req: &Request,
) -> Vec<Weapon> {
    let ids = ctx.ids;
    let fx = ctx.fx;
    let info = ctx.info;
    let src = source_data(ctx, fit, calc);
    let mut turrets: Vec<usize> = Vec::new();
    let mut launchers: Vec<usize> = Vec::new();
    for (i, (item, result)) in fit.items.iter().zip(&calc.items).enumerate() {
        if result.state < State::Active || Rack::of_slot(item.slot).is_none() {
            continue;
        }
        if fx.has(info, item.type_id, fx.turret_fitted) && !has(result, ids.mining_amount) {
            turrets.push(i);
        } else if fx.has(info, item.type_id, fx.launcher_fitted) {
            launchers.push(i);
        }
    }
    let (indexes, turret) = if turrets.len() >= launchers.len() && !turrets.is_empty() {
        (turrets, true)
    } else if !launchers.is_empty() {
        (launchers, false)
    } else {
        return Vec::new();
    };
    let mut by_type: Vec<(i32, Vec<usize>)> = Vec::new();
    for i in indexes {
        let t = fit.items[i].type_id;
        match by_type.iter_mut().find(|(k, _)| *k == t) {
            Some(e) => e.1.push(i),
            None => by_type.push((t, vec![i])),
        }
    }
    let mut out = Vec::new();
    for (type_id, members) in by_type {
        let index = members[0];
        let item = &fit.items[index];
        let result = &calc.items[index];
        let charges = valid_charges(ctx, type_id, req.options.ammo_tier);
        if charges.is_empty() {
            continue;
        }
        let cycle = cycles::module_cycles(ids, item, result, req.options.reload)
            .map(|c| c.average())
            .unwrap_or(0.0);
        if cycle <= 0.0 {
            continue;
        }
        let damage_mult = if has(result, ids.damage_multiplier) {
            attr(result, ids.damage_multiplier)
        } else {
            1.0
        };
        let mut candidates = Vec::new();
        let mut max_range: f64 = 0.0;
        if turret {
            let loaded = item.charge.as_ref().map(|c| c.type_id);
            let div = |id: i32| {
                loaded
                    .map(|c| base_or(info, c, id, 1.0))
                    .filter(|v| *v > 0.0)
                    .unwrap_or(1.0)
            };
            let optimal = attr(result, ids.max_range) / div(ids.weapon_range_multiplier);
            let falloff = attr(result, ids.falloff) / div(ids.falloff_multiplier);
            let tracking = attr(result, ids.tracking_speed) / div(ids.tracking_speed_multiplier);
            let skill_mult = match (loaded, result.charge.as_deref()) {
                (Some(c), Some(modified)) => {
                    let base_sum: f64 = [
                        ids.em_damage,
                        ids.thermal_damage,
                        ids.kinetic_damage,
                        ids.explosive_damage,
                    ]
                    .iter()
                    .map(|id| base_or(info, c, *id, 0.0))
                    .sum();
                    let mod_sum: f64 = [
                        ids.em_damage,
                        ids.thermal_damage,
                        ids.kinetic_damage,
                        ids.explosive_damage,
                    ]
                    .iter()
                    .map(|id| attr(modified, *id))
                    .sum();
                    if base_sum > 0.0 {
                        mod_sum / base_sum
                    } else {
                        1.0
                    }
                }
                _ => 1.0,
            };
            for (charge_id, name) in charges {
                let dmg = [
                    ids.em_damage,
                    ids.thermal_damage,
                    ids.kinetic_damage,
                    ids.explosive_damage,
                ]
                .map(|id| base_or(info, charge_id, id, 0.0) * skill_mult * damage_mult);
                let eff_optimal =
                    optimal * base_or(info, charge_id, ids.weapon_range_multiplier, 1.0);
                let eff_falloff = falloff * base_or(info, charge_id, ids.falloff_multiplier, 1.0);
                let eff_tracking =
                    tracking * base_or(info, charge_id, ids.tracking_speed_multiplier, 1.0);
                max_range = max_range.max(eff_optimal + eff_falloff * 3.1);
                candidates.push(Candidate {
                    type_id: charge_id,
                    name,
                    damage: dmg,
                    kind: CandidateKind::Turret {
                        optimal: eff_optimal,
                        falloff: eff_falloff,
                        tracking: eff_tracking,
                    },
                });
            }
        } else {
            let Some(modified) = loaded(ctx, fit, index, calc, charges[0].0) else {
                continue;
            };
            let loaded_type = item
                .charge
                .as_ref()
                .map(|c| c.type_id)
                .unwrap_or(charges[0].0);
            let mult = |id: i32| {
                let b = base_or(info, loaded_type, id, 0.0);
                let m = modified.get(&id).copied().unwrap_or(0.0);
                if b > 0.0 {
                    m / b
                } else if m > 0.0 {
                    m
                } else {
                    1.0
                }
            };
            let mults = [
                ids.em_damage,
                ids.thermal_damage,
                ids.kinetic_damage,
                ids.explosive_damage,
            ]
            .map(mult);
            let mult_v = mult(ids.max_velocity);
            let mult_t = mult(ids.explosion_delay);
            let mult_er = mult(ids.aoe_cloud_size);
            let mult_ev = mult(ids.aoe_velocity);
            let mult_drf = mult(ids.aoe_damage_reduction_factor);
            for (charge_id, name) in charges {
                let velocity = base_or(info, charge_id, ids.max_velocity, 0.0) * mult_v;
                let delay = base_or(info, charge_id, ids.explosion_delay, 0.0) * mult_t / 1000.0;
                if velocity <= 0.0 || delay <= 0.0 {
                    continue;
                }
                let Some(flight) = Flight::new(
                    velocity,
                    delay,
                    base_or(info, charge_id, ids.mass, 1.0),
                    base_or(info, charge_id, ids.agility, 1.0),
                    src.radius,
                    None,
                ) else {
                    continue;
                };
                let dmg = [
                    ids.em_damage,
                    ids.thermal_damage,
                    ids.kinetic_damage,
                    ids.explosive_damage,
                ]
                .iter()
                .zip(mults)
                .map(|(id, m)| base_or(info, charge_id, *id, 0.0) * m * damage_mult)
                .collect::<Vec<f64>>();
                max_range = max_range.max(flight.higher);
                candidates.push(Candidate {
                    type_id: charge_id,
                    name: name.clone(),
                    damage: [dmg[0], dmg[1], dmg[2], dmg[3]],
                    kind: CandidateKind::Missile {
                        flight,
                        radius: base_or(info, charge_id, ids.aoe_cloud_size, 0.0) * mult_er,
                        velocity: base_or(info, charge_id, ids.aoe_velocity, 0.0) * mult_ev,
                        drf: base_or(info, charge_id, ids.aoe_damage_reduction_factor, 1.0)
                            * mult_drf,
                        priority: damage_priority(&name),
                    },
                });
            }
            candidates.sort_by(|a, b| {
                let ra = match &a.kind {
                    CandidateKind::Missile { flight, .. } => flight.higher,
                    _ => 0.0,
                };
                let rb = match &b.kind {
                    CandidateKind::Missile { flight, .. } => flight.higher,
                    _ => 0.0,
                };
                rb.total_cmp(&ra).then(
                    b.damage
                        .iter()
                        .sum::<f64>()
                        .total_cmp(&a.damage.iter().sum::<f64>()),
                )
            });
        }
        if candidates.is_empty() {
            continue;
        }
        out.push(Weapon {
            count: members.len(),
            cycle,
            optimal_sig: attr(result, ids.optimal_sig_radius),
            candidates,
            max_range: max_range.floor(),
            transitions: Vec::new(),
        });
    }
    out
}

/// Attacker, target and tackle state for one source against one target.
pub struct Prepared<'a> {
    pub src: SourceData,
    pub tackle: Tackle,
    pub weapons: Vec<Weapon>,
    pub target: Target<'a>,
    pub data: TargetData,
    pub motion: Motion,
}

pub fn prepare<'a, I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
    target: Target<'a>,
) -> Prepared<'a> {
    let mut p = Prepared {
        src: source_data(ctx, fit, calc),
        tackle: tackle::collect(ctx, fit, calc),
        weapons: weapons(ctx, fit, calc, req),
        data: target.data(ctx.ids),
        motion: tackle::motion(ctx, &target),
        target,
    };
    let weapons = std::mem::take(&mut p.weapons);
    p.weapons = weapons
        .into_iter()
        .map(|mut w| {
            w.transitions = transitions(&p, req, &w);
            w
        })
        .collect();
    p
}

impl Prepared<'_> {
    fn shot(&self, req: &Request, d: f64) -> Shot {
        let tgt_speed = req.vectors.target.length.clamp(0.0, 1.0) * self.data.velocity;
        let (tgt_speed, tgt_sig) = if req.options.projected {
            let s = self
                .tackle
                .speed(&self.src, &self.motion, Some(d), tgt_speed, &req.options);
            let f = self
                .tackle
                .signature(&self.src, &self.motion, Some(d), s, &req.options);
            (s, self.data.signature * f)
        } else {
            (tgt_speed, self.data.signature)
        };
        Shot {
            atk_speed: req.vectors.attacker.length.clamp(0.0, 1.0) * self.src.max_velocity,
            atk_angle: req.vectors.attacker.angle,
            distance: Some(d),
            tgt_speed,
            tgt_angle: req.vectors.target.angle,
            tgt_sig,
        }
    }

    fn resists(&self, req: &Request) -> [f64; 4] {
        if req.options.resists {
            self.data.resists
        } else {
            [0.0; 4]
        }
    }

    /// Damage one shot of a candidate lands at `d`.
    fn applied(&self, req: &Request, w: &Weapon, c: &Candidate, shot: &Shot) -> f64 {
        let resists = self.resists(req);
        let raw: f64 = c
            .damage
            .iter()
            .zip(resists)
            .map(|(dmg, r)| dmg * (1.0 - r))
            .sum();
        let d = shot.distance.unwrap_or(0.0);
        match &c.kind {
            CandidateKind::Turret {
                optimal,
                falloff,
                tracking,
            } => {
                let range = if d <= *optimal {
                    1.0
                } else {
                    apply::range_factor(*optimal, *falloff, Some(d), false)
                };
                let track = if self.data.signature <= 0.0 {
                    1.0
                } else {
                    let angular = apply::angular_speed(
                        shot.atk_speed,
                        shot.atk_angle,
                        self.src.radius,
                        Some(d),
                        shot.tgt_speed,
                        shot.tgt_angle,
                        self.data.radius,
                    );
                    if *tracking <= 0.0 || shot.tgt_sig <= 0.0 {
                        0.0
                    } else {
                        apply::tracking_factor(angular, w.optimal_sig, *tracking, shot.tgt_sig)
                    }
                };
                raw * apply::turret_mult(range * track)
            }
            CandidateKind::Missile {
                flight,
                radius,
                velocity,
                drf,
                ..
            } => {
                let rf = flight.factor(Some(d));
                if rf <= 0.0 {
                    return 0.0;
                }
                raw * rf
                    * apply::missile_factor(*radius, *velocity, *drf, shot.tgt_speed, shot.tgt_sig)
            }
        }
    }

    fn best(&self, req: &Request, w: &Weapon, d: f64) -> (Option<usize>, f64) {
        let shot = self.shot(req, d);
        let mut best: Option<usize> = None;
        let mut best_value = 0.0;
        for (i, c) in w.candidates.iter().enumerate() {
            let v = self.applied(req, w, c, &shot);
            let better = v > best_value
                || (v > 0.0
                    && v == best_value
                    && matches!(
                        (&c.kind, best.map(|b| &w.candidates[b].kind)),
                        (CandidateKind::Missile { priority: a, .. }, Some(CandidateKind::Missile { priority: b, .. })) if a < b
                    ));
            if better {
                best = Some(i);
                best_value = v;
            }
        }
        (best, best_value)
    }
}

/// Where the best charge changes along the distance, to ten metres.
fn transitions(p: &Prepared<'_>, req: &Request, w: &Weapon) -> Vec<(f64, Option<usize>)> {
    let step = (w.max_range / 300.0 / 100.0).ceil().max(1.0) * 100.0;
    let (mut current, _) = p.best(req, w, 0.0);
    let mut out = vec![(0.0, current)];
    let mut d = step;
    while d <= w.max_range {
        let (next, value) = p.best(req, w, d);
        if next != current {
            let (mut lo, mut hi) = (d - step, d);
            while hi - lo > 10.0 {
                let mid = ((lo + hi) / 2.0).floor();
                if p.best(req, w, mid).0 == current {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            out.push((hi, next));
            current = next;
        }
        if value < 0.01
            && w.candidates
                .iter()
                .any(|c| matches!(c.kind, CandidateKind::Missile { .. }))
        {
            out.push((d, None));
            break;
        }
        d += step;
    }
    out
}

fn charge_at(w: &Weapon, d: f64) -> Option<usize> {
    w.transitions
        .iter()
        .rev()
        .find(|(from, _)| *from <= d)
        .or(w.transitions.first())
        .and_then(|(_, c)| *c)
}

/// Total applied damage at `d` metres, DPS or volley by the Y axis.
pub fn value(p: &Prepared<'_>, req: &Request, d: f64) -> f64 {
    let shot = p.shot(req, d);
    let per_second = req.y_axis().q == Quantity::Dps;
    p.weapons
        .iter()
        .map(|w| {
            let Some(c) = charge_at(w, d) else {
                return 0.0;
            };
            let v = p.applied(req, w, &w.candidates[c], &shot) * w.count as f64;
            if per_second { v / w.cycle } else { v }
        })
        .sum()
}

/// The charge name at `d`, from the first weapon.
pub fn charge_name(p: &Prepared<'_>, d: f64) -> Option<String> {
    let w = p.weapons.first()?;
    charge_at(w, d).map(|c| w.candidates[c].name.clone())
}

/// Points and charge segments over the requested range in km.
pub fn series(p: &Prepared<'_>, req: &Request) -> (Vec<(f64, f64)>, Vec<Segment>) {
    let (lo, hi) = (
        req.range.0.min(req.range.1) * 1000.0,
        req.range.0.max(req.range.1) * 1000.0,
    );
    let Some(first) = p.weapons.first() else {
        return (vec![(lo / 1000.0, 0.0), (hi / 1000.0, 0.0)], Vec::new());
    };
    let step = ((hi - lo) / 100.0).max(1.0);
    let mut points = Vec::new();
    let mut segments = Vec::new();
    let mut bounds: Vec<(f64, f64, Option<usize>)> = Vec::new();
    for (i, (from, c)) in first.transitions.iter().enumerate() {
        let to = first
            .transitions
            .get(i + 1)
            .map(|t| t.0)
            .unwrap_or(f64::INFINITY);
        bounds.push((*from, to, *c));
    }
    for (from, to, c) in bounds {
        let Some(c) = c else {
            continue;
        };
        let a = from.max(lo);
        let b = to.min(hi);
        if a > b {
            continue;
        }
        segments.push(Segment {
            from: points.len(),
            name: first.candidates[c].name.clone(),
        });
        let mut d = a;
        while d < b {
            points.push((d / 1000.0, value(p, req, d)));
            d += step;
        }
        points.push((b / 1000.0, value(p, req, b)));
    }
    if points.is_empty() {
        points = super::sample::sample(lo, hi, 100, 0, |d| value(p, req, d))
            .into_iter()
            .map(|(d, y)| (d / 1000.0, y))
            .collect();
    }
    (points, segments)
}

/// Pyfa's default range: the longest weapon reach of the sources plus a
/// tenth, capped at 300 km.
pub fn default_range<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fits: &[(&Fit, &Calculation)],
) -> (f64, f64) {
    let mut max_m: f64 = 0.0;
    for (fit, calc) in fits {
        for w in weapons(ctx, fit, calc, req) {
            for c in &w.candidates {
                let reach = match &c.kind {
                    CandidateKind::Turret {
                        optimal, falloff, ..
                    } => optimal + falloff * 2.5,
                    CandidateKind::Missile { flight, .. } => flight.higher * 1.1,
                };
                max_m = max_m.max(reach);
            }
        }
    }
    if max_m <= 0.0 {
        return (0.0, 100.0);
    }
    let km = (max_m * 1.1 / 1000.0).min(300.0);
    (0.0, (km + 0.5).floor().max(1.0))
}
