//! Capacitor and shield graphs: the recharge curve and its rate.

use rufa_data::Info;

use super::model::{Quantity, Request, Unit};
use super::sample;
use super::{Ctx, InputKey};
use crate::{Calculation, Fit};

const SAMPLES: usize = 200;
pub const CAP_TIME_LIMIT: f64 = 3600.0;

/// The level after `t` seconds from `start`, for a pool of `capacity`
/// that fills in `period` seconds.
pub fn level(start: f64, capacity: f64, period: f64, t: f64) -> f64 {
    if capacity <= 0.0 || period <= 0.0 {
        return start.clamp(0.0, capacity.max(0.0));
    }
    let root = (start / capacity).clamp(0.0, 1.0).sqrt();
    let inner = 1.0 + (-5.0 * t / period).exp() * (root - 1.0);
    (inner * inner * capacity).clamp(0.0, capacity)
}

/// The recharge rate at a given level.
pub fn rate(at: f64, capacity: f64, period: f64) -> f64 {
    if capacity <= 0.0 || period <= 0.0 {
        return 0.0;
    }
    let share = (at / capacity).clamp(0.0, 1.0);
    10.0 * capacity / period * (share.sqrt() - share)
}

struct Pool {
    capacity: f64,
    period: f64,
}

fn pool<I: Info>(ctx: &Ctx<'_, I>, calc: &Calculation, shield: bool) -> Pool {
    let ids = ctx.ids;
    let attr = |id: i32| {
        calc.ship
            .attributes
            .get(&id)
            .map(|a| a.value)
            .unwrap_or(0.0)
    };
    if shield {
        Pool {
            capacity: attr(ids.shield_hp),
            period: attr(ids.shield_recharge_rate) / 1000.0,
        }
    } else {
        Pool {
            capacity: attr(ids.capacitor_capacity),
            period: attr(ids.recharge_rate) / 1000.0,
        }
    }
}

/// Divides raw shield HP by the fit's own damage pattern through the shield
/// resonances, the stats panel's EHP.
fn effectivity<I: Info>(ctx: &Ctx<'_, I>, fit: &Fit, calc: &Calculation) -> f64 {
    let ids = ctx.ids;
    let p = &fit.environment.damage_profile;
    let total = p.em + p.thermal + p.kinetic + p.explosive;
    if total <= 0.0 {
        return 1.0;
    }
    let attr = |id: i32| {
        calc.ship
            .attributes
            .get(&id)
            .map(|a| a.value)
            .unwrap_or(1.0)
    };
    let taken = p.em / total * attr(ids.shield_em)
        + p.thermal / total * attr(ids.shield_thermal)
        + p.kinetic / total * attr(ids.shield_kinetic)
        + p.explosive / total * attr(ids.shield_explosive);
    if taken > 0.0 { 1.0 / taken } else { 1.0 }
}

/// The capacitor level over time with every module cycling, from the event
/// simulation. None when nothing drains the capacitor.
fn capsim<I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &Fit,
    calc: &Calculation,
    start: f64,
    until: f64,
) -> Option<Vec<(f64, f64)>> {
    let drains = crate::cap::drains(ctx.ids, fit, calc);
    if drains.is_empty() {
        return None;
    }
    let pool = pool(ctx, calc, false);
    let result = crate::cap::simulate_from(start, pool.capacity, pool.period, 0.0, &drains, until);
    Some(result.series)
}

/// Fills the regen arcs between simulator events into a dense polyline.
fn arcs(events: &[(f64, f64)], pool: &Pool, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let step = ((hi - lo) / SAMPLES as f64).max(1e-6);
    let mut out = Vec::new();
    let mut prev: Option<(f64, f64)> = None;
    let push_arc = |from: (f64, f64), to_t: f64, out: &mut Vec<(f64, f64)>| {
        let n = ((to_t - from.0) / step).ceil().max(1.0) as usize;
        for i in 1..=n {
            let t = (from.0 + (to_t - from.0) * i as f64 / n as f64).min(to_t);
            out.push((t, level(from.1, pool.capacity, pool.period, t - from.0)));
        }
    };
    for &(t, c) in events {
        if t < lo {
            prev = Some((t, c));
            continue;
        }
        if t > hi {
            break;
        }
        if let Some(p) = prev {
            if out.is_empty() {
                out.push((lo, level(p.1, pool.capacity, pool.period, lo - p.0)));
            }
            if t > p.0 {
                push_arc(
                    (p.0.max(lo), out.last().map(|l| l.1).unwrap_or(p.1)),
                    t,
                    &mut out,
                );
            }
        }
        out.push((t, c));
        prev = Some((t, c));
    }
    if let Some(p) = prev {
        if out.is_empty() {
            out.push((lo, level(p.1, pool.capacity, pool.period, lo - p.0)));
        }
        if hi > p.0 {
            let from = (p.0.max(lo), out.last().map(|l| l.1).unwrap_or(p.1));
            push_arc(from, hi, &mut out);
        }
    }
    out
}

pub fn series<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
) -> Option<(Vec<(f64, f64)>, bool)> {
    let shield = req.kind == super::Kind::Shield;
    let pool = pool(ctx, calc, shield);
    let x = req.x_axis();
    let y = req.y_axis();
    let (lo, hi) = (req.range.0.min(req.range.1), req.range.0.max(req.range.1));
    let eff = if shield && req.options.ehp {
        effectivity(ctx, fit, calc)
    } else {
        1.0
    };
    let y_scale = |v: f64| v * eff;
    match x.q {
        Quantity::Time => {
            let (lo, hi) = if shield {
                (lo.max(0.0), hi.max(0.0))
            } else {
                (lo.clamp(0.0, CAP_TIME_LIMIT), hi.clamp(0.0, CAP_TIME_LIMIT))
            };
            let start_key = if shield {
                InputKey::ShieldAmountT0
            } else {
                InputKey::CapAmountT0
            };
            let start = req.input(start_key).unwrap_or(0.0) / 100.0 * pool.capacity;
            let is_level = matches!(y.q, Quantity::CapAmount | Quantity::ShieldAmount);
            if !shield
                && is_level
                && req.options.capsim
                && let Some(events) = capsim(ctx, fit, calc, start, hi)
            {
                let last = events.last().map(|e| e.0).unwrap_or(hi);
                let hi = hi.min(last);
                return Some((arcs(&events, &pool, lo, hi), false));
            }
            let pts = sample::sample(lo, hi, SAMPLES, 0, |t| {
                let at = level(start, pool.capacity, pool.period, t);
                if is_level {
                    y_scale(at)
                } else {
                    y_scale(rate(at, pool.capacity, pool.period))
                }
            });
            Some((pts, false))
        }
        _ => {
            let to_abs = |v: f64| match x.unit {
                Unit::Percent => v / 100.0 * pool.capacity,
                _ => v / if shield { eff } else { 1.0 },
            };
            let (lo, hi) = (
                to_abs(lo).clamp(0.0, pool.capacity),
                to_abs(hi).clamp(0.0, pool.capacity),
            );
            let is_level = matches!(y.q, Quantity::CapAmount | Quantity::ShieldAmount);
            let pts = sample::sample(lo, hi, SAMPLES, 0, |at| {
                if is_level {
                    y_scale(at)
                } else {
                    y_scale(rate(at, pool.capacity, pool.period))
                }
            });
            let back = |v: f64| match x.unit {
                Unit::Percent => {
                    if pool.capacity > 0.0 {
                        v * 100.0 / pool.capacity
                    } else {
                        0.0
                    }
                }
                _ => v * if shield { eff } else { 1.0 },
            };
            Some((pts.into_iter().map(|(a, y)| (back(a), y)).collect(), false))
        }
    }
}

pub fn point<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    fit: &Fit,
    calc: &Calculation,
    x_value: f64,
) -> Option<f64> {
    let shield = req.kind == super::Kind::Shield;
    let pool = pool(ctx, calc, shield);
    let x = req.x_axis();
    let y = req.y_axis();
    let eff = if shield && req.options.ehp {
        effectivity(ctx, fit, calc)
    } else {
        1.0
    };
    let is_level = matches!(y.q, Quantity::CapAmount | Quantity::ShieldAmount);
    let at = match x.q {
        Quantity::Time => {
            let start_key = if shield {
                InputKey::ShieldAmountT0
            } else {
                InputKey::CapAmountT0
            };
            let start = req.input(start_key).unwrap_or(0.0) / 100.0 * pool.capacity;
            let t = x_value.max(0.0);
            if !shield
                && is_level
                && req.options.capsim
                && let Some(events) = capsim(ctx, fit, calc, start, t.max(1.0))
            {
                let last = events.last().copied();
                if let Some((lt, _)) = last
                    && t > lt
                {
                    return None;
                }
                let before = events.iter().rev().find(|e| e.0 <= t);
                return before.map(|(et, c)| level(*c, pool.capacity, pool.period, t - et));
            }
            level(start, pool.capacity, pool.period, t)
        }
        _ => match x.unit {
            Unit::Percent => x_value / 100.0 * pool.capacity,
            _ => x_value / if shield { eff } else { 1.0 },
        },
    };
    let value = if is_level {
        at
    } else {
        rate(at, pool.capacity, pool.period)
    };
    Some(value * eff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_reaches_the_peak_rate_at_a_quarter() {
        let peak = rate(250.0, 1000.0, 500.0);
        assert!((peak - 2.5 * 1000.0 / 500.0).abs() < 1e-9);
        assert!(rate(100.0, 1000.0, 500.0) < peak);
        assert!(rate(900.0, 1000.0, 500.0) < peak);
        assert_eq!(rate(1000.0, 1000.0, 500.0), 0.0);
    }

    #[test]
    fn a_full_period_from_empty_lands_near_ninety_nine_percent() {
        let at = level(0.0, 1000.0, 500.0, 500.0);
        assert!((at - 1000.0 * (1.0 - (-5.0f64).exp()).powi(2)).abs() < 1e-6);
        assert!(at > 985.0 && at < 990.0, "{at}");
        assert_eq!(level(1000.0, 1000.0, 500.0, 10.0), 1000.0);
    }

    #[test]
    fn the_level_derivative_is_the_rate() {
        let (c, p) = (1000.0, 500.0);
        for start in [0.0, 100.0, 400.0, 900.0] {
            let h = 1e-3;
            let slope = (level(start, c, p, h) - start) / h;
            let r = rate(start, c, p);
            assert!((slope - r).abs() < 1e-2, "{start}: {slope} vs {r}");
        }
    }
}
