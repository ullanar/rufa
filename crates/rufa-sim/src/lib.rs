pub mod attrs;
pub mod cap;
pub mod document;
pub mod edit;
pub mod graph;
pub mod layout;
pub mod place;
pub mod projection;
pub mod stats;

pub use esf_dogma_engine::{
    AttributeValue, Calculation, Character, Charge, DamageProfile, Environment, Fit, FitItem,
    ItemResult, Mutation, Options, Projection, Resource, Rule, Security, Ship, Slot, SlotKind,
    State, Target, Violation,
};
pub use esf_format as format;
use rufa_data::{Info, Sde};

pub fn calculate(info: &impl Info, fit: &Fit) -> Calculation {
    let options = Options {
        sources: true,
        validate: true,
    };
    esf_dogma_engine::calculate(info, fit, &options)
}

pub fn fit_with_skill_level(sde: &Sde, mut fit: Fit, level: u8) -> Fit {
    const SKILL_CATEGORY: i32 = 16;
    for ty in sde.types() {
        if ty.category_id() == SKILL_CATEGORY {
            fit.character.skills.insert(ty.id(), level);
        }
    }
    fit
}
