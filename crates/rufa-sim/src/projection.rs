use rufa_data::Info;

use crate::attrs::AttrIds;
use crate::place::{Placement, placement};
use crate::{Character, Charge, Fit, FitItem, Options, Projection, Slot, State};

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

pub fn incoming(
    info: &impl Info,
    ids: &AttrIds,
    target: &Fit,
    sources: &[Projected],
) -> Projection {
    let mut total = Projection::default();
    for source in sources {
        let part = match source {
            Projected::Beacon(type_id) => esf_dogma_engine::beacon(info, *type_id),
            Projected::Module {
                type_id,
                charge,
                quantity,
            } => module_outgoing(info, ids, target, *type_id, *charge, *quantity),
            Projected::Fit { fit, .. } => {
                let options = Options {
                    sources: false,
                    validate: false,
                };
                esf_dogma_engine::calculate(info, fit, &options).outgoing
            }
        };
        total.buffs.extend(part.buffs);
        total.effects.extend(part.effects);
    }
    total
}

fn module_outgoing(
    info: &impl Info,
    ids: &AttrIds,
    target: &Fit,
    type_id: i32,
    charge: Option<i32>,
    quantity: u32,
) -> Projection {
    let slot = match placement(info, ids, type_id) {
        Some(Placement::Module(rack)) => rack.slot(0),
        Some(Placement::Drone) => Slot::DroneBay,
        Some(Placement::Fighter) => Slot::FighterTube(0),
        _ => return Projection::default(),
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
    let options = Options {
        sources: false,
        validate: false,
    };
    esf_dogma_engine::calculate(info, &carrier, &options).outgoing
}
