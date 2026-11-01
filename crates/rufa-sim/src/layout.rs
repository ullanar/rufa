use crate::attrs::AttrIds;
use crate::place::Rack;
use crate::{Calculation, ItemResult};

#[derive(Debug, Clone, Copy, Default)]
pub struct SlotLayout {
    pub high: u8,
    pub medium: u8,
    pub low: u8,
    pub rig: u8,
    pub subsystem: u8,
    pub service: u8,
    pub turrets_free: f64,
    pub launchers_free: f64,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

impl SlotLayout {
    pub fn from_calc(ids: &AttrIds, calc: &Calculation) -> SlotLayout {
        let ship = &calc.ship;
        let count = |id: i32| attr(ship, id).round().max(0.0) as u8;
        SlotLayout {
            high: count(ids.hi_slots),
            medium: count(ids.med_slots),
            low: count(ids.low_slots),
            rig: count(ids.rig_slots),
            subsystem: count(ids.max_subsystems),
            service: count(ids.service_slots),
            turrets_free: attr(ship, ids.turret_slots_left),
            launchers_free: attr(ship, ids.launcher_slots_left),
        }
    }

    pub fn count(&self, rack: Rack) -> u8 {
        match rack {
            Rack::High => self.high,
            Rack::Medium => self.medium,
            Rack::Low => self.low,
            Rack::Rig => self.rig,
            Rack::Subsystem => self.subsystem,
            Rack::Service => self.service,
        }
    }
}
