use std::collections::BTreeMap;

use rufa_data::Info;

use crate::attrs::AttrIds;
use crate::layout::SlotLayout;
use crate::place::{Placement, Rack, initial_state, new_item, placement};
use crate::{Character, Charge, Environment, Fit, Projection, Ship, Slot, State};

#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    UnknownType(i32),
    RackFull(Rack),
    NotAModule(usize),
    NotACharge(i32),
    NoSuchItem(usize),
    DoesNotFit(crate::Rule),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::UnknownType(id) => write!(f, "unknown type {id}"),
            EditError::RackFull(rack) => write!(f, "no free {} slot", rack.label()),
            EditError::NotAModule(i) => write!(f, "item {i} is not a module"),
            EditError::NotACharge(id) => write!(f, "type {id} is not a charge"),
            EditError::NoSuchItem(i) => write!(f, "no item {i}"),
            EditError::DoesNotFit(rule) => write!(f, "does not fit: {}", rule_text(rule)),
        }
    }
}

pub fn new_fit(ship: i32, skills: BTreeMap<i32, u8>) -> Fit {
    Fit {
        name: None,
        ship: Ship {
            type_id: ship,
            mode: None,
        },
        items: Vec::new(),
        character: Character {
            skills,
            security_status: 0.0,
        },
        environment: Environment::default(),
        incoming: Projection::default(),
    }
}

pub fn free_index(fit: &Fit, rack: Rack, count: u8) -> Option<u8> {
    (0..count).find(|i| !fit.items.iter().any(|it| it.slot == rack.slot(*i)))
}

pub fn add(
    fit: &mut Fit,
    info: &impl Info,
    ids: &AttrIds,
    layout: &SlotLayout,
    type_id: i32,
) -> Result<usize, EditError> {
    let place = placement(info, ids, type_id).ok_or(EditError::UnknownType(type_id))?;
    let state = initial_state(place);
    let slot = match place {
        Placement::Module(rack) => {
            let index =
                free_index(fit, rack, layout.count(rack)).ok_or(EditError::RackFull(rack))?;
            rack.slot(index)
        }
        Placement::Drone => {
            if let Some(i) = fit
                .items
                .iter()
                .position(|it| it.slot == Slot::DroneBay && it.type_id == type_id)
            {
                fit.items[i].quantity += 1;
                return Ok(i);
            }
            Slot::DroneBay
        }
        Placement::Fighter => {
            let used: Vec<u8> = fit
                .items
                .iter()
                .filter_map(|it| match it.slot {
                    Slot::FighterTube(i) => Some(i),
                    _ => None,
                })
                .collect();
            let index = (0..u8::MAX).find(|i| !used.contains(i)).unwrap_or(0);
            Slot::FighterTube(index)
        }
        Placement::Implant(slot) => {
            fit.items.retain(|it| it.slot != Slot::Implant(slot));
            Slot::Implant(slot)
        }
        Placement::Booster(slot) => {
            fit.items.retain(|it| it.slot != Slot::Booster(slot));
            Slot::Booster(slot)
        }
        Placement::Charge | Placement::Cargo => {
            if let Some(i) = fit
                .items
                .iter()
                .position(|it| it.slot == Slot::Cargo && it.type_id == type_id)
            {
                fit.items[i].quantity += 1;
                return Ok(i);
            }
            Slot::Cargo
        }
    };
    fit.items.push(new_item(type_id, slot, state));
    Ok(fit.items.len() - 1)
}

/// Put a type in the cargo hold, whatever it is. A spare booster or a second
/// set of implants rides along without touching a single number.
pub fn add_to_cargo(fit: &mut Fit, type_id: i32) -> usize {
    if let Some(i) = fit
        .items
        .iter()
        .position(|it| it.slot == Slot::Cargo && it.type_id == type_id)
    {
        fit.items[i].quantity += 1;
        return i;
    }
    fit.items
        .push(new_item(type_id, Slot::Cargo, State::Offline));
    fit.items.len() - 1
}

/// Take an item out of its slot and drop it in the hold, keeping the count.
pub fn move_to_cargo(fit: &mut Fit, index: usize) -> Result<(), EditError> {
    let item = fit.items.get(index).ok_or(EditError::NoSuchItem(index))?;
    if item.slot == Slot::Cargo {
        return Ok(());
    }
    let (type_id, quantity) = (item.type_id, item.quantity.max(1));
    fit.items.remove(index);
    let at = add_to_cargo(fit, type_id);
    fit.items[at].quantity = fit.items[at].quantity.max(1) + quantity - 1;
    Ok(())
}

pub fn remove(fit: &mut Fit, index: usize) -> Result<(), EditError> {
    if index >= fit.items.len() {
        return Err(EditError::NoSuchItem(index));
    }
    fit.items.remove(index);
    Ok(())
}

pub fn set_state(fit: &mut Fit, index: usize, state: State) -> Result<(), EditError> {
    let item = fit
        .items
        .get_mut(index)
        .ok_or(EditError::NoSuchItem(index))?;
    item.state = state;
    Ok(())
}

pub fn cycle_state(current: State, max: State) -> State {
    match current {
        State::Offline => State::Online.min(max),
        State::Online => {
            if max >= State::Active {
                State::Active
            } else {
                State::Offline
            }
        }
        State::Active => State::Online,
        State::Overload => State::Active,
    }
}

pub fn toggle_overload(current: State, max: State) -> State {
    match current {
        State::Overload => State::Active,
        _ if max >= State::Overload => State::Overload,
        other => other,
    }
}

pub fn toggle_offline(current: State) -> State {
    match current {
        State::Offline => State::Online,
        _ => State::Offline,
    }
}

pub fn set_charge(fit: &mut Fit, index: usize, charge: Option<i32>) -> Result<(), EditError> {
    let item = fit
        .items
        .get_mut(index)
        .ok_or(EditError::NoSuchItem(index))?;
    if Rack::of_slot(item.slot).is_none() {
        return Err(EditError::NotAModule(index));
    }
    item.charge = charge.map(|type_id| Charge { type_id });
    Ok(())
}

pub fn set_quantity(fit: &mut Fit, index: usize, quantity: u32) -> Result<(), EditError> {
    let item = fit
        .items
        .get_mut(index)
        .ok_or(EditError::NoSuchItem(index))?;
    if quantity == 0 {
        fit.items.remove(index);
    } else {
        item.quantity = quantity;
    }
    Ok(())
}

pub fn all_skills(info_skills: impl Iterator<Item = i32>, level: u8) -> BTreeMap<i32, u8> {
    info_skills.map(|id| (id, level)).collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MutationRange {
    pub attribute_id: i32,
    pub base: f64,
    pub min: f64,
    pub max: f64,
}

pub fn mutaplasmids_for(sde: &rufa_data::Sde, base_type: i32) -> Vec<i32> {
    sde.mutaplasmids_of(base_type).to_vec()
}

fn base_attribute(info: &impl Info, type_id: i32, attribute_id: i32) -> Option<f64> {
    info.get_dogma_attributes(type_id)?
        .iter()
        .find(|a| a.attribute_id() == attribute_id)
        .map(|a| a.value() as f64)
}

pub fn mutation_ranges(
    sde: &rufa_data::Sde,
    info: &impl Info,
    mutaplasmid: i32,
    base_type: i32,
) -> Vec<MutationRange> {
    let Some(m) = sde.get_mutaplasmid(mutaplasmid) else {
        return Vec::new();
    };
    m.attributes()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let base = base_attribute(info, base_type, a.attribute_id())?;
            let (lo, hi) = (base * a.min() as f64, base * a.max() as f64);
            Some(MutationRange {
                attribute_id: a.attribute_id(),
                base,
                min: lo.min(hi),
                max: lo.max(hi),
            })
        })
        .collect()
}

pub fn mutate(
    fit: &mut Fit,
    sde: &rufa_data::Sde,
    info: &impl Info,
    index: usize,
    mutaplasmid: i32,
) -> Result<(), EditError> {
    let item = fit.items.get(index).ok_or(EditError::NoSuchItem(index))?;
    let base = item
        .mutation
        .as_ref()
        .map(|m| m.base)
        .unwrap_or(item.type_id);
    let m = sde
        .get_mutaplasmid(mutaplasmid)
        .ok_or(EditError::UnknownType(mutaplasmid))?;
    let resulting = m
        .mappings()
        .into_iter()
        .flatten()
        .find(|map| {
            map.applicable_type_ids()
                .into_iter()
                .flatten()
                .any(|t| t == base)
        })
        .map(|map| map.resulting_type_id())
        .ok_or(EditError::UnknownType(mutaplasmid))?;
    let attributes = mutation_ranges(sde, info, mutaplasmid, base)
        .into_iter()
        .map(|r| (r.attribute_id, (r.min + r.max) / 2.0))
        .collect();
    let item = &mut fit.items[index];
    item.type_id = resulting;
    item.mutation = Some(crate::Mutation { base, attributes });
    Ok(())
}

pub fn set_mutation_value(
    fit: &mut Fit,
    index: usize,
    attribute_id: i32,
    value: f64,
) -> Result<(), EditError> {
    let item = fit
        .items
        .get_mut(index)
        .ok_or(EditError::NoSuchItem(index))?;
    let mutation = item.mutation.as_mut().ok_or(EditError::NotAModule(index))?;
    mutation.attributes.insert(attribute_id, value);
    Ok(())
}

pub fn revert_mutation(fit: &mut Fit, index: usize) -> Result<(), EditError> {
    let item = fit
        .items
        .get_mut(index)
        .ok_or(EditError::NoSuchItem(index))?;
    if let Some(mutation) = item.mutation.take() {
        item.type_id = mutation.base;
    }
    Ok(())
}

pub fn side_effects(info: &impl Info, type_id: i32) -> Vec<(i32, String)> {
    let Some(effects) = info.get_dogma_effects(type_id) else {
        return Vec::new();
    };
    effects
        .iter()
        .filter_map(|e| {
            let effect = info.get_dogma_effect(e.effect_id())?;
            if effect.fitting_usage_chance_attribute_id() == 0 {
                return None;
            }
            let name = effect
                .display_name()
                .filter(|n| !n.is_empty())
                .unwrap_or(effect.name());
            Some((effect.id(), name.to_string()))
        })
        .collect()
}

pub fn toggle_side_effect(fit: &mut Fit, index: usize, effect_id: i32) -> Result<(), EditError> {
    let item = fit
        .items
        .get_mut(index)
        .ok_or(EditError::NoSuchItem(index))?;
    if !item.booster_side_effects.remove(&effect_id) {
        item.booster_side_effects.insert(effect_id);
    }
    Ok(())
}

pub fn fighter_abilities(info: &impl Info, type_id: i32) -> Vec<(i32, String, bool)> {
    let Some(effects) = info.get_dogma_effects(type_id) else {
        return Vec::new();
    };
    effects
        .iter()
        .filter_map(|e| {
            let effect = info.get_dogma_effect(e.effect_id())?;
            if effect.effect_category() == rufa_data::eve::EffectCategory::Passive {
                return None;
            }
            let name = effect
                .display_name()
                .filter(|n| !n.is_empty())
                .unwrap_or(effect.name());
            Some((effect.id(), name.to_string(), e.is_default()))
        })
        .collect()
}

pub fn ability_on(info: &impl Info, fit: &Fit, index: usize, effect_id: i32) -> bool {
    let Some(item) = fit.items.get(index) else {
        return false;
    };
    match &item.fighter_abilities {
        Some(set) => set.contains(&effect_id),
        None => fighter_abilities(info, item.type_id)
            .iter()
            .any(|(id, _, default)| *id == effect_id && *default),
    }
}

pub fn toggle_ability(
    fit: &mut Fit,
    info: &impl Info,
    index: usize,
    effect_id: i32,
) -> Result<(), EditError> {
    let on = ability_on(info, fit, index, effect_id);
    let item = fit.items.get(index).ok_or(EditError::NoSuchItem(index))?;
    let mut set = match &item.fighter_abilities {
        Some(set) => set.clone(),
        None => fighter_abilities(info, item.type_id)
            .into_iter()
            .filter(|(_, _, default)| *default)
            .map(|(id, _, _)| id)
            .collect(),
    };
    if on {
        set.remove(&effect_id);
    } else {
        set.insert(effect_id);
    }
    fit.items[index].fighter_abilities = Some(set);
    Ok(())
}

pub fn rule_text(rule: &crate::Rule) -> String {
    use crate::Rule;
    match rule {
        Rule::Resource {
            resource,
            used,
            available,
        } => format!("{resource:?} {used:.1} of {available:.1}"),
        Rule::Slots {
            slot,
            used,
            available,
        } => format!("{slot:?} slots {used} of {available}"),
        Rule::Skill {
            required, level, ..
        } => format!("skill level {required} needed, has {level}"),
        Rule::MaxGroup { used, allowed, .. } => {
            format!("too many of this group: {used}, allowed {allowed}")
        }
        Rule::MaxType { used, allowed, .. } => {
            format!("too many of this type: {used}, allowed {allowed}")
        }
        Rule::WrongSlot { expected } => format!("belongs in a {expected:?} slot"),
        Rule::RigSize { .. } => "rig size does not match the hull".to_string(),
        Rule::ShipRestricted => "this hull cannot fit it".to_string(),
        Rule::CapitalItem => "capital module on a subcapital hull".to_string(),
        Rule::StructureItem => "structure module on a ship".to_string(),
        Rule::ShipItem => "ship module on a structure".to_string(),
        Rule::ChargeGroup => "charge type does not match the module".to_string(),
        Rule::ChargeSize { .. } => "charge size does not match the module".to_string(),
        other => format!("{other:?}"),
    }
}

fn structural(rule: &crate::Rule) -> bool {
    !matches!(
        rule,
        crate::Rule::Resource { .. } | crate::Rule::Skill { .. }
    )
}

pub fn add_checked(
    fit: &mut Fit,
    info: &impl Info,
    ids: &AttrIds,
    layout: &SlotLayout,
    type_id: i32,
) -> Result<usize, EditError> {
    let options = crate::Options {
        sources: false,
        validate: true,
    };
    let before = esf_dogma_engine::calculate(info, fit, &options)
        .violations
        .unwrap_or_default();
    let mut trial = fit.clone();
    let index = add(&mut trial, info, ids, layout, type_id)?;
    let after = esf_dogma_engine::calculate(info, &trial, &options)
        .violations
        .unwrap_or_default();
    let blocking = after.into_iter().find(|v| {
        structural(&v.rule)
            && match v.target {
                crate::Target::Item { index: i } | crate::Target::Charge { index: i } => i == index,
                crate::Target::Ship => !before.iter().any(|b| b.rule == v.rule),
            }
    });
    match blocking {
        Some(v) => Err(EditError::DoesNotFit(v.rule)),
        None => {
            *fit = trial;
            Ok(index)
        }
    }
}
