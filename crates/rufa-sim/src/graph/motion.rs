//! Mobility, warp time and lock time of a fit.

use rufa_data::Info;

use super::apply;
use super::model::{Quantity, Request, Unit};
use super::sample;
use super::{Ctx, InputKey};
use crate::{Calculation, Fit, State};

pub const AU: f64 = 149_597_870_700.0;
const SAMPLES: usize = 200;
const WARP_SAMPLES: usize = 500;

fn attr(calc: &Calculation, id: i32) -> f64 {
    calc.ship
        .attributes
        .get(&id)
        .map(|a| a.value)
        .unwrap_or(0.0)
}

/// Speed after `t` seconds from rest.
pub fn speed(vmax: f64, tau: f64, t: f64) -> f64 {
    if tau <= 0.0 {
        return if t > 0.0 { vmax } else { 0.0 };
    }
    vmax * (1.0 - (-t / tau).exp())
}

/// Metres covered in `t` seconds from rest.
pub fn travelled(vmax: f64, tau: f64, t: f64) -> f64 {
    if tau <= 0.0 {
        return vmax * t;
    }
    vmax * (t - tau * (1.0 - (-t / tau).exp()))
}

/// Seconds a warp of `d` metres takes at `warp_speed` AU/s, dropping out at
/// half the sub warp speed and at most 100 m/s.
pub fn warp_time(warp_speed: f64, sub_warp: f64, d: f64) -> f64 {
    if d <= 0.0 || warp_speed <= 0.0 {
        return 0.0;
    }
    let k_a = warp_speed;
    let k_d = (warp_speed / 3.0).min(2.0);
    let v_drop = (sub_warp / 2.0).clamp(1e-9, 100.0);
    let mut v = warp_speed * AU;
    let d_accel = AU;
    let d_decel = v / k_d;
    let t_cruise = if d_accel + d_decel > d {
        v = d * k_a * k_d / (k_a + k_d);
        0.0
    } else {
        (d - d_accel - d_decel) / v
    };
    let t_accel = (v / k_a).max(1.0).ln() / k_a;
    let t_decel = (v / v_drop).max(1.0).ln() / k_d;
    t_accel + t_cruise + t_decel
}

/// Top speed with every propulsion module offline and nothing projected.
pub fn sub_warp_speed<I: Info>(ctx: &Ctx<'_, I>, fit: &Fit, calc: &Calculation) -> f64 {
    let quiet: Vec<usize> = fit
        .items
        .iter()
        .zip(&calc.items)
        .enumerate()
        .filter(|(_, (item, result))| {
            result.state >= State::Active
                && ctx
                    .fx
                    .has_any(ctx.info, item.type_id, &[ctx.fx.mwd, ctx.fx.afterburner])
        })
        .map(|(i, _)| i)
        .collect();
    if quiet.is_empty() && fit.incoming.effects.is_empty() && fit.incoming.buffs.is_empty() {
        return attr(calc, ctx.ids.max_velocity);
    }
    let mut still = fit.clone();
    for i in quiet {
        still.items[i].state = State::Online;
    }
    still.incoming = Default::default();
    let calc = crate::calculate(ctx.info, &still);
    attr(&calc, ctx.ids.max_velocity)
}

struct Ship {
    vmax: f64,
    mass: f64,
    tau: f64,
    warp_speed: f64,
    max_warp: f64,
    scan_resolution: f64,
    scan_speed: f64,
}

fn ship<I: Info>(ctx: &Ctx<'_, I>, calc: &Calculation) -> Ship {
    let ids = ctx.ids;
    let mass = attr(calc, ids.mass);
    let agility = attr(calc, ids.agility);
    let base = attr(calc, ids.base_warp_speed);
    let mult = attr(calc, ids.warp_speed_multiplier);
    let need = attr(calc, ids.warp_capacitor_need) * mass;
    Ship {
        vmax: attr(calc, ids.max_velocity),
        mass,
        tau: agility * mass / 1e6,
        warp_speed: if base > 0.0 { base } else { 1.0 } * if mult > 0.0 { mult } else { 1.0 },
        max_warp: if need > 0.0 {
            attr(calc, ids.capacitor_capacity) / need
        } else {
            0.0
        },
        scan_resolution: attr(calc, ids.scan_resolution),
        scan_speed: attr(calc, ids.scan_speed) / 1000.0,
    }
}

fn mobility_value(s: &Ship, req: &Request, t: f64) -> Option<f64> {
    let y = req.y_axis().q;
    let v = speed(s.vmax, s.tau, t);
    Some(match y {
        Quantity::Speed => v,
        Quantity::Travelled => travelled(s.vmax, s.tau, t) / 1000.0,
        Quantity::Momentum => v * s.mass / 1e9,
        Quantity::BumpSpeed | Quantity::BumpDistance => {
            let m = req.input(InputKey::TgtMass)? * 1e6;
            if s.mass + m <= 0.0 {
                return None;
            }
            let u = 2.0 * v * s.mass / (s.mass + m);
            if y == Quantity::BumpSpeed {
                u
            } else {
                let inertia = req.input(InputKey::TgtInertia)?;
                u * (m / 1e6) * inertia / 1000.0
            }
        }
        _ => return None,
    })
}

pub fn series<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
) -> Option<(Vec<(f64, f64)>, bool)> {
    let s = ship(ctx, calc);
    let (lo, hi) = (req.range.0.min(req.range.1), req.range.0.max(req.range.1));
    match req.kind {
        super::Kind::Mobility => {
            let lo = lo.max(0.0);
            mobility_value(&s, req, lo)?;
            let pts = sample::sample(lo, hi.max(lo), SAMPLES, 0, |t| {
                mobility_value(&s, req, t).unwrap_or(0.0)
            });
            Some((pts, false))
        }
        super::Kind::Warp => {
            let unit = if req.x_axis().unit == Unit::Au {
                AU
            } else {
                1000.0
            };
            let cap = s.max_warp * AU;
            let lo = (lo * unit).clamp(0.0, cap);
            let hi = (hi * unit).clamp(0.0, cap);
            let sub = sub_warp_speed(ctx, fit, calc);
            let pts = sample::sample(lo, hi, WARP_SAMPLES, 0, |d| warp_time(s.warp_speed, sub, d));
            Some((pts.into_iter().map(|(d, t)| (d / unit, t)).collect(), false))
        }
        super::Kind::Lock => {
            let lo = lo.max(1.0);
            let hi = hi.max(lo);
            let pts = sample::sample(lo, hi, SAMPLES, 0, |sig| lock(&s, sig));
            Some((pts, false))
        }
        _ => None,
    }
}

fn lock(s: &Ship, signature: f64) -> f64 {
    if s.scan_resolution > 0.0 {
        apply::lock_time(s.scan_resolution, signature).unwrap_or(0.0)
    } else {
        s.scan_speed
    }
}

pub fn point<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
    x_value: f64,
) -> Option<f64> {
    let s = ship(ctx, calc);
    match req.kind {
        super::Kind::Mobility => mobility_value(&s, req, x_value.max(0.0)),
        super::Kind::Warp => {
            let unit = if req.x_axis().unit == Unit::Au {
                AU
            } else {
                1000.0
            };
            let d = x_value * unit;
            if d > s.max_warp * AU + 1e-6 || d < 0.0 {
                return None;
            }
            Some(warp_time(s.warp_speed, sub_warp_speed(ctx, fit, calc), d))
        }
        super::Kind::Lock => (x_value > 0.0).then(|| lock(&s, x_value.max(1.0))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_reaches_three_quarters_at_ln_four_tau() {
        let tau = 5.0;
        let t = 4f64.ln() * tau;
        assert!((speed(1000.0, tau, t) - 750.0).abs() < 1e-9);
        assert_eq!(speed(1000.0, tau, 0.0), 0.0);
        assert!((travelled(1000.0, tau, 100.0) - 1000.0 * (100.0 - tau)).abs() < 1e-3);
    }

    #[test]
    fn a_twenty_au_warp_at_three_au_per_second() {
        let t = warp_time(3.0, 300.0, 20.0 * AU);
        let v = 3.0 * AU;
        let k_d: f64 = 1.0;
        let expected =
            (v / 3.0).ln() / 3.0 + (20.0 * AU - AU - v / k_d) / v + (v / 100.0).ln() / k_d;
        assert!((t - expected).abs() < 1e-6, "{t} vs {expected}");
        assert!(t > 20.0 && t < 40.0, "{t}");
    }

    #[test]
    fn a_short_warp_never_reaches_cruise() {
        let short = warp_time(3.0, 300.0, 200_000.0);
        let long = warp_time(3.0, 300.0, 5.0 * AU);
        assert!(short > 0.0 && short < long, "{short} {long}");
        assert_eq!(warp_time(3.0, 300.0, 0.0), 0.0);
    }
}
