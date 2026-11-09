//! Webs, paints and scrams of the attacker, applied to the target by range.

use rufa_data::Info;

use super::apply;
use super::dealers::{SourceData, module_falloff, module_optimal};
use super::model::{DroneMode, Options};
use super::target::Target;
use super::{Ctx, radius_of};
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, Slot, State};

#[derive(Debug, Clone, PartialEq)]
pub struct Effector {
    /// Percent change of the target attribute, negative for webs.
    pub boost: f64,
    pub optimal: f64,
    pub falloff: f64,
    pub resist: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mobile {
    pub effector: Effector,
    pub speed: f64,
    pub radius: f64,
    pub drone: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tackle {
    pub webs: Vec<Effector>,
    pub paints: Vec<Effector>,
    pub mobile_webs: Vec<Mobile>,
    pub mobile_paints: Vec<Mobile>,
    pub scram_range: Option<f64>,
}

/// How the target moves and shows before and after a scram.
#[derive(Debug, Clone, PartialEq)]
pub struct Motion {
    pub max_velocity: f64,
    pub signature: f64,
    /// Speed and signature with the propulsion module shut off.
    pub scrammed: Option<(f64, f64)>,
    pub immune: bool,
    pub resists: Vec<(i32, f64)>,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn resist_attr<I: Info>(ctx: &Ctx<'_, I>, effect: Option<i32>, result: &ItemResult) -> Option<i32> {
    if let Some(id) = effect
        && let Some(e) = ctx.info.get_dogma_effect(id)
        && e.resistance_attribute_id() > 0
    {
        return Some(e.resistance_attribute_id());
    }
    let remote = ctx.info.attribute_name_to_id("remoteResistanceID")?;
    let id = attr(result, remote);
    (id > 0.0).then_some(id as i32)
}

pub fn collect<I: Info>(ctx: &Ctx<'_, I>, fit: &Fit, calc: &Calculation) -> Tackle {
    let ids = ctx.ids;
    let fx = ctx.fx;
    let info = ctx.info;
    let mut out = Tackle::default();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active {
            continue;
        }
        let effects = fx.of(info, item.type_id);
        let has = |e: Option<i32>| e.is_some_and(|e| effects.contains(&e));
        match item.slot {
            Slot::DroneBay => {
                let mobile = |boost: f64, effect: Option<i32>| Mobile {
                    effector: Effector {
                        boost,
                        optimal: module_optimal(ctx, result),
                        falloff: module_falloff(ctx, result),
                        resist: resist_attr(ctx, effect, result),
                    },
                    speed: attr(result, ids.max_velocity),
                    radius: radius_of(info, item.type_id, result),
                    drone: true,
                };
                for _ in 0..item.quantity {
                    if has(fx.web_drone) {
                        out.mobile_webs
                            .push(mobile(attr(result, ids.speed_factor), fx.web_drone));
                    }
                    if has(fx.paint_drone) {
                        out.mobile_paints.push(mobile(
                            attr(result, ids.signature_radius_bonus),
                            fx.paint_drone,
                        ));
                    }
                }
            }
            Slot::FighterTube(_) if has(fx.web_fighter) => {
                out.mobile_webs.push(Mobile {
                    effector: Effector {
                        boost: attr(result, ids.fighter_web_speed_penalty)
                            * f64::from(item.quantity.max(1)),
                        optimal: attr(result, ids.fighter_web_optimal),
                        falloff: attr(result, ids.fighter_web_falloff),
                        resist: resist_attr(ctx, fx.web_fighter, result),
                    },
                    speed: attr(result, ids.max_velocity),
                    radius: radius_of(info, item.type_id, result),
                    drone: false,
                });
            }
            slot if Rack::of_slot(slot).is_some() => {
                let effector = |effect: Option<i32>, burst: bool| {
                    let mut optimal = module_optimal(ctx, result);
                    if burst {
                        optimal = (optimal + attr(result, ids.doomsday_aoe_range)).max(0.0);
                    }
                    Effector {
                        boost: 0.0,
                        optimal,
                        falloff: module_falloff(ctx, result),
                        resist: resist_attr(ctx, effect, result),
                    }
                };
                for (effect, burst) in [
                    (fx.web, false),
                    (fx.structure_web, false),
                    (fx.burst_web, true),
                ] {
                    if has(effect) {
                        let mut e = effector(effect, burst);
                        e.boost = attr(result, ids.speed_factor);
                        if e.boost != 0.0 {
                            out.webs.push(e);
                        }
                    }
                }
                for (effect, burst) in [
                    (fx.paint, false),
                    (fx.structure_paint, false),
                    (fx.burst_paint, true),
                ] {
                    if has(effect) {
                        let mut e = effector(effect, burst);
                        e.boost = attr(result, ids.signature_radius_bonus);
                        if e.boost != 0.0 {
                            out.paints.push(e);
                        }
                    }
                }
                let scram = (has(fx.scram) || has(fx.structure_scram))
                    && attr(result, ids.activation_blocked_strength) > 0.0;
                let focused = has(fx.bubble)
                    && result
                        .charge
                        .as_deref()
                        .is_some_and(|c| attr(c, ids.activation_blocked_strength) > 0.0);
                if scram || focused {
                    let range = module_optimal(ctx, result);
                    out.scram_range = Some(out.scram_range.map_or(range, |r: f64| r.max(range)));
                }
            }
            _ => {}
        }
    }
    out
}

impl Tackle {
    pub fn any(&self) -> bool {
        !self.webs.is_empty()
            || !self.paints.is_empty()
            || !self.mobile_webs.is_empty()
            || !self.mobile_paints.is_empty()
    }
}

/// The target's speed and signature, with and without its propulsion module.
pub fn motion<I: Info>(ctx: &Ctx<'_, I>, target: &Target<'_>) -> Motion {
    let ids = ctx.ids;
    match target {
        Target::Profile(p) => Motion {
            max_velocity: p.velocity.max(0.0),
            signature: p.signature.filter(|s| *s > 0.0).unwrap_or(f64::INFINITY),
            scrammed: None,
            immune: false,
            resists: Vec::new(),
        },
        Target::Fit { fit, calc, .. } => {
            let ship = &calc.ship;
            let max_velocity = attr(ship, ids.max_velocity);
            let signature = attr(ship, ids.signature_radius);
            let immune = attr(ship, ids.disallow_offensive_modifiers) > 0.0;
            let scrammable: Vec<usize> = fit
                .items
                .iter()
                .zip(&calc.items)
                .enumerate()
                .filter(|(_, (item, result))| {
                    result.state >= State::Active
                        && ctx.fx.has_any(
                            ctx.info,
                            item.type_id,
                            &[ctx.fx.mwd, ctx.fx.micro_jump, ctx.fx.micro_jump_portal],
                        )
                })
                .map(|(i, _)| i)
                .collect();
            let scrammed = if scrammable.is_empty() {
                None
            } else {
                let mut quiet = (*fit).clone();
                for i in scrammable {
                    quiet.items[i].state = State::Online;
                }
                let calc = crate::calculate(ctx.info, &quiet);
                Some((
                    attr(&calc.ship, ids.max_velocity),
                    attr(&calc.ship, ids.signature_radius),
                ))
            };
            Motion {
                max_velocity,
                signature,
                scrammed,
                immune,
                resists: ship
                    .attributes
                    .iter()
                    .map(|(id, a)| (*id, a.value))
                    .collect(),
            }
        }
    }
}

impl Motion {
    fn resist(&self, id: Option<i32>) -> f64 {
        id.and_then(|id| self.resists.iter().find(|(k, _)| *k == id))
            .map(|(_, v)| *v)
            .unwrap_or(1.0)
    }

    fn multiplier(&self, boost: f64, resist: Option<i32>) -> f64 {
        1.0 + boost / 100.0 * self.resist(resist)
    }
}

struct Gate<'a> {
    src: &'a SourceData,
    options: &'a Options,
    d: Option<f64>,
}

impl Gate<'_> {
    fn lock(&self) -> bool {
        match self.d {
            None => true,
            Some(d) => !self.options.lock_range || d <= self.src.max_target_range,
        }
    }

    fn drone(&self) -> bool {
        match self.d {
            None => true,
            Some(d) => !self.options.drone_range || d <= self.src.drone_control_range,
        }
    }
}

impl Tackle {
    fn scrammed(&self, gate: &Gate<'_>, motion: &Motion) -> bool {
        motion.scrammed.is_some()
            && gate.lock()
            && self
                .scram_range
                .is_some_and(|r| gate.d.is_none_or(|d| d <= r))
    }

    /// Multipliers of the mobile effectors, fastest batch first.
    fn mobile_mults(
        &self,
        list: &[Mobile],
        gate: &Gate<'_>,
        src: &SourceData,
        tgt_speed: &mut dyn FnMut(&[f64]) -> f64,
        motion: &Motion,
        options: &Options,
    ) -> Vec<f64> {
        let mut mults: Vec<f64> = Vec::new();
        let mut pending: Vec<&Mobile> = list
            .iter()
            .filter(|m| gate.lock() && (!m.drone || gate.drone()))
            .collect();
        pending.retain(|m| {
            let long_enough = gate
                .d
                .is_none_or(|d| d <= m.effector.optimal - src.radius + m.radius);
            if long_enough {
                mults.push(motion.multiplier(m.effector.boost, m.effector.resist));
            }
            !long_enough
        });
        pending.sort_by(|a, b| b.speed.total_cmp(&a.speed));
        while !pending.is_empty() {
            let speed = pending[0].speed;
            let current = tgt_speed(&mults);
            let batch: Vec<&Mobile> = pending
                .iter()
                .copied()
                .filter(|m| m.speed == speed)
                .collect();
            pending.retain(|m| m.speed != speed);
            for m in batch {
                let keeps_up = (options.drone_mode == DroneMode::Auto && m.speed >= current)
                    || options.drone_mode == DroneMode::FollowTarget;
                let boost = if keeps_up {
                    m.effector.boost
                } else {
                    let rd = gate.d.map(|d| d + src.radius - m.radius);
                    m.effector.boost
                        * apply::range_factor(m.effector.optimal, m.effector.falloff, rd, true)
                };
                mults.push(motion.multiplier(boost, m.effector.resist));
            }
        }
        mults
    }

    /// Target speed at `d` with the attacker's webs and scram on it.
    pub fn speed(
        &self,
        src: &SourceData,
        motion: &Motion,
        d: Option<f64>,
        untackled: f64,
        options: &Options,
    ) -> f64 {
        if motion.immune || motion.max_velocity <= 0.0 {
            return untackled.min(motion.max_velocity.max(0.0));
        }
        let gate = Gate { src, options, d };
        let ratio = untackled / motion.max_velocity;
        let base = if self.scrammed(&gate, motion) {
            motion.scrammed.map(|s| s.0).unwrap_or(motion.max_velocity)
        } else {
            motion.max_velocity
        };
        let mut mults: Vec<f64> = Vec::new();
        if gate.lock() {
            for web in &self.webs {
                let boost = web.boost * apply::range_factor(web.optimal, web.falloff, d, true);
                if boost != 0.0 {
                    mults.push(motion.multiplier(boost, web.resist));
                }
            }
        }
        let mut speed_with = |extra: &[f64]| {
            let mut all = mults.clone();
            all.extend_from_slice(extra);
            base * apply::penalized(&all) * ratio
        };
        let mobile = self.mobile_mults(
            &self.mobile_webs,
            &gate,
            src,
            &mut speed_with,
            motion,
            options,
        );
        apply::unerr(speed_with(&mobile))
    }

    /// Factor on the target signature from the attacker's paints and scram at `d`.
    pub fn signature(
        &self,
        src: &SourceData,
        motion: &Motion,
        d: Option<f64>,
        tgt_speed: f64,
        options: &Options,
    ) -> f64 {
        if motion.immune || !motion.signature.is_finite() || motion.signature <= 0.0 {
            return 1.0;
        }
        let gate = Gate { src, options, d };
        let base = if self.scrammed(&gate, motion) {
            motion.scrammed.map(|s| s.1).unwrap_or(motion.signature)
        } else {
            motion.signature
        };
        let mut mults: Vec<f64> = Vec::new();
        if gate.lock() {
            for paint in &self.paints {
                let boost =
                    paint.boost * apply::range_factor(paint.optimal, paint.falloff, d, true);
                if boost != 0.0 {
                    mults.push(motion.multiplier(boost, paint.resist));
                }
            }
        }
        let mut constant = |_: &[f64]| tgt_speed;
        let mobile = self.mobile_mults(
            &self.mobile_paints,
            &gate,
            src,
            &mut constant,
            motion,
            options,
        );
        mults.extend(mobile);
        apply::unerr(base * apply::penalized(&mults) / motion.signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion(speed: f64, sig: f64) -> Motion {
        Motion {
            max_velocity: speed,
            signature: sig,
            scrammed: None,
            immune: false,
            resists: Vec::new(),
        }
    }

    fn src() -> SourceData {
        SourceData {
            radius: 0.0,
            max_velocity: 1000.0,
            max_target_range: 50_000.0,
            drone_control_range: 20_000.0,
        }
    }

    fn web(boost: f64, optimal: f64, falloff: f64) -> Effector {
        Effector {
            boost,
            optimal,
            falloff,
            resist: None,
        }
    }

    #[test]
    fn two_webs_stack_with_the_penalty() {
        let t = Tackle {
            webs: vec![web(-60.0, 10_000.0, 0.0), web(-60.0, 10_000.0, 0.0)],
            ..Tackle::default()
        };
        let m = motion(1000.0, 100.0);
        let o = Options::default();
        let expected = 1000.0 * apply::penalized(&[0.4, 0.4]);
        assert!((t.speed(&src(), &m, Some(5_000.0), 1000.0, &o) - expected).abs() < 1e-6);
        assert_eq!(t.speed(&src(), &m, Some(15_000.0), 1000.0, &o), 1000.0);
        assert!((t.speed(&src(), &m, None, 500.0, &o) - expected / 2.0).abs() < 1e-6);
    }

    #[test]
    fn a_scram_drops_the_target_to_its_quiet_speed() {
        let t = Tackle {
            scram_range: Some(9_000.0),
            ..Tackle::default()
        };
        let m = Motion {
            scrammed: Some((200.0, 50.0)),
            ..motion(1000.0, 100.0)
        };
        let o = Options::default();
        assert_eq!(t.speed(&src(), &m, Some(8_000.0), 1000.0, &o), 200.0);
        assert_eq!(t.speed(&src(), &m, Some(10_000.0), 1000.0, &o), 1000.0);
        assert!((t.signature(&src(), &m, Some(8_000.0), 200.0, &o) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn a_fast_web_drone_lands_in_full_and_a_slow_one_by_range() {
        let drone = |speed: f64| Mobile {
            effector: web(-50.0, 5_000.0, 0.0),
            speed,
            radius: 0.0,
            drone: true,
        };
        let m = motion(1000.0, 100.0);
        let o = Options::default();
        let fast = Tackle {
            mobile_webs: vec![drone(2000.0)],
            ..Tackle::default()
        };
        assert!((fast.speed(&src(), &m, Some(15_000.0), 1000.0, &o) - 500.0).abs() < 1e-9);
        let slow = Tackle {
            mobile_webs: vec![drone(500.0)],
            ..Tackle::default()
        };
        assert_eq!(slow.speed(&src(), &m, Some(15_000.0), 1000.0, &o), 1000.0);
        assert!((slow.speed(&src(), &m, Some(4_000.0), 1000.0, &o) - 500.0).abs() < 1e-9);
    }
}
