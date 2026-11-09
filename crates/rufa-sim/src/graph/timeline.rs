//! Damage of every dealer laid out on a clock: cycles, reloads and spool.

use super::dealers::{Dealer, Weapon};
use super::dmg::Dmg;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Line {
    /// DPS from each time on, until the next entry.
    pub dps: Vec<(f64, Dmg)>,
    /// The largest volley of the cycle running from each time on.
    pub volley: Vec<(f64, Dmg)>,
    /// Damage landing at each time, with the running total after it.
    pub events: Vec<(f64, Dmg, Dmg)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timeline {
    pub lines: Vec<Line>,
    pub until: f64,
}

fn push_change(list: &mut Vec<(f64, Dmg)>, t: f64, v: Dmg) {
    if list.last().is_some_and(|(lt, _)| *lt == t) {
        list.pop();
    }
    if list.last().is_some_and(|(_, lv)| *lv == v) {
        return;
    }
    list.push((t, v));
}

/// Builds the timelines of all dealers from zero to `until` seconds.
pub fn build(dealers: &[Dealer], until: f64) -> Timeline {
    let mut lines = Vec::with_capacity(dealers.len());
    for dealer in dealers {
        let mut line = Line::default();
        let Some(cycles) = &dealer.cycles else {
            lines.push(line);
            continue;
        };
        let mut segments: Vec<(f64, f64, Dmg, Dmg)> = Vec::new();
        let mut events: Vec<(f64, Dmg)> = Vec::new();
        let breacher = matches!(dealer.weapon, Weapon::Breacher { .. });
        cycles.walk(until, |t, run, nonstop| {
            let volley = dealer.volley_at(nonstop);
            if breacher {
                if let Weapon::Breacher { ticks, .. } = dealer.weapon {
                    for k in 0..ticks {
                        let at = t + 1.0 + f64::from(k);
                        events.push((at, volley));
                        segments.push((at, at + 1.0, volley, volley));
                    }
                }
            } else {
                let mut sum = Dmg::default();
                let mut best = Dmg::default();
                for delay in &dealer.delays {
                    events.push((t + delay, volley));
                    sum = sum.plus(volley);
                    if volley.total() > best.total() {
                        best = volley;
                    }
                }
                if sum.total() > 0.0 && run.active > 0.0 {
                    segments.push((t, t + run.active, sum.scale(1.0 / run.active), best));
                }
            }
            t <= until
        });
        if breacher {
            segments.sort_by(|a, b| a.0.total_cmp(&b.0));
            segments.dedup_by(|a, b| {
                if a.0 == b.0 {
                    if a.2.total() > b.2.total() {
                        b.2 = a.2;
                        b.3 = a.3;
                    }
                    true
                } else {
                    false
                }
            });
        }
        let mut end = None::<f64>;
        for (start, stop, dps, best) in segments {
            if let Some(e) = end
                && start > e
            {
                push_change(&mut line.dps, e, Dmg::default());
                push_change(&mut line.volley, e, Dmg::default());
            }
            push_change(&mut line.dps, start, dps);
            push_change(&mut line.volley, start, best);
            end = Some(stop);
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut total = Dmg::default();
        for (t, v) in events {
            total = total.plus(v);
            line.events.push((t, v, total));
        }
        lines.push(line);
    }
    Timeline { lines, until }
}

fn at(list: &[(f64, Dmg)], t: f64) -> Dmg {
    let i = list.partition_point(|(lt, _)| *lt <= t + 1e-9);
    if i == 0 {
        Dmg::default()
    } else {
        list[i - 1].1
    }
}

impl Line {
    pub fn dps_at(&self, t: f64) -> Dmg {
        at(&self.dps, t)
    }

    pub fn volley_at(&self, t: f64) -> Dmg {
        at(&self.volley, t)
    }

    pub fn damage_at(&self, t: f64) -> Dmg {
        let i = self.events.partition_point(|(et, _, _)| *et <= t + 1e-9);
        if i == 0 {
            Dmg::default()
        } else {
            self.events[i - 1].2
        }
    }
}

impl Timeline {
    /// Every time at which some line changes, within the window.
    pub fn changes(&self, lo: f64, hi: f64, damage: bool) -> Vec<f64> {
        let mut times: Vec<f64> = self
            .lines
            .iter()
            .flat_map(|l| {
                let a = l.dps.iter().map(|(t, _)| *t);
                let b = l.volley.iter().map(|(t, _)| *t);
                let c = l.events.iter().map(|(t, _, _)| *t);
                if damage {
                    c.collect::<Vec<f64>>()
                } else {
                    a.chain(b).collect()
                }
            })
            .filter(|t| *t >= lo && *t <= hi)
            .collect();
        times.sort_by(|a, b| a.total_cmp(b));
        times.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        times
    }
}

#[cfg(test)]
mod tests {
    use super::super::cycles::{Cycles, Run};
    use super::*;

    fn gun(volley: f64, active: f64, shots: u32, reload: f64) -> Dealer {
        Dealer {
            index: 0,
            weapon: Weapon::Smartbomb { range: 1.0 },
            volley: Dmg::types([volley, 0.0, 0.0, 0.0]),
            dps: Dmg::default(),
            cycles: Some(Cycles {
                runs: vec![
                    Run {
                        active,
                        inactive: 0.0,
                        count: Some(shots - 1),
                        reload: false,
                    },
                    Run {
                        active,
                        inactive: reload,
                        count: Some(1),
                        reload: true,
                    },
                ],
            }),
            spool: (0.0, 0.0),
            delays: vec![0.0],
        }
    }

    #[test]
    fn a_reload_shows_as_a_gap_in_dps_and_a_plateau_in_damage() {
        let tl = build(&[gun(100.0, 2.0, 3, 10.0)], 30.0);
        let line = &tl.lines[0];
        assert_eq!(line.dps_at(0.0).em, 50.0);
        assert_eq!(line.dps_at(5.0).em, 50.0);
        assert_eq!(line.dps_at(7.0).em, 0.0);
        assert_eq!(line.dps_at(16.0).em, 50.0);
        assert_eq!(line.damage_at(0.0).em, 100.0);
        assert_eq!(line.damage_at(3.0).em, 200.0);
        assert_eq!(line.damage_at(15.0).em, 300.0);
        assert_eq!(line.damage_at(16.0).em, 400.0);
        assert_eq!(line.volley_at(1.0).em, 100.0);
    }

    #[test]
    fn spool_grows_with_uninterrupted_cycles_and_resets_on_reload() {
        let mut d = gun(100.0, 1.0, 3, 5.0);
        d.spool = (0.5, 0.25);
        let tl = build(&[d], 30.0);
        let line = &tl.lines[0];
        assert_eq!(line.volley_at(0.0).em, 100.0);
        assert_eq!(line.volley_at(1.0).em, 125.0);
        assert_eq!(line.volley_at(2.0).em, 150.0);
        assert_eq!(line.volley_at(8.0).em, 100.0);
    }
}
