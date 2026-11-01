use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::attrs::AttrIds;
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, State};

#[derive(Debug, Clone, PartialEq)]
pub struct Drain {
    pub cycle: f64,
    pub amount: f64,
    pub offset: f64,
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

pub fn drains(ids: &AttrIds, fit: &Fit, calc: &Calculation) -> Vec<Drain> {
    let mut groups: Vec<(i32, Vec<(f64, f64)>)> = Vec::new();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if Rack::of_slot(item.slot).is_none() || result.state < State::Active {
            continue;
        }
        let Some(cycle) = attr(result, ids.cycle_time).filter(|c| *c > 0.0) else {
            continue;
        };
        let need = attr(result, ids.capacitor_need).unwrap_or(0.0);
        if need == 0.0 {
            continue;
        }
        let entry = match groups.iter_mut().find(|(t, _)| *t == item.type_id) {
            Some(g) => g,
            None => {
                groups.push((item.type_id, Vec::new()));
                groups.last_mut().unwrap()
            }
        };
        entry.1.push((cycle / 1000.0, need));
    }
    let mut out = Vec::new();
    for (_, list) in groups {
        let n = list.len() as f64;
        for (i, (cycle, amount)) in list.into_iter().enumerate() {
            out.push(Drain {
                cycle,
                amount,
                offset: cycle * i as f64 / n,
            });
        }
    }
    out
}

pub fn simulate(capacity: f64, recharge_time: f64, drains: &[Drain], horizon: f64) -> CapResult {
    let mut cap = capacity;
    let mut low = capacity;
    let mut time = 0.0;
    let mut series = vec![(0.0, cap)];
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
        cap = recharge(cap, capacity, recharge_time, event.time - time);
        time = event.time;
        let drain = &drains[event.drain];
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
        heap.push(Event {
            time: time + drain.cycle,
            drain: event.drain,
        });
    }
    let end = recharge(cap, capacity, recharge_time, horizon - time);
    series.push((horizon, end));
    CapResult {
        depleted_at: None,
        low,
        series,
    }
}

pub fn simulate_fit(ids: &AttrIds, fit: &Fit, calc: &Calculation, horizon: f64) -> CapResult {
    let capacity = attr(&calc.ship, ids.capacitor_capacity).unwrap_or(0.0);
    let recharge_time = attr(&calc.ship, ids.recharge_rate).unwrap_or(0.0) / 1000.0;
    simulate(capacity, recharge_time, &drains(ids, fit, calc), horizon)
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
        }];
        let r = simulate(1000.0, 500.0, &heavy, 600.0);
        assert!(r.depleted_at.is_some());
        let light = [Drain {
            cycle: 10.0,
            amount: 10.0,
            offset: 0.0,
        }];
        let r = simulate(1000.0, 500.0, &light, 600.0);
        assert!(r.depleted_at.is_none());
        assert!(r.low > 800.0);
    }
}
