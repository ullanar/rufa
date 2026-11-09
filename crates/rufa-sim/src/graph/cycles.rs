//! How a module fires over time: cycle length, charges per clip, reload,
//! reactivation delay and spool-up.

use crate::attrs::AttrIds;
use crate::{FitItem, ItemResult};

/// One run of identical cycles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Run {
    /// Seconds the module works.
    pub active: f64,
    /// Seconds it waits after that.
    pub inactive: f64,
    /// How many times in a row. None repeats forever.
    pub count: Option<u32>,
    /// The wait is a reload, which resets any spool.
    pub reload: bool,
}

/// The runs repeat in order, forever once the last one is reached.
#[derive(Debug, Clone, PartialEq)]
pub struct Cycles {
    pub runs: Vec<Run>,
}

impl Cycles {
    pub fn forever(active: f64, inactive: f64) -> Cycles {
        Cycles {
            runs: vec![Run {
                active,
                inactive,
                count: None,
                reload: false,
            }],
        }
    }

    /// Mean cycle length over one pass through the runs, in seconds.
    pub fn average(&self) -> f64 {
        let mut total = 0.0;
        let mut cycles = 0.0;
        for run in &self.runs {
            let n = f64::from(run.count.unwrap_or(1));
            total += (run.active + run.inactive) * n;
            cycles += n;
        }
        if cycles > 0.0 { total / cycles } else { 0.0 }
    }

    /// Hands out every cycle start up to `until` seconds with its run and
    /// the count of cycles that ran back to back before it. False stops.
    pub fn walk(&self, until: f64, mut each: impl FnMut(f64, Run, u32) -> bool) {
        if self.runs.is_empty() {
            return;
        }
        let mut t = 0.0;
        let mut nonstop = 0u32;
        let mut guard = 0u32;
        loop {
            for run in &self.runs {
                let count = run.count.unwrap_or(u32::MAX);
                for _ in 0..count {
                    if !each(t, *run, nonstop) {
                        return;
                    }
                    nonstop = if run.inactive > 0.0 { 0 } else { nonstop + 1 };
                    t += run.active + run.inactive;
                    guard += 1;
                    if t > until || guard > 1_000_000 {
                        return;
                    }
                }
            }
        }
    }
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn has(item: &ItemResult, id: i32) -> bool {
    item.attributes.contains_key(&id)
}

/// Seconds one activation takes.
pub fn raw_cycle(ids: &AttrIds, result: &ItemResult) -> f64 {
    [
        attr(result, ids.speed),
        attr(result, ids.duration),
        attr(result, ids.duration_high_is_good),
    ]
    .into_iter()
    .fold(0.0, f64::max)
        / 1000.0
}

/// Activations one clip gives. None means it never runs out.
pub fn shots(ids: &AttrIds, item: &FitItem, result: &ItemResult) -> Option<u32> {
    let charge = result.charge.as_deref()?;
    item.charge.as_ref()?;
    let volume = attr(charge, ids.volume);
    if volume <= 0.0 {
        return None;
    }
    let charges = (attr(result, ids.capacity) / volume).floor();
    if has(result, ids.charge_rate) {
        let rate = attr(result, ids.charge_rate);
        if rate <= 0.0 {
            return None;
        }
        return Some((charges / rate).floor() as u32);
    }
    if attr(charge, ids.crystals_get_damaged) > 0.0 {
        let wear = attr(charge, ids.crystal_volatility_damage)
            * attr(charge, ids.crystal_volatility_chance);
        if wear <= 0.0 {
            return None;
        }
        return Some((charges * attr(charge, ids.hull_hp) / wear).floor() as u32);
    }
    None
}

pub fn reload_time(ids: &AttrIds, result: &ItemResult) -> f64 {
    attr(result, ids.reload_time) / 1000.0
}

/// The cycles of a racked module. With `reload` off the clip never runs out.
pub fn module_cycles(
    ids: &AttrIds,
    item: &FitItem,
    result: &ItemResult,
    reload: bool,
) -> Option<Cycles> {
    let active = raw_cycle(ids, result);
    if active <= 0.0 {
        return None;
    }
    let inactive = attr(result, ids.module_reactivation_delay) / 1000.0;
    let reload_time = reload_time(ids, result);
    let shots = shots(ids, item, result);
    let Some(shots) = shots.filter(|_| reload) else {
        return Some(Cycles::forever(active, inactive));
    };
    if inactive >= reload_time {
        return Some(Cycles {
            runs: vec![Run {
                active,
                inactive,
                count: None,
                reload: true,
            }],
        });
    }
    if shots <= 1 {
        return Some(Cycles {
            runs: vec![Run {
                active,
                inactive: reload_time,
                count: None,
                reload: true,
            }],
        });
    }
    Some(Cycles {
        runs: vec![
            Run {
                active,
                inactive,
                count: Some(shots - 1),
                reload: false,
            },
            Run {
                active,
                inactive: reload_time,
                count: Some(1),
                reload: true,
            },
        ],
    })
}

/// The cycle of a drone: its launcher if it fires missiles, else its own.
pub fn drone_cycle(ids: &AttrIds, result: &ItemResult) -> Option<f64> {
    let seconds = if result.charge.is_some() {
        attr(result, ids.missile_launch_duration)
    } else {
        [
            attr(result, ids.speed),
            attr(result, ids.duration),
            attr(result, ids.duration_high_is_good),
        ]
        .into_iter()
        .find(|v| *v > 0.0)
        .unwrap_or(0.0)
    } / 1000.0;
    (seconds > 0.0).then_some(seconds)
}

/// How much a spooling weapon or repairer has run up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Spool {
    /// Share of the maximum, 0 to 1.
    Scale(f64),
    /// Cycles done back to back.
    Cycles(u32),
    /// Seconds done back to back.
    Time(f64),
}

/// The bonus after the given spool, from the per cycle step and the cap.
/// Zero when the item does not spool.
pub fn spool_bonus(max: f64, step: f64, cycle: f64, spool: Spool) -> f64 {
    if max <= 0.0 || step <= 0.0 {
        return 0.0;
    }
    let full = (max / step).ceil();
    let cycles = match spool {
        Spool::Scale(share) => (max * share.clamp(0.0, 1.0) / step).ceil(),
        Spool::Cycles(n) => f64::from(n).min(full),
        Spool::Time(t) => {
            if cycle <= 0.0 {
                full
            } else {
                (t / cycle).floor().min(full)
            }
        }
    };
    (cycles * step).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_weighs_runs_by_their_count() {
        let c = Cycles {
            runs: vec![
                Run {
                    active: 2.0,
                    inactive: 0.0,
                    count: Some(3),
                    reload: false,
                },
                Run {
                    active: 2.0,
                    inactive: 10.0,
                    count: Some(1),
                    reload: true,
                },
            ],
        };
        assert!((c.average() - 4.5).abs() < 1e-12);
        assert_eq!(Cycles::forever(3.0, 1.0).average(), 4.0);
    }

    #[test]
    fn a_walk_resets_the_nonstop_count_at_a_pause() {
        let c = Cycles {
            runs: vec![
                Run {
                    active: 1.0,
                    inactive: 0.0,
                    count: Some(2),
                    reload: false,
                },
                Run {
                    active: 1.0,
                    inactive: 5.0,
                    count: Some(1),
                    reload: true,
                },
            ],
        };
        let mut seen = Vec::new();
        c.walk(20.0, |t, _, nonstop| {
            seen.push((t, nonstop));
            true
        });
        assert_eq!(seen[..4], [(0.0, 0), (1.0, 1), (2.0, 2), (8.0, 0)]);
    }

    #[test]
    fn spool_caps_at_the_maximum() {
        assert!((spool_bonus(0.5, 0.05, 1.0, Spool::Scale(1.0)) - 0.5).abs() < 1e-12);
        assert!((spool_bonus(0.5, 0.05, 1.0, Spool::Scale(0.5)) - 0.25).abs() < 1e-12);
        assert!((spool_bonus(0.5, 0.05, 1.0, Spool::Cycles(3)) - 0.15).abs() < 1e-12);
        assert!((spool_bonus(0.5, 0.05, 1.0, Spool::Cycles(30)) - 0.5).abs() < 1e-12);
        assert!((spool_bonus(0.5, 0.05, 2.0, Spool::Time(5.0)) - 0.1).abs() < 1e-12);
        assert_eq!(spool_bonus(0.0, 0.05, 1.0, Spool::Cycles(3)), 0.0);
    }
}
