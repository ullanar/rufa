//! Everything on a fit that deals damage, with what each needs to land it.

use rufa_data::Info;

use super::apply::{self, Flight};
use super::cycles::{self, Cycles, Spool};
use super::dmg::Dmg;
use super::model::{DroneMode, Options};
use super::target::TargetData;
use super::{Ctx, radius_of};
use crate::place::Rack;
use crate::{Calculation, Fit, FitItem, ItemResult, Slot, State};

const CAPITAL_SHIPS_SKILL: i32 = 20_533;
const GROUP_SMARTBOMB: i32 = 72;
const GROUP_BOMB_LAUNCHER: i32 = 862;
const GROUP_SUPER_WEAPON: i32 = 588;
const GROUP_STRUCTURE_GUIDED_BOMB: i32 = 1328;
const GROUP_STRUCTURE_AREA_DENIAL: i32 = 1330;
const GROUP_STRUCTURE_DOOMSDAY: i32 = 1333;

/// Where the attacker stands and flies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceData {
    pub radius: f64,
    pub max_velocity: f64,
    pub max_target_range: f64,
    pub drone_control_range: f64,
}

/// The geometry of one shot: speeds along the line, the range, the target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    pub atk_speed: f64,
    pub atk_angle: f64,
    pub distance: Option<f64>,
    pub tgt_speed: f64,
    pub tgt_angle: f64,
    pub tgt_sig: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Weapon {
    Turret {
        optimal: f64,
        falloff: f64,
        tracking: f64,
        optimal_sig: f64,
    },
    Launcher {
        flight: Option<Flight>,
        radius: f64,
        velocity: f64,
        drf: f64,
        fof: bool,
    },
    Vorton {
        range: f64,
        radius: f64,
        velocity: f64,
        drf: f64,
    },
    Smartbomb {
        range: f64,
    },
    Bomb {
        range: Option<f64>,
        blast: f64,
        radius: f64,
    },
    GuidedBomb {
        range: f64,
        radius: f64,
    },
    Doomsday {
        range: f64,
        signature: f64,
        single: bool,
    },
    Breacher {
        flight: Option<Flight>,
        ticks: u32,
        absolute: f64,
        relative: f64,
    },
    Drone {
        speed: f64,
        radius: f64,
        optimal: f64,
        falloff: f64,
        tracking: f64,
        optimal_sig: f64,
    },
    Fighter {
        speed: f64,
        radius: f64,
        optimal: f64,
        falloff: f64,
        explosion_radius: f64,
        explosion_velocity: f64,
        drf: f64,
        resist_attr: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dealer {
    pub index: usize,
    pub weapon: Weapon,
    /// Damage of one activation at zero spool.
    pub volley: Dmg,
    /// Steady damage per second at full spool, as the stats panel shows.
    pub dps: Dmg,
    pub cycles: Option<Cycles>,
    /// Spool cap and step per cycle.
    pub spool: (f64, f64),
    /// Seconds from activation to each volley.
    pub delays: Vec<f64>,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn has(item: &ItemResult, id: i32) -> bool {
    item.attributes.contains_key(&id)
}

pub fn source_data<I: Info>(ctx: &Ctx<'_, I>, fit: &Fit, calc: &Calculation) -> SourceData {
    let ids = ctx.ids;
    SourceData {
        radius: radius_of(ctx.info, fit.ship.type_id, &calc.ship),
        max_velocity: attr(&calc.ship, ids.max_velocity),
        max_target_range: attr(&calc.ship, ids.max_target_range),
        drone_control_range: calc
            .character
            .attributes
            .get(&ids.drone_control_distance)
            .map(|a| a.value)
            .unwrap_or(20_000.0),
    }
}

/// The optimal range a module fires or reaches to, whichever attribute it has.
pub fn module_optimal<I: Info>(ctx: &Ctx<'_, I>, result: &ItemResult) -> f64 {
    let ids = ctx.ids;
    [
        ids.max_range,
        ids.shield_transfer_range,
        ids.power_transfer_range,
        ids.emp_field_range,
        ids.ecm_burst_range,
        ids.warp_scramble_range,
    ]
    .into_iter()
    .map(|id| attr(result, id))
    .find(|v| *v > 0.0)
    .unwrap_or(0.0)
}

pub fn module_falloff<I: Info>(ctx: &Ctx<'_, I>, result: &ItemResult) -> f64 {
    let ids = ctx.ids;
    [ids.falloff_effectiveness, ids.falloff]
        .into_iter()
        .map(|id| attr(result, id))
        .find(|v| *v > 0.0)
        .unwrap_or(0.0)
}

fn damage_of(ids: &crate::attrs::AttrIds, item: &ItemResult) -> [f64; 4] {
    [
        attr(item, ids.em_damage),
        attr(item, ids.thermal_damage),
        attr(item, ids.kinetic_damage),
        attr(item, ids.explosive_damage),
    ]
}

fn group_of(info: &impl Info, type_id: i32) -> i32 {
    info.get_type(type_id).map(|t| t.group_id()).unwrap_or(0)
}

fn flight<I: Info>(
    ctx: &Ctx<'_, I>,
    result: &ItemResult,
    ship_radius: f64,
    fof: bool,
) -> Option<Flight> {
    let ids = ctx.ids;
    let charge = result.charge.as_deref()?;
    let velocity = attr(charge, ids.max_velocity);
    let cap = if fof && has(charge, ids.max_fof_target_range) {
        Some(attr(charge, ids.max_fof_target_range))
    } else {
        None
    };
    Flight::new(
        velocity,
        attr(charge, ids.explosion_delay) / 1000.0,
        attr(charge, ids.mass),
        attr(charge, ids.agility),
        ship_radius,
        cap,
    )
}

/// Classifies one racked module. None when it deals no damage.
fn module_weapon<I: Info>(
    ctx: &Ctx<'_, I>,
    item: &FitItem,
    result: &ItemResult,
    src: &SourceData,
) -> Option<(Weapon, Dmg, Vec<f64>)> {
    let ids = ctx.ids;
    let fx = ctx.fx;
    let info = ctx.info;
    let type_id = item.type_id;
    let charge_type = item.charge.as_ref().map(|c| c.type_id).unwrap_or(0);
    let group = group_of(info, type_id);
    let charge = result.charge.as_deref();
    let source = charge.unwrap_or(result);
    let multiplier = if has(result, ids.damage_multiplier) {
        attr(result, ids.damage_multiplier)
    } else {
        1.0
    };
    let volley = Dmg::types(damage_of(ids, source)).scale(multiplier);

    if let Some(charge) = charge
        && fx.has(info, charge_type, fx.dot_missile)
    {
        let ticks = attr(charge, ids.dot_duration).round().max(0.0) as u32;
        return Some((
            Weapon::Breacher {
                flight: flight(ctx, result, src.radius, false),
                ticks,
                absolute: attr(charge, ids.dot_max_damage_per_tick),
                relative: attr(charge, ids.dot_max_hp_percentage_per_tick) / 100.0,
            },
            Dmg {
                pure: attr(charge, ids.dot_max_damage_per_tick),
                ..Dmg::default()
            },
            vec![0.0],
        ));
    }
    if volley.total() <= 0.0 {
        return None;
    }
    let weapon = if fx.has(info, type_id, fx.chain_lightning) {
        Weapon::Vorton {
            range: attr(result, ids.max_range),
            radius: attr(result, ids.aoe_cloud_size),
            velocity: attr(result, ids.aoe_velocity),
            drf: attr(result, ids.aoe_damage_reduction_factor),
        }
    } else if fx.has(info, type_id, fx.turret_fitted) {
        Weapon::Turret {
            optimal: module_optimal(ctx, result),
            falloff: module_falloff(ctx, result),
            tracking: attr(result, ids.tracking_speed),
            optimal_sig: attr(result, ids.optimal_sig_radius),
        }
    } else if fx.has(info, type_id, fx.launcher_fitted) || type_id == 32_461 {
        let charge = charge?;
        let fof = fx.has(info, charge_type, fx.fof_missile);
        Weapon::Launcher {
            flight: flight(ctx, result, src.radius, fof),
            radius: attr(charge, ids.aoe_cloud_size),
            velocity: attr(charge, ids.aoe_velocity),
            drf: attr(charge, ids.aoe_damage_reduction_factor),
            fof,
        }
    } else if group == GROUP_SMARTBOMB || group == GROUP_STRUCTURE_AREA_DENIAL {
        Weapon::Smartbomb {
            range: module_optimal(ctx, result),
        }
    } else if group == GROUP_BOMB_LAUNCHER {
        let charge = charge?;
        Weapon::Bomb {
            range: flight(ctx, result, src.radius, false).map(|f| f.expected()),
            blast: attr(charge, ids.explosion_range),
            radius: attr(charge, ids.aoe_cloud_size),
        }
    } else if group == GROUP_STRUCTURE_GUIDED_BOMB {
        let charge = charge?;
        Weapon::GuidedBomb {
            range: module_optimal(ctx, result),
            radius: attr(charge, ids.aoe_cloud_size),
        }
    } else if group == GROUP_SUPER_WEAPON || group == GROUP_STRUCTURE_DOOMSDAY {
        let single = fx.has_any(
            info,
            type_id,
            &[
                fx.super_weapon_amarr,
                fx.super_weapon_caldari,
                fx.super_weapon_gallente,
                fx.super_weapon_minmatar,
                fx.lightning_weapon,
            ],
        );
        Weapon::Doomsday {
            range: module_optimal(ctx, result),
            signature: attr(result, ids.signature_radius),
            single,
        }
    } else {
        return None;
    };
    let delays = volley_delays(ctx, type_id, result);
    Some((weapon, volley, delays))
}

/// Seconds after activation at which each volley of a module lands.
fn volley_delays<I: Info>(ctx: &Ctx<'_, I>, type_id: i32, result: &ItemResult) -> Vec<f64> {
    let ids = ctx.ids;
    let fx = ctx.fx;
    let info = ctx.info;
    let delay = if fx.has_any(
        info,
        type_id,
        &[
            fx.super_weapon_amarr,
            fx.super_weapon_caldari,
            fx.super_weapon_gallente,
            fx.super_weapon_minmatar,
            fx.lightning_weapon,
        ],
    ) {
        attr(result, ids.damage_delay_duration)
    } else if fx.has_any(
        info,
        type_id,
        &[
            fx.doomsday_beam_dot,
            fx.doomsday_slash,
            fx.doomsday_cone_dot,
            fx.debuff_lance,
        ],
    ) {
        attr(result, ids.doomsday_warning_duration)
    } else {
        0.0
    } / 1000.0;
    let duration = attr(result, ids.doomsday_damage_duration);
    let sub = attr(result, ids.doomsday_damage_cycle_time);
    let subcycles = if duration > 0.0 && sub > 0.0 && !fx.has(info, type_id, fx.doomsday_slash) {
        (duration / sub).floor().max(1.0) as usize
    } else {
        1
    };
    (0..subcycles)
        .map(|i| delay + sub / 1000.0 * i as f64)
        .collect()
}

/// Fighter ability attributes by prefix, looked up by name once.
struct Ability {
    effect: Option<i32>,
    damage: [Option<i32>; 4],
    multiplier: Option<i32>,
    duration: Option<i32>,
    optimal: Option<i32>,
    range: Option<i32>,
    falloff: Option<i32>,
    explosion_radius: Option<i32>,
    explosion_velocity: Option<i32>,
    reduction_factor: Option<i32>,
    reduction_sensitivity: Option<i32>,
    resist_id: Option<i32>,
}

fn ability(info: &impl Info, effect: Option<i32>, prefix: &str) -> Ability {
    let id = |suffix: &str| info.attribute_name_to_id(&format!("{prefix}{suffix}"));
    Ability {
        effect,
        damage: [
            id("DamageEM"),
            id("DamageTherm"),
            id("DamageKin"),
            id("DamageExp"),
        ],
        multiplier: id("DamageMultiplier"),
        duration: id("Duration"),
        optimal: id("RangeOptimal"),
        range: id("Range"),
        falloff: id("RangeFalloff"),
        explosion_radius: id("ExplosionRadius"),
        explosion_velocity: id("ExplosionVelocity"),
        reduction_factor: id("ReductionFactor"),
        reduction_sensitivity: id("ReductionSensitivity"),
        resist_id: id("ResistanceID"),
    }
}

fn opt_attr(item: &ItemResult, id: Option<i32>) -> f64 {
    id.map(|id| attr(item, id)).unwrap_or(0.0)
}

fn fighter_dealers<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    index: usize,
    result: &ItemResult,
    dps_shares: Dmg,
    out: &mut Vec<Dealer>,
) {
    let ids = ctx.ids;
    let info = ctx.info;
    let item = &fit.items[index];
    let abilities = [
        ability(
            info,
            ctx.fx.fighter_attack_missile,
            "fighterAbilityAttackMissile",
        ),
        ability(info, ctx.fx.fighter_missiles, "fighterAbilityMissiles"),
    ];
    let count = f64::from(item.quantity.max(1));
    let speed = attr(result, ids.max_velocity);
    let radius = radius_of(info, item.type_id, result);
    let mut split = Vec::new();
    for ab in &abilities {
        let Some(effect) = ab.effect else {
            continue;
        };
        if !crate::edit::ability_on(info, fit, index, effect) {
            continue;
        }
        let damage = [
            opt_attr(result, ab.damage[0]),
            opt_attr(result, ab.damage[1]),
            opt_attr(result, ab.damage[2]),
            opt_attr(result, ab.damage[3]),
        ];
        let multiplier = ab
            .multiplier
            .filter(|id| has(result, *id))
            .map(|id| attr(result, id))
            .unwrap_or(1.0);
        let volley = Dmg::types(damage).scale(multiplier * count);
        let duration = opt_attr(result, ab.duration) / 1000.0;
        if volley.total() <= 0.0 || duration <= 0.0 {
            continue;
        }
        let drf = {
            let f = opt_attr(result, ab.reduction_factor);
            let s = opt_attr(result, ab.reduction_sensitivity);
            if f > 0.0 && s > 0.0 && s != 1.0 {
                f.ln() / s.ln()
            } else {
                1.0
            }
        };
        let optimal = match ab.optimal.filter(|id| has(result, *id)) {
            Some(id) => attr(result, id),
            None => opt_attr(result, ab.range),
        };
        let resist_attr = ab
            .resist_id
            .map(|id| attr(result, id))
            .filter(|v| *v > 0.0)
            .map(|v| v as i32);
        split.push((
            Weapon::Fighter {
                speed,
                radius,
                optimal,
                falloff: opt_attr(result, ab.falloff),
                explosion_radius: opt_attr(result, ab.explosion_radius),
                explosion_velocity: opt_attr(result, ab.explosion_velocity),
                drf,
                resist_attr,
            },
            volley,
            duration,
        ));
    }
    let total_dps: f64 = split.iter().map(|(_, v, d)| v.total() / d).sum();
    for (weapon, volley, duration) in split {
        let share = if total_dps > 0.0 {
            volley.total() / duration / total_dps
        } else {
            0.0
        };
        out.push(Dealer {
            index,
            weapon,
            volley,
            dps: dps_shares.scale(share),
            cycles: Some(Cycles::forever(duration, 0.0)),
            spool: (0.0, 0.0),
            delays: vec![0.0],
        });
    }
}

/// Every active damage dealer of the fit. `reload` folds reload into the
/// steady figures and the cycle plan.
pub fn dealers<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    calc: &Calculation,
    reload: bool,
) -> Vec<Dealer> {
    let ids = ctx.ids;
    let src = source_data(ctx, fit, calc);
    let mut out = Vec::new();
    for (index, (item, result)) in fit.items.iter().zip(&calc.items).enumerate() {
        if result.state < State::Active {
            continue;
        }
        let steady = if reload {
            attr(result, ids.dps_with_reload)
        } else {
            attr(result, ids.dps)
        };
        let source = result.charge.as_deref().unwrap_or(result);
        let shares = damage_of(ids, source);
        match item.slot {
            Slot::DroneBay => {
                if steady <= 0.0 || item.quantity == 0 {
                    continue;
                }
                let count = f64::from(item.quantity);
                let multiplier = if has(result, ids.damage_multiplier) {
                    attr(result, ids.damage_multiplier)
                } else {
                    1.0
                };
                let Some(cycle) = cycles::drone_cycle(ids, result) else {
                    continue;
                };
                let volley = Dmg::types(shares).scale(multiplier * count);
                let optimal = [
                    ids.shield_transfer_range,
                    ids.power_transfer_range,
                    ids.emp_field_range,
                    ids.ecm_burst_range,
                    ids.max_range,
                    ids.ecm_range_optimal,
                ]
                .into_iter()
                .map(|id| attr(result, id))
                .find(|v| *v > 0.0)
                .unwrap_or(0.0);
                let falloff = [ids.falloff, ids.falloff_effectiveness]
                    .into_iter()
                    .map(|id| attr(result, id))
                    .find(|v| *v > 0.0)
                    .unwrap_or(0.0);
                out.push(Dealer {
                    index,
                    weapon: Weapon::Drone {
                        speed: attr(result, ids.max_velocity),
                        radius: radius_of(ctx.info, item.type_id, result),
                        optimal,
                        falloff,
                        tracking: attr(result, ids.tracking_speed),
                        optimal_sig: attr(result, ids.optimal_sig_radius),
                    },
                    volley,
                    dps: Dmg::split(steady * count, shares),
                    cycles: Some(Cycles::forever(cycle, 0.0)),
                    spool: (0.0, 0.0),
                    delays: vec![0.0],
                });
            }
            Slot::FighterTube(_) => {
                if steady <= 0.0 {
                    continue;
                }
                fighter_dealers(
                    ctx,
                    fit,
                    index,
                    result,
                    Dmg::split(steady, shares),
                    &mut out,
                );
            }
            slot if Rack::of_slot(slot).is_some() => {
                let Some((weapon, volley, delays)) = module_weapon(ctx, item, result, &src) else {
                    continue;
                };
                let spool_max = attr(result, ids.damage_multiplier_bonus_max);
                let spool_step = attr(result, ids.damage_multiplier_bonus_per_cycle);
                let cycles = if matches!(weapon, Weapon::Breacher { .. }) {
                    Some(Cycles::forever(1.0, 0.0))
                } else {
                    cycles::module_cycles(ids, item, result, reload)
                };
                let dps = if matches!(weapon, Weapon::Breacher { .. }) {
                    volley
                } else {
                    let full = 1.0 + spool_max;
                    Dmg::split(steady * full, shares)
                };
                out.push(Dealer {
                    index,
                    weapon,
                    volley,
                    dps,
                    cycles,
                    spool: (spool_max, spool_step),
                    delays,
                });
            }
            _ => {}
        }
    }
    out
}

impl Dealer {
    /// Damage one activation deals after `nonstop` cycles of spool.
    pub fn volley_at(&self, nonstop: u32) -> Dmg {
        let cycle = self
            .cycles
            .as_ref()
            .map(|c| c.runs[0].active)
            .unwrap_or(0.0);
        let bonus = cycles::spool_bonus(self.spool.0, self.spool.1, cycle, Spool::Cycles(nonstop));
        self.volley.scale(1.0 + bonus)
    }

    pub fn needs_lock(&self) -> bool {
        match &self.weapon {
            Weapon::Launcher { fof, .. } => !fof,
            Weapon::Smartbomb { .. } | Weapon::Bomb { .. } => false,
            Weapon::Doomsday { single, .. } => *single,
            _ => true,
        }
    }

    pub fn is_drone(&self) -> bool {
        matches!(self.weapon, Weapon::Drone { .. })
    }
}

fn in_lock_range(src: &SourceData, d: Option<f64>, options: &Options) -> bool {
    match d {
        None => true,
        Some(d) => !options.lock_range || d <= src.max_target_range,
    }
}

fn in_drone_range(src: &SourceData, d: Option<f64>, options: &Options) -> bool {
    match d {
        None => true,
        Some(d) => !options.drone_range || d <= src.drone_control_range,
    }
}

fn turret_cth(
    shot: &Shot,
    atk_radius: f64,
    optimal: f64,
    falloff: f64,
    tracking: f64,
    optimal_sig: f64,
    tgt: &TargetData,
) -> f64 {
    let angular = apply::angular_speed(
        shot.atk_speed,
        shot.atk_angle,
        atk_radius,
        shot.distance,
        shot.tgt_speed,
        shot.tgt_angle,
        tgt.radius,
    );
    apply::range_factor(optimal, falloff, shot.distance, false)
        * apply::tracking_factor(angular, optimal_sig, tracking, shot.tgt_sig)
}

/// Whether a target hull needs Capital Ships, which a titan's beam asks for.
fn is_capital(info: &impl Info, calc: &Calculation, type_id: i32) -> bool {
    let _ = calc;
    let Some(attrs) = info.get_dogma_attributes(type_id) else {
        return false;
    };
    attrs.iter().any(|a| {
        [182, 183, 184].contains(&a.attribute_id()) && a.value() as i32 == CAPITAL_SHIPS_SKILL
    })
}

/// The share of a dealer's damage that lands, 0 to 1.
pub fn application<I: Info>(
    ctx: &Ctx<'_, I>,
    dealer: &Dealer,
    src: &SourceData,
    tgt: &TargetData,
    tgt_fit: Option<(&Fit, &Calculation)>,
    shot: &Shot,
    options: &Options,
) -> f64 {
    let d = shot.distance;
    if dealer.needs_lock() && !in_lock_range(src, d, options) {
        return 0.0;
    }
    let value = match &dealer.weapon {
        Weapon::Turret {
            optimal,
            falloff,
            tracking,
            optimal_sig,
        } => apply::turret_mult(turret_cth(
            shot,
            src.radius,
            *optimal,
            *falloff,
            *tracking,
            *optimal_sig,
            tgt,
        )),
        Weapon::Launcher {
            flight,
            radius,
            velocity,
            drf,
            ..
        } => {
            let Some(flight) = flight else {
                return 0.0;
            };
            flight.factor(d)
                * apply::missile_factor(*radius, *velocity, *drf, shot.tgt_speed, shot.tgt_sig)
        }
        Weapon::Vorton {
            range,
            radius,
            velocity,
            drf,
        } => {
            apply::range_factor(*range, 0.0, d, true)
                * apply::missile_factor(*radius, *velocity, *drf, shot.tgt_speed, shot.tgt_sig)
        }
        Weapon::Smartbomb { range } => match d {
            Some(d) if *range <= 0.0 || d > *range => 0.0,
            _ => 1.0,
        },
        Weapon::Bomb {
            range,
            blast,
            radius,
        } => {
            let Some(range) = range else {
                return 0.0;
            };
            if let Some(d) = d {
                let near = (range - src.radius - tgt.radius - blast).max(0.0);
                let far = (range - src.radius + tgt.radius + blast).max(0.0);
                if d < near || d > far {
                    return 0.0;
                }
            }
            apply::bomb_factor(*radius, shot.tgt_sig)
        }
        Weapon::GuidedBomb { range, radius } => match d {
            Some(d) if *range <= 0.0 || d > range - src.radius => 0.0,
            _ => apply::bomb_factor(*radius, shot.tgt_sig),
        },
        Weapon::Doomsday {
            range,
            signature,
            single,
        } => {
            if let Some(d) = d
                && *range > 0.0
                && d > *range
            {
                return 0.0;
            }
            if *single
                && let Some((fit, calc)) = tgt_fit
                && !is_capital(ctx.info, calc, fit.ship.type_id)
            {
                return 0.0;
            }
            if *signature <= 0.0 || !shot.tgt_sig.is_finite() {
                1.0
            } else {
                (shot.tgt_sig / signature).min(1.0)
            }
        }
        Weapon::Breacher { flight, .. } => {
            let Some(flight) = flight else {
                return 0.0;
            };
            let resist = tgt_fit
                .map(|(_, calc)| attr(&calc.ship, ctx.ids.breacher_pod_damage_resistance))
                .filter(|v| *v > 0.0)
                .unwrap_or(1.0);
            flight.factor(d) * resist
        }
        Weapon::Drone {
            speed,
            radius,
            optimal,
            falloff,
            tracking,
            optimal_sig,
        } => {
            if !in_drone_range(src, d, options) {
                return 0.0;
            }
            let keeps_up = *speed > 1.0
                && ((options.drone_mode == DroneMode::Auto && *speed >= shot.tgt_speed)
                    || options.drone_mode == DroneMode::FollowTarget);
            if keeps_up {
                apply::turret_mult(1.0)
            } else {
                let placed = Shot {
                    atk_speed: shot.atk_speed.min(*speed),
                    distance: d.map(|d| d + src.radius - radius),
                    ..*shot
                };
                apply::turret_mult(turret_cth(
                    &placed,
                    *radius,
                    *optimal,
                    *falloff,
                    *tracking,
                    *optimal_sig,
                    tgt,
                ))
            }
        }
        Weapon::Fighter {
            speed,
            radius,
            optimal,
            falloff,
            explosion_radius,
            explosion_velocity,
            drf,
            resist_attr,
        } => {
            let keeps_up = (options.drone_mode == DroneMode::Auto && *speed >= shot.tgt_speed)
                || options.drone_mode == DroneMode::FollowTarget;
            let range = if keeps_up {
                1.0
            } else {
                let rd = d.map(|d| d + src.radius - radius);
                apply::range_factor(*optimal, *falloff, rd, true)
            };
            let missile = apply::missile_factor(
                *explosion_radius,
                *explosion_velocity,
                *drf,
                shot.tgt_speed,
                shot.tgt_sig,
            );
            let resist = match (resist_attr, tgt_fit) {
                (Some(id), Some((_, calc))) if calc.ship.attributes.contains_key(id) => {
                    attr(&calc.ship, *id)
                }
                _ => 1.0,
            };
            range * missile * resist
        }
    };
    apply::unerr(value)
}
