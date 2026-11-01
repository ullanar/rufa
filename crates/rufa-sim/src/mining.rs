//! Ore a fit pulls in per second, from miners and from mining drones.

use crate::attrs::AttrIds;
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, Slot, State};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mining {
    pub miner: f64,
    pub drone: f64,
    pub drain: f64,
}

impl Mining {
    pub fn total(self) -> f64 {
        self.miner + self.drone
    }

    pub fn per_hour(self) -> f64 {
        self.total() * 3600.0
    }

    pub fn any(self) -> bool {
        self.total() > 0.0
    }
}

fn attr(result: &ItemResult, id: i32) -> f64 {
    result.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

pub fn yields(ids: &AttrIds, fit: &Fit, calc: &Calculation) -> Mining {
    let mut out = Mining::default();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active {
            continue;
        }
        let cycle = attr(result, ids.duration) / 1000.0;
        let amount = attr(result, ids.mining_amount);
        if cycle <= 0.0 || amount <= 0.0 {
            continue;
        }
        let per_second = amount / cycle;
        match item.slot {
            Slot::DroneBay => out.drone += per_second * f64::from(item.quantity),
            slot if Rack::of_slot(slot).is_some() => {
                let crit = attr(result, ids.mining_crit_chance)
                    * attr(result, ids.mining_crit_bonus_yield);
                out.miner += per_second * (1.0 + crit);
                out.drain += attr(result, ids.capacitor_need) / cycle;
            }
            _ => continue,
        }
    }
    out
}
