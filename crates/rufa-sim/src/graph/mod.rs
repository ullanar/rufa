pub mod ammo;
pub mod apply;
pub mod cycles;
pub mod damage;
pub mod dealers;
pub mod dmg;
pub mod effects;
pub mod ewar;
pub mod model;
pub mod motion;
pub mod plot;
pub mod regen;
pub mod reps;
pub mod sample;
pub mod tackle;
pub mod target;
pub mod timeline;

use rufa_data::{Catalog, Info};

use crate::ItemResult;
use crate::attrs::AttrIds;

pub use effects::EffectIds;
pub use model::*;

/// What every graph calculation reads: game data, attribute and effect ids.
pub struct Ctx<'a, I: Info> {
    pub info: &'a I,
    pub ids: &'a AttrIds,
    pub fx: &'a EffectIds,
    pub catalog: &'a Catalog,
}

/// The radius of an item: the attribute when the engine has it, else the
/// type's own.
pub fn radius_of(info: &impl Info, type_id: i32, result: &ItemResult) -> f64 {
    if let Some(id) = info.attribute_name_to_id("radius")
        && let Some(a) = result.attributes.get(&id)
        && a.value > 0.0
    {
        return a.value;
    }
    info.get_type(type_id)
        .and_then(|t| t.radius())
        .map(f64::from)
        .unwrap_or(0.0)
}
