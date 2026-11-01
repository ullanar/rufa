use rufa_data::Info;
use rufa_data::catalog::group;

use crate::attrs::AttrIds;
use crate::place::{Placement, Rack, placement};
use crate::repair::Remote;
use crate::{Calculation, Character, Charge, Fit, FitItem, Options, Projection, Slot, State};

#[derive(Debug, Clone, Default)]
pub struct Incoming {
    pub projection: Projection,
    pub reps: Remote,
    pub jammers: Vec<[f64; 4]>,
}

#[derive(Debug, Clone)]
pub enum Projected {
    Beacon(i32),
    Module {
        type_id: i32,
        charge: Option<i32>,
        quantity: u32,
    },
    Fit {
        name: String,
        fit: Box<Fit>,
    },
}

pub fn incoming(info: &impl Info, ids: &AttrIds, target: &Fit, sources: &[Projected]) -> Incoming {
    let options = Options {
        sources: false,
        validate: false,
    };
    let mut total = Incoming::default();
    for source in sources {
        let fit = match source {
            Projected::Beacon(type_id) => {
                let part = esf_dogma_engine::beacon(info, *type_id);
                total.projection.buffs.extend(part.buffs);
                total.projection.effects.extend(part.effects);
                continue;
            }
            Projected::Module {
                type_id,
                charge,
                quantity,
            } => match carrier(info, ids, target, *type_id, *charge, *quantity) {
                Some(fit) => fit,
                None => continue,
            },
            Projected::Fit { fit, .. } => (**fit).clone(),
        };
        let calc = esf_dogma_engine::calculate(info, &fit, &options);
        total.projection.buffs.extend(calc.outgoing.buffs.clone());
        total
            .projection
            .effects
            .extend(calc.outgoing.effects.clone());
        total.reps = total
            .reps
            .plus(crate::repair::outgoing(info, ids, &fit, &calc));
        total.jammers.extend(jammers(info, ids, &fit, &calc));
    }
    total
}

fn jammers(info: &impl Info, ids: &AttrIds, fit: &Fit, calc: &Calculation) -> Vec<[f64; 4]> {
    let mut out = Vec::new();
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active || Rack::of_slot(item.slot).is_none() {
            continue;
        }
        if info.get_type(item.type_id).map(|t| t.group_id()) != Some(group::ECM) {
            continue;
        }
        let attr = |id: i32| result.attributes.get(&id).map(|a| a.value).unwrap_or(0.0);
        let strength = [
            attr(ids.scan_radar_strength_bonus),
            attr(ids.scan_ladar_strength_bonus),
            attr(ids.scan_magnetometric_strength_bonus),
            attr(ids.scan_gravimetric_strength_bonus),
        ];
        if strength.iter().all(|value| *value <= 0.0) {
            continue;
        }
        for _ in 0..item.quantity.max(1) {
            out.push(strength);
        }
    }
    out
}

fn carrier(
    info: &impl Info,
    ids: &AttrIds,
    target: &Fit,
    type_id: i32,
    charge: Option<i32>,
    quantity: u32,
) -> Option<Fit> {
    let slot = match placement(info, ids, type_id) {
        Some(Placement::Module(rack)) => rack.slot(0),
        Some(Placement::Drone) => Slot::DroneBay,
        Some(Placement::Fighter) => Slot::FighterTube(0),
        _ => return None,
    };
    let carrier = Fit {
        name: None,
        ship: target.ship.clone(),
        items: vec![FitItem {
            type_id,
            slot,
            quantity,
            state: State::Active,
            charge: charge.map(|type_id| Charge { type_id }),
            mutation: None,
            fighter_abilities: None,
            booster_side_effects: Default::default(),
            spool: None,
        }],
        character: Character {
            skills: target.character.skills.clone(),
            security_status: target.character.security_status,
        },
        environment: target.environment.clone(),
        incoming: Projection::default(),
    };
    Some(carrier)
}
