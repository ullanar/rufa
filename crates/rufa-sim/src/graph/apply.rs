//! The hit and application formulas of the game, as plain functions.

use std::f64::consts::PI;

/// Stacking penalty the game applies to the i-th strongest modifier.
pub fn penalty(i: usize) -> f64 {
    (-((i * i) as f64) / 7.1289).exp()
}

/// Multipliers of one stacking group, strongest first, each weakened by its
/// place in the queue. Bonuses and penalties queue separately.
pub fn penalized(mults: &[f64]) -> f64 {
    let mut up: Vec<f64> = mults.iter().copied().filter(|m| *m > 1.0).collect();
    let mut down: Vec<f64> = mults.iter().copied().filter(|m| *m < 1.0).collect();
    up.sort_by(|a, b| (b - 1.0).abs().total_cmp(&(a - 1.0).abs()));
    down.sort_by(|a, b| (b - 1.0).abs().total_cmp(&(a - 1.0).abs()));
    let fold = |list: &[f64]| {
        list.iter()
            .enumerate()
            .fold(1.0, |acc, (i, m)| acc * (1.0 + (m - 1.0) * penalty(i)))
    };
    fold(&up) * fold(&down)
}

/// How much of an effect reaches a target at `d` metres. None is point
/// blank. A restricted effect cannot start past optimal plus three falloffs.
pub fn range_factor(optimal: f64, falloff: f64, d: Option<f64>, restricted: bool) -> f64 {
    let Some(d) = d else {
        return 1.0;
    };
    if falloff > 0.0 {
        if restricted && d > optimal + 3.0 * falloff {
            return 0.0;
        }
        0.5f64.powf(((d - optimal).max(0.0) / falloff).powi(2))
    } else if d <= optimal {
        1.0
    } else {
        0.0
    }
}

/// Radians per second the target sweeps across the attacker, with both on a
/// line and each heading measured from that line.
pub fn angular_speed(
    atk_speed: f64,
    atk_angle: f64,
    atk_radius: f64,
    d: Option<f64>,
    tgt_speed: f64,
    tgt_angle: f64,
    tgt_radius: f64,
) -> f64 {
    let Some(d) = d else {
        return 0.0;
    };
    let a = atk_angle * PI / 180.0;
    let t = tgt_angle * PI / 180.0;
    let centre = atk_radius + d + tgt_radius;
    let transversal = (atk_speed * a.sin() - tgt_speed * t.sin()).abs();
    if centre <= 0.0 {
        return if transversal == 0.0 {
            0.0
        } else {
            f64::INFINITY
        };
    }
    transversal / centre
}

pub fn tracking_factor(angular: f64, optimal_sig: f64, tracking: f64, tgt_sig: f64) -> f64 {
    if angular <= 0.0 || !tgt_sig.is_finite() {
        return 1.0;
    }
    if tracking <= 0.0 || tgt_sig <= 0.0 {
        return 0.0;
    }
    0.5f64.powf(((angular * optimal_sig) / (tracking * tgt_sig)).powi(2))
}

/// Mean damage share of a turret with hit chance `cth`: wrecking shots on
/// the first percent, a 50 to 100 percent spread on the rest.
pub fn turret_mult(cth: f64) -> f64 {
    let cth = cth.clamp(0.0, 1.0);
    let wreck = cth.min(0.01);
    let normal = cth - wreck;
    let normal_part = if normal > 0.0 {
        normal * ((0.01 + cth) / 2.0 + 0.49)
    } else {
        0.0
    };
    normal_part + 3.0 * wreck
}

pub fn missile_factor(radius: f64, velocity: f64, drf: f64, tgt_speed: f64, tgt_sig: f64) -> f64 {
    let mut factor: f64 = 1.0;
    if radius > 0.0 && tgt_sig.is_finite() {
        factor = factor.min(tgt_sig / radius);
    }
    if tgt_speed > 0.0 && radius > 0.0 && tgt_sig.is_finite() {
        factor = factor.min(((velocity * tgt_sig) / (radius * tgt_speed)).powf(drf));
    }
    factor
}

pub fn bomb_factor(radius: f64, tgt_sig: f64) -> f64 {
    if radius <= 0.0 || !tgt_sig.is_finite() {
        1.0
    } else {
        (tgt_sig / radius).min(1.0)
    }
}

/// Metres a missile covers in `t` seconds, speeding up over the first
/// `mass * agility / 1e6` seconds.
pub fn missile_range(velocity: f64, mass: f64, agility: f64, t: f64) -> f64 {
    let accel = (mass * agility / 1e6).min(t).max(0.0);
    velocity / 2.0 * accel + velocity * (t - accel)
}

/// How far a missile flies, split by the whole seconds the flight time
/// straddles. The game counts flight time in whole seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flight {
    pub lower: f64,
    pub higher: f64,
    pub higher_chance: f64,
}

impl Flight {
    pub fn new(
        velocity: f64,
        delay_s: f64,
        mass: f64,
        agility: f64,
        ship_radius: f64,
        fof_cap: Option<f64>,
    ) -> Option<Flight> {
        if velocity <= 0.0 {
            return None;
        }
        let t = unerr(delay_s + ship_radius / velocity);
        let lo = t.floor();
        let hi = t.ceil();
        let mut lower = missile_range(velocity, mass, agility, lo);
        let mut higher = missile_range(velocity, mass, agility, hi);
        if let Some(cap) = fof_cap {
            lower = lower.min(cap);
            higher = higher.min(cap);
        }
        Some(Flight {
            lower: (lower - ship_radius).max(0.0),
            higher: (higher - ship_radius).max(0.0),
            higher_chance: t - lo,
        })
    }

    pub fn factor(&self, d: Option<f64>) -> f64 {
        match d {
            None => 1.0,
            Some(d) if d <= self.lower => 1.0,
            Some(d) if d <= self.higher => self.higher_chance,
            _ => 0.0,
        }
    }

    pub fn expected(&self) -> f64 {
        self.lower * (1.0 - self.higher_chance) + self.higher * self.higher_chance
    }
}

/// Rounds to ten significant digits so equal quantities compare equal.
pub fn unerr(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let digits = 10 - v.abs().log10().floor() as i32 - 1;
    let scale = 10f64.powi(digits);
    (v * scale).round() / scale
}

/// Lock time against a signature, capped at thirty minutes.
pub fn lock_time(scan_resolution: f64, signature: f64) -> Option<f64> {
    if scan_resolution <= 0.0 || signature <= 0.0 {
        return None;
    }
    Some((40_000.0 / (scan_resolution * signature.asinh().powi(2))).min(1800.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn the_stacking_penalty_matches_the_engine_constant() {
        assert!(close(penalty(0), 1.0));
        assert!(close(penalty(1), 0.869_119_980_800_397_4));
        assert!(close(
            penalized(&[0.4, 0.4]),
            0.4 * (1.0 - 0.6 * penalty(1))
        ));
        assert!(close(penalized(&[1.5, 0.5]), 1.5 * 0.5));
    }

    #[test]
    fn range_factor_halves_at_one_falloff_and_cuts_at_three() {
        assert!(close(
            range_factor(10_000.0, 5_000.0, Some(15_000.0), false),
            0.5
        ));
        assert_eq!(range_factor(10_000.0, 5_000.0, Some(5_000.0), false), 1.0);
        assert_eq!(range_factor(10_000.0, 5_000.0, None, false), 1.0);
        assert_eq!(range_factor(10_000.0, 5_000.0, Some(25_001.0), true), 0.0);
        assert!(range_factor(10_000.0, 5_000.0, Some(25_001.0), false) > 0.0);
        assert_eq!(range_factor(10_000.0, 0.0, Some(10_000.0), false), 1.0);
        assert_eq!(range_factor(10_000.0, 0.0, Some(10_001.0), false), 0.0);
    }

    #[test]
    fn a_target_crossing_at_a_kilometre_sweeps_its_speed_over_the_distance() {
        let w = angular_speed(0.0, 90.0, 0.0, Some(1000.0), 500.0, 90.0, 0.0);
        assert!(close(w, 0.5));
        let same_way = angular_speed(500.0, 90.0, 0.0, Some(1000.0), 500.0, 90.0, 0.0);
        assert_eq!(same_way, 0.0);
        let head_on = angular_speed(0.0, 90.0, 0.0, Some(1000.0), 500.0, 0.0, 0.0);
        assert!(close(head_on, 0.0));
        assert_eq!(angular_speed(0.0, 90.0, 0.0, None, 500.0, 90.0, 0.0), 0.0);
    }

    #[test]
    fn tracking_factor_halves_when_the_terms_match() {
        assert!(close(tracking_factor(0.1, 40.0, 0.1, 40.0), 0.5));
        assert_eq!(tracking_factor(0.0, 40.0, 0.1, 40.0), 1.0);
        assert_eq!(tracking_factor(0.1, 40.0, 0.1, f64::INFINITY), 1.0);
        assert_eq!(tracking_factor(0.1, 40.0, 0.0, 40.0), 0.0);
    }

    #[test]
    fn turret_mult_spreads_hits_the_way_the_game_does() {
        assert!(close(
            turret_mult(1.0),
            0.99 * ((0.01 + 1.0) / 2.0 + 0.49) + 0.03
        ));
        assert!(close(turret_mult(1.0), 1.01505));
        assert!(close(
            turret_mult(0.5),
            0.49 * ((0.01 + 0.5) / 2.0 + 0.49) + 0.03
        ));
        assert!(close(turret_mult(0.005), 0.015));
        assert_eq!(turret_mult(0.0), 0.0);
    }

    #[test]
    fn missile_factor_takes_the_worse_of_size_and_speed() {
        assert!(close(missile_factor(100.0, 150.0, 2.0, 0.0, 50.0), 0.5));
        let speed = ((150.0_f64 * 50.0) / (100.0 * 300.0)).powf(2.0);
        assert!(close(missile_factor(100.0, 150.0, 2.0, 300.0, 50.0), speed));
        assert_eq!(missile_factor(100.0, 150.0, 2.0, 300.0, f64::INFINITY), 1.0);
        assert_eq!(missile_factor(0.0, 150.0, 2.0, 300.0, 50.0), 1.0);
        assert_eq!(missile_factor(100.0, 150.0, 2.0, 0.0, 400.0), 1.0);
    }

    #[test]
    fn a_missile_flight_splits_on_whole_seconds() {
        let f = Flight::new(2000.0, 5.5, 0.0, 0.0, 0.0, None).unwrap();
        assert!(close(f.lower, 10_000.0));
        assert!(close(f.higher, 12_000.0));
        assert!(close(f.higher_chance, 0.5));
        assert_eq!(f.factor(None), 1.0);
        assert_eq!(f.factor(Some(9_000.0)), 1.0);
        assert!(close(f.factor(Some(11_000.0)), 0.5));
        assert_eq!(f.factor(Some(13_000.0)), 0.0);
        assert!(close(f.expected(), 11_000.0));
        assert!(Flight::new(0.0, 5.0, 0.0, 0.0, 0.0, None).is_none());
    }

    #[test]
    fn a_heavy_missile_loses_the_acceleration_time() {
        let full = missile_range(1000.0, 0.0, 0.0, 4.0);
        let slow = missile_range(1000.0, 1000.0, 2000.0, 4.0);
        assert!(close(full, 4000.0));
        assert!(close(slow, 3000.0));
    }

    #[test]
    fn unerr_keeps_ten_digits() {
        assert_eq!(unerr(0.1 + 0.2), 0.3);
        assert_eq!(unerr(123_456.789_012_345), 123_456.789);
        assert_eq!(unerr(0.0), 0.0);
    }

    #[test]
    fn lock_time_matches_the_wiki_formula() {
        let t = lock_time(400.0, 40.0).unwrap();
        assert!(close(t, 40_000.0 / (400.0 * 40f64.asinh().powi(2))));
        assert_eq!(lock_time(0.01, 1.0).unwrap(), 1800.0);
        assert!(lock_time(0.0, 40.0).is_none());
    }
}
