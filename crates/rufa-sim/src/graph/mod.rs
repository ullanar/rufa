pub mod apply;
pub mod cycles;
pub mod damage;
pub mod dealers;
pub mod dmg;
pub mod effects;
pub mod model;
pub mod sample;
pub mod tackle;
pub mod target;
pub mod timeline;

use rufa_data::Info;

pub use effects::EffectIds;
pub use model::*;

/// What every graph calculation reads: game data, attribute and effect ids.
pub struct Ctx<'a, I: Info> {
    pub info: &'a I,
    pub ids: &'a AttrIds,
    pub fx: &'a EffectIds,
}

/// The radius of an item: the attribute when the engine has it, else the
/// type's own.
pub fn radius_of(info: &impl Info, type_id: i32, result: &ItemResult) -> f64 {
    if let Some(id) = info.attribute_name_to_id("radius")
        && let Some(a) = result.attributes.get(&id)
        && a.value > 0.0
    {
        return a.value;
    }
    info.get_type(type_id)
        .and_then(|t| t.radius())
        .map(f64::from)
        .unwrap_or(0.0)
}

use crate::attrs::AttrIds;
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, Slot, State};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub signature: f64,
    pub velocity: f64,
    pub resists: [f64; 4],
}

impl Target {
    pub const IDEAL: Target = Target {
        signature: f64::INFINITY,
        velocity: 0.0,
        resists: [0.0; 4],
    };
}

fn resist_factor(ids: &AttrIds, result: &ItemResult, target: &Target) -> f64 {
    if target.resists == [0.0; 4] {
        return 1.0;
    }
    let source = result.charge.as_deref().unwrap_or(result);
    let mix = [
        attr(source, ids.em_damage).unwrap_or(0.0),
        attr(source, ids.thermal_damage).unwrap_or(0.0),
        attr(source, ids.kinetic_damage).unwrap_or(0.0),
        attr(source, ids.explosive_damage).unwrap_or(0.0),
    ];
    let total: f64 = mix.iter().sum();
    if total <= 0.0 {
        return 1.0 - target.resists.iter().sum::<f64>() / 4.0;
    }
    mix.iter()
        .zip(&target.resists)
        .map(|(d, r)| d / total * (1.0 - r))
        .sum()
}

#[derive(Debug, Clone, Default)]
pub struct Curve {
    pub points: Vec<(f64, f64)>,
}

fn attr(item: &ItemResult, id: i32) -> Option<f64> {
    item.attributes.get(&id).map(|a| a.value)
}

fn turret_quality(chance: f64) -> f64 {
    let c = chance.clamp(0.0, 1.0);
    (c * c + c) / 2.0
}

fn turret_chance(distance: f64, optimal: f64, falloff: f64) -> f64 {
    let over = (distance - optimal).max(0.0);
    if over == 0.0 {
        return 1.0;
    }
    if falloff <= 0.0 {
        return 0.0;
    }
    0.5f64.powf((over / falloff).powi(2))
}

fn missile_application(charge: &ItemResult, ids: &AttrIds, target: &Target) -> f64 {
    let (Some(radius), Some(velocity)) = (
        attr(charge, ids.aoe_cloud_size),
        attr(charge, ids.aoe_velocity),
    ) else {
        return 1.0;
    };
    if radius <= 0.0 || !target.signature.is_finite() {
        return 1.0;
    }
    let drf = attr(charge, ids.aoe_damage_reduction_factor)
        .unwrap_or(1.0)
        .max(0.0);
    let sig_term = target.signature / radius;
    if target.velocity <= 0.0 || velocity <= 0.0 {
        return sig_term.min(1.0);
    }
    let speed_term = (sig_term * velocity / target.velocity).powf(drf);
    sig_term.min(speed_term).min(1.0)
}

pub fn item_dps_at(
    ids: &AttrIds,
    fit: &Fit,
    calc: &Calculation,
    index: usize,
    distance: f64,
    target: &Target,
) -> f64 {
    let (Some(item), Some(result)) = (fit.items.get(index), calc.items.get(index)) else {
        return 0.0;
    };
    if result.state < State::Active {
        return 0.0;
    }
    let base = attr(result, ids.dps).unwrap_or(0.0);
    if base <= 0.0 {
        return 0.0;
    }
    let base = base * resist_factor(ids, result, target);
    match item.slot {
        Slot::DroneBay => {
            let control = attr(&calc.character, ids.drone_control_distance).unwrap_or(20_000.0);
            if distance > control {
                return 0.0;
            }
            base * item.quantity as f64
        }
        Slot::FighterTube(_) => base,
        slot if Rack::of_slot(slot).is_some() => {
            if let Some(charge) = &result.charge
                && attr(charge, ids.aoe_cloud_size).is_some()
            {
                let speed = attr(charge, ids.max_velocity).unwrap_or(0.0);
                let flight = attr(charge, ids.explosion_delay).unwrap_or(0.0) / 1000.0;
                if distance > speed * flight {
                    return 0.0;
                }
                return base * missile_application(charge, ids, target);
            }
            let optimal = attr(result, ids.max_range)
                .or_else(|| attr(result, ids.emp_field_range))
                .unwrap_or(f64::INFINITY);
            let falloff = attr(result, ids.falloff).unwrap_or(0.0);
            if attr(result, ids.tracking_speed).is_some() {
                base * turret_quality(turret_chance(distance, optimal, falloff))
            } else if distance <= optimal + falloff {
                base
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

pub fn dps_at(ids: &AttrIds, fit: &Fit, calc: &Calculation, distance: f64, target: &Target) -> f64 {
    (0..fit.items.len())
        .map(|i| item_dps_at(ids, fit, calc, i, distance, target))
        .fold(0.0, |total, dps| total + dps)
}

pub fn reach(ids: &AttrIds, fit: &Fit, calc: &Calculation) -> f64 {
    let mut reach: f64 = 0.0;
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if attr(result, ids.dps).unwrap_or(0.0) <= 0.0 {
            continue;
        }
        let item_reach = match item.slot {
            Slot::DroneBay => attr(&calc.character, ids.drone_control_distance).unwrap_or(20_000.0),
            _ => match &result.charge {
                Some(charge) if attr(charge, ids.aoe_cloud_size).is_some() => {
                    attr(charge, ids.max_velocity).unwrap_or(0.0)
                        * attr(charge, ids.explosion_delay).unwrap_or(0.0)
                        / 1000.0
                }
                _ => {
                    attr(result, ids.max_range)
                        .or_else(|| attr(result, ids.emp_field_range))
                        .unwrap_or(0.0)
                        + 3.0 * attr(result, ids.falloff).unwrap_or(0.0)
                }
            },
        };
        reach = reach.max(item_reach);
    }
    reach
}

pub fn dps_curve(
    ids: &AttrIds,
    fit: &Fit,
    calc: &Calculation,
    target: &Target,
    max_distance: f64,
    samples: usize,
) -> Curve {
    let samples = samples.max(2);
    let points = (0..samples)
        .map(|i| {
            let d = max_distance * i as f64 / (samples - 1) as f64;
            (d, dps_at(ids, fit, calc, d, target))
        })
        .collect();
    Curve { points }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turret_chance_halves_at_one_falloff() {
        assert!((turret_chance(10_000.0, 5_000.0, 5_000.0) - 0.5).abs() < 1e-12);
        assert_eq!(turret_chance(4_000.0, 5_000.0, 5_000.0), 1.0);
        assert_eq!(turret_chance(6_000.0, 5_000.0, 0.0), 0.0);
    }

    #[test]
    fn turret_quality_is_one_at_full_chance() {
        assert_eq!(turret_quality(1.0), 1.0);
        assert!((turret_quality(0.5) - 0.375).abs() < 1e-12);
    }
}
