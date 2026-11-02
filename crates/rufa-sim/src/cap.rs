use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::attrs::AttrIds;
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, State};

#[derive(Debug, Clone, PartialEq)]
pub struct Drain {
    pub cycle: f64,
    /// Positive spends capacitor, negative injects it.
    pub amount: f64,
    pub offset: f64,
    /// Cycles the clip holds. None runs on without reloading.
    pub clip: Option<u32>,
    pub reload: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapResult {
    pub depleted_at: Option<f64>,
    pub low: f64,
    pub series: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Event {
    time: f64,
    drain: usize,
}

impl Eq for Event {}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other
            .time
            .total_cmp(&self.time)
            .then(other.drain.cmp(&self.drain))
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Event) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn attr(item: &ItemResult, id: i32) -> Option<f64> {
    item.attributes.get(&id).map(|a| a.value)
}

pub fn recharge(cap: f64, capacity: f64, recharge_time: f64, dt: f64) -> f64 {
    if capacity <= 0.0 || recharge_time <= 0.0 {
        return cap;
    }
    let tau = recharge_time / 5.0;
    let root = (cap / capacity).max(0.0).sqrt();
    let next = 1.0 + (root - 1.0) * (-dt / tau).exp();
    (next * next * capacity).min(capacity)
}

/// The level a cycle later: the capacitor's own recharge plus what a remote
/// transmitter hands over in that time.
fn grow(cap: f64, capacity: f64, recharge_time: f64, incoming: f64, dt: f64) -> f64 {
    (recharge(cap, capacity, recharge_time, dt) + incoming * dt).min(capacity)
}

pub fn drains(ids: &AttrIds, fit: &Fit, calc: &Calculation) -> Vec<Drain> {
    let mut groups: Vec<(i32, Vec<Drain>)> = Vec::new();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if Rack::of_slot(item.slot).is_none() || result.state < State::Active {
            continue;
        }
        let Some(cycle) = attr(result, ids.cycle_time).filter(|c| *c > 0.0) else {
            continue;
        };
        let need = attr(result, ids.capacitor_need).unwrap_or(0.0);
        let injected = attr(result, ids.capacitor_injection_amount).unwrap_or(0.0);
        let amount = need - injected;
        if amount == 0.0 {
            continue;
        }
        // The clip empties only while a charge sits in the module.
        let clip = item.charge.as_ref().and_then(|_| {
            attr(result, ids.charge_amount)
                .map(f64::round)
                .filter(|a| *a >= 1.0)
                .map(|a| a as u32)
        });
        let drain = Drain {
            cycle: cycle / 1000.0,
            amount,
            offset: 0.0,
            clip,
            reload: attr(result, ids.reload_time).unwrap_or(0.0) / 1000.0,
        };
        let entry = match groups.iter_mut().find(|(t, _)| *t == item.type_id) {
            Some(g) => g,
            None => {
                groups.push((item.type_id, Vec::new()));
                groups.last_mut().unwrap()
            }
        };
        entry.1.push(drain);
    }
    let mut out = Vec::new();
    for (_, list) in groups {
        let n = list.len() as f64;
        for (i, mut drain) in list.into_iter().enumerate() {
            let stagger = drain.cycle * i as f64 / n;
            // An injector hands over its capacitor when the cycle ends.
            drain.offset = if drain.amount < 0.0 {
                drain.cycle + stagger
            } else {
                stagger
            };
            out.push(drain);
        }
    }
    out
}

pub fn simulate(
    capacity: f64,
    recharge_time: f64,
    incoming: f64,
    drains: &[Drain],
    horizon: f64,
) -> CapResult {
    let mut cap = capacity;
    let mut low = capacity;
    let mut time = 0.0;
    let mut series = vec![(0.0, cap)];
    let mut fired = vec![0u32; drains.len()];
    let mut heap: BinaryHeap<Event> = drains
        .iter()
        .enumerate()
        .map(|(i, d)| Event {
            time: d.offset,
            drain: i,
        })
        .collect();
    while let Some(event) = heap.pop() {
        if event.time > horizon {
            break;
        }
        let drain = &drains[event.drain];
        let level = grow(cap, capacity, recharge_time, incoming, event.time - time);
        // An injector holds its charge back rather than spill what the
        // capacitor cannot take, and tries again a cycle later.
        if drain.amount < 0.0 && level - drain.amount > capacity + 1e-9 {
            heap.push(Event {
                time: event.time + drain.cycle,
                drain: event.drain,
            });
            continue;
        }
        cap = level;
        time = event.time;
        if drain.amount > cap + 1e-9 {
            series.push((time, cap));
            return CapResult {
                depleted_at: Some(time),
                low: low.min(cap),
                series,
            };
        }
        cap = (cap - drain.amount).min(capacity);
        low = low.min(cap);
        series.push((time, cap));
        let shot = &mut fired[event.drain];
        *shot += 1;
        let mut wait = drain.cycle;
        if let Some(clip) = drain.clip
            && *shot >= clip
        {
            *shot = 0;
            wait += drain.reload;
        }
        heap.push(Event {
            time: time + wait,
            drain: event.drain,
        });
    }
    let end = grow(cap, capacity, recharge_time, incoming, horizon - time);
    series.push((horizon, end));
    CapResult {
        depleted_at: None,
        low,
        series,
    }
}

pub fn simulate_fit(
    ids: &AttrIds,
    fit: &Fit,
    calc: &Calculation,
    incoming: f64,
    horizon: f64,
) -> CapResult {
    let capacity = attr(&calc.ship, ids.capacitor_capacity).unwrap_or(0.0);
    let recharge_time = attr(&calc.ship, ids.recharge_rate).unwrap_or(0.0) / 1000.0;
    simulate(
        capacity,
        recharge_time,
        incoming,
        &drains(ids, fit, calc),
        horizon,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recharge_reaches_the_peak_rate_near_a_quarter() {
        let capacity = 1000.0;
        let tau_total = 500.0;
        let quarter = recharge(250.0, capacity, tau_total, 1.0) - 250.0;
        let full = recharge(990.0, capacity, tau_total, 1.0) - 990.0;
        let empty = recharge(10.0, capacity, tau_total, 1.0) - 10.0;
        assert!(
            quarter > full && quarter > empty,
            "{quarter} {full} {empty}"
        );
        let peak = 2.5 * capacity / tau_total;
        assert!((quarter - peak).abs() / peak < 0.02, "{quarter} vs {peak}");
    }

    #[test]
    fn a_heavy_drain_depletes_and_a_light_one_holds() {
        let heavy = [Drain {
            cycle: 1.0,
            amount: 100.0,
            offset: 0.0,
            clip: None,
            reload: 0.0,
        }];
        let r = simulate(1000.0, 500.0, 0.0, &heavy, 600.0);
        assert!(r.depleted_at.is_some());
        let light = [Drain {
            cycle: 10.0,
            amount: 10.0,
            offset: 0.0,
            clip: None,
            reload: 0.0,
        }];
        let r = simulate(1000.0, 500.0, 0.0, &light, 600.0);
        assert!(r.depleted_at.is_none());
        assert!(r.low > 800.0);
    }

    #[test]
    fn a_clip_that_reloads_drains_slower_than_one_that_does_not() {
        let endless = [Drain {
            cycle: 1.0,
            amount: 60.0,
            offset: 0.0,
            clip: None,
            reload: 0.0,
        }];
        let reloading = [Drain {
            clip: Some(4),
            reload: 10.0,
            ..endless[0].clone()
        }];
        let a = simulate(1000.0, 500.0, 0.0, &endless, 600.0)
            .depleted_at
            .unwrap();
        let b = simulate(1000.0, 500.0, 0.0, &reloading, 600.0)
            .depleted_at
            .unwrap();
        assert!(b > a, "reloading {b} should outlast endless {a}");
    }

    #[test]
    fn an_injector_postpones_what_the_capacitor_cannot_hold() {
        let injector = [Drain {
            cycle: 10.0,
            amount: -400.0,
            offset: 10.0,
            clip: Some(5),
            reload: 10.0,
        }];
        let r = simulate(1000.0, 500.0, 0.0, &injector, 600.0);
        assert!(r.depleted_at.is_none());
        // A full capacitor takes nothing, so every injection waits.
        assert!((r.low - 1000.0).abs() < 1e-6, "{}", r.low);
    }

    #[test]
    fn an_injector_holds_a_drain_the_capacitor_alone_cannot() {
        let drain = Drain {
            cycle: 1.0,
            amount: 40.0,
            offset: 0.0,
            clip: None,
            reload: 0.0,
        };
        let alone = simulate(1000.0, 500.0, 0.0, std::slice::from_ref(&drain), 600.0);
        let injector = Drain {
            cycle: 12.0,
            amount: -400.0,
            offset: 12.0,
            clip: Some(8),
            reload: 10.0,
        };
        let helped = simulate(1000.0, 500.0, 0.0, &[drain, injector], 600.0);
        let a = alone.depleted_at.unwrap();
        if let Some(b) = helped.depleted_at {
            assert!(b > a, "injected {b} should outlast {a}");
        }
        assert!(helped.low > alone.low);
    }

    #[test]
    fn a_remote_transmitter_feeds_the_capacitor_while_it_drains() {
        let drain = [Drain {
            cycle: 1.0,
            amount: 40.0,
            offset: 0.0,
            clip: None,
            reload: 0.0,
        }];
        let alone = simulate(1000.0, 500.0, 0.0, &drain, 600.0);
        let fed = simulate(1000.0, 500.0, 35.0, &drain, 600.0);
        let a = alone.depleted_at.expect("the fit runs dry alone");
        match fed.depleted_at {
            Some(b) => assert!(b > a, "fed {b} should outlast {a}"),
            None => assert!(fed.low > alone.low),
        }
    }
}
