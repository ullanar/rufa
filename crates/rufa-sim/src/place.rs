use rufa_data::{Info, catalog::category};

use crate::attrs::AttrIds;
use crate::{FitItem, Slot, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rack {
    Subsystem,
    High,
    Medium,
    Low,
    Rig,
    Service,
}

impl Rack {
    pub const ALL: [Rack; 6] = [
        Rack::Subsystem,
        Rack::High,
        Rack::Medium,
        Rack::Low,
        Rack::Rig,
        Rack::Service,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Rack::Subsystem => "Subsystems",
            Rack::High => "High",
            Rack::Medium => "Medium",
            Rack::Low => "Low",
            Rack::Rig => "Rigs",
            Rack::Service => "Services",
        }
    }

    pub fn slot(self, index: u8) -> Slot {
        match self {
            Rack::Subsystem => Slot::Subsystem(index),
            Rack::High => Slot::High(index),
            Rack::Medium => Slot::Medium(index),
            Rack::Low => Slot::Low(index),
            Rack::Rig => Slot::Rig(index),
            Rack::Service => Slot::Service(index),
        }
    }

    pub fn of_slot(slot: Slot) -> Option<(Rack, u8)> {
        Some(match slot {
            Slot::Subsystem(i) => (Rack::Subsystem, i),
            Slot::High(i) => (Rack::High, i),
            Slot::Medium(i) => (Rack::Medium, i),
            Slot::Low(i) => (Rack::Low, i),
            Slot::Rig(i) => (Rack::Rig, i),
            Slot::Service(i) => (Rack::Service, i),
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Module(Rack),
    Drone,
    Fighter,
    Implant(u8),
    Booster(u16),
    Charge,
    Cargo,
}

const EFFECT_LO_POWER: i32 = 11;
const EFFECT_HI_POWER: i32 = 12;
const EFFECT_MED_POWER: i32 = 13;
const EFFECT_RIG_SLOT: i32 = 2663;
const EFFECT_SUBSYSTEM: i32 = 3772;
const EFFECT_SERVICE_SLOT: i32 = 6306;

pub fn placement(info: &impl Info, ids: &AttrIds, type_id: i32) -> Option<Placement> {
    let ty = info.get_type(type_id)?;
    for effect in ty.dogma_effects().into_iter().flatten() {
        let rack = match effect.effect_id() {
            EFFECT_HI_POWER => Rack::High,
            EFFECT_MED_POWER => Rack::Medium,
            EFFECT_LO_POWER => Rack::Low,
            EFFECT_RIG_SLOT => Rack::Rig,
            EFFECT_SUBSYSTEM => Rack::Subsystem,
            EFFECT_SERVICE_SLOT => Rack::Service,
            _ => continue,
        };
        return Some(Placement::Module(rack));
    }
    let attr = |id: i32| {
        ty.dogma_attributes()
            .into_iter()
            .flatten()
            .find(|a| a.attribute_id() == id)
            .map(|a| a.value())
    };
    Some(match ty.category_id() {
        category::DRONE => Placement::Drone,
        category::FIGHTER => Placement::Fighter,
        category::CHARGE => Placement::Charge,
        category::IMPLANT => {
            if let Some(slot) = attr(ids.implantness) {
                Placement::Implant(slot as u8)
            } else if let Some(slot) = attr(ids.boosterness) {
                Placement::Booster(slot as u16)
            } else {
                Placement::Cargo
            }
        }
        _ => Placement::Cargo,
    })
}

pub fn initial_state(placement: Placement) -> State {
    match placement {
        Placement::Module(Rack::Rig | Rack::Subsystem) => State::Online,
        Placement::Module(_) => State::Active,
        Placement::Drone | Placement::Fighter => State::Active,
        Placement::Implant(_) | Placement::Booster(_) => State::Online,
        Placement::Charge | Placement::Cargo => State::Offline,
    }
}

pub fn new_item(type_id: i32, slot: Slot, state: State) -> FitItem {
    FitItem {
        type_id,
        slot,
        quantity: 1,
        state,
        charge: None,
        mutation: None,
        fighter_abilities: None,
        booster_side_effects: Default::default(),
        spool: None,
    }
}

pub fn accepts_charge(info: &impl Info, ids: &AttrIds, module_type: i32, charge_type: i32) -> bool {
    let (Some(module), Some(charge)) = (info.get_type(module_type), info.get_type(charge_type))
    else {
        return false;
    };
    if charge.category_id() != category::CHARGE {
        return false;
    }
    let attr = |ty: &rufa_data::eve::Type<'_>, id: i32| {
        ty.dogma_attributes()
            .into_iter()
            .flatten()
            .find(|a| a.attribute_id() == id)
            .map(|a| a.value())
    };
    let groups = [
        ids.charge_group1,
        ids.charge_group2,
        ids.charge_group3,
        ids.charge_group4,
        ids.charge_group5,
    ];
    let group_ok = groups
        .iter()
        .filter_map(|id| attr(&module, *id))
        .any(|g| g as i32 == charge.group_id());
    if !group_ok {
        return false;
    }
    match (
        attr(&module, ids.charge_size),
        attr(&charge, ids.charge_size),
    ) {
        (Some(m), Some(c)) => (m - c).abs() < 0.5,
        _ => true,
    }
}

pub fn charge_groups(info: &impl Info, ids: &AttrIds, module_type: i32) -> Vec<i32> {
    let Some(module) = info.get_type(module_type) else {
        return Vec::new();
    };
    let groups = [
        ids.charge_group1,
        ids.charge_group2,
        ids.charge_group3,
        ids.charge_group4,
        ids.charge_group5,
    ];
    module
        .dogma_attributes()
        .into_iter()
        .flatten()
        .filter(|a| groups.contains(&a.attribute_id()) && a.value() > 0.0)
        .map(|a| a.value() as i32)
        .collect()
}
