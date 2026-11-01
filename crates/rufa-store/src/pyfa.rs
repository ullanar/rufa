use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use rufa_data::Info;
use rufa_sim::attrs::AttrIds;
use rufa_sim::place::{Placement, Rack, placement};
use rufa_sim::{
    Character, Charge, Environment, Fit, FitItem, Mutation, Projection, Ship, Slot, State,
};

use crate::sqlite::{Database, SqliteError, Table, Value};

pub const DB_NAME: &str = "saveddata.db";

#[derive(Debug, Clone)]
pub struct Imported {
    pub pyfa_id: i64,
    pub fit: Fit,
    pub unknown_types: usize,
}

#[derive(Debug)]
pub struct Report {
    pub fits: Vec<Imported>,
    pub skipped: usize,
}

pub fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = std::env::var_os("RUFA_PYFA_DB") {
        out.push(PathBuf::from(p));
    }
    let home = rufa_data::home_dir();
    out.push(home.join(".pyfa").join(DB_NAME));
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        out.push(PathBuf::from(profile).join(".pyfa").join(DB_NAME));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(appdata).join("pyfa").join(DB_NAME));
    }
    out.push(
        home.join("Library")
            .join("Application Support")
            .join("pyfa")
            .join(DB_NAME),
    );
    out.dedup();
    out
}

pub fn locate() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|p| p.is_file())
}

fn rows_by<'a>(table: &'a Table, column: &str) -> HashMap<i64, Vec<&'a [Value]>> {
    let mut out: HashMap<i64, Vec<&[Value]>> = HashMap::new();
    for row in &table.rows {
        if let Some(key) = table.get(row, column).as_i64() {
            out.entry(key).or_default().push(row);
        }
    }
    out
}

fn state_of(value: i64) -> State {
    match value {
        -1 => State::Offline,
        0 => State::Online,
        1 => State::Active,
        _ => State::Overload,
    }
}

fn item(type_id: i32, slot: Slot, state: State) -> FitItem {
    FitItem {
        type_id,
        slot,
        quantity: 1,
        state,
        charge: None,
        mutation: None,
        fighter_abilities: None,
        booster_side_effects: BTreeSet::new(),
        spool: None,
    }
}

pub fn read(path: &Path, info: &impl Info, ids: &AttrIds) -> Result<Report, SqliteError> {
    let db = Database::open(path)?;
    let fits = db.table("fits")?;
    let modules = db.table("modules")?;
    let drones = db.table("drones")?;
    let cargo = db.table("cargo")?;
    let fit_implants = db.table("fitImplants")?;
    let implants = db.table("implants")?;
    let optional = |name: &str| db.has_table(name).then(|| db.table(name)).transpose();
    let fighters = optional("fighters")?;
    let abilities = optional("fightersAbilities")?;
    let boosters = optional("boosters")?;
    let side_effects = optional("boosterSideEffects")?;
    let mutators = optional("mutators")?;

    let modules_by_fit = rows_by(&modules, "fitID");
    let drones_by_fit = rows_by(&drones, "fitID");
    let cargo_by_fit = rows_by(&cargo, "fitID");
    let implants_by_fit = rows_by(&fit_implants, "fitID");
    let implant_rows = rows_by(&implants, "ID");
    let fighters_by_fit = fighters
        .as_ref()
        .map(|t| rows_by(t, "fitID"))
        .unwrap_or_default();
    let abilities_by_fighter = abilities
        .as_ref()
        .map(|t| rows_by(t, "groupID"))
        .unwrap_or_default();
    let boosters_by_fit = boosters
        .as_ref()
        .map(|t| rows_by(t, "fitID"))
        .unwrap_or_default();
    let effects_by_booster = side_effects
        .as_ref()
        .map(|t| rows_by(t, "boosterID"))
        .unwrap_or_default();
    let mutators_by_module = mutators
        .as_ref()
        .map(|t| rows_by(t, "moduleID"))
        .unwrap_or_default();

    let known = |type_id: i64| -> Option<i32> {
        let id = i32::try_from(type_id).ok()?;
        info.get_type(id).map(|_| id)
    };

    let mut out = Vec::new();
    let mut skipped = 0;
    for row in &fits.rows {
        let Some(pyfa_id) = fits.get(row, "ID").as_i64() else {
            skipped += 1;
            continue;
        };
        let Some(ship) = fits.get(row, "shipID").as_i64().and_then(known) else {
            skipped += 1;
            continue;
        };
        let name = fits
            .get(row, "name")
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        let mode = fits.get(row, "modeID").as_i64().and_then(known);
        let mut fit = Fit {
            name: (!name.is_empty()).then_some(name),
            ship: Ship {
                type_id: ship,
                mode,
            },
            items: Vec::new(),
            character: Character::default(),
            environment: Environment::default(),
            incoming: Projection::default(),
        };
        let mut unknown = 0;

        let mut module_rows: Vec<&[Value]> =
            modules_by_fit.get(&pyfa_id).cloned().unwrap_or_default();
        module_rows.sort_by_key(|r| modules.get(r, "position").as_i64().unwrap_or(0));
        let mut next_index: HashMap<Rack, u8> = HashMap::new();
        for r in module_rows {
            if modules.get(r, "projected").as_i64().unwrap_or(0) != 0 {
                continue;
            }
            if !matches!(modules.get(r, "dummySlot"), Value::Null) {
                continue;
            }
            let Some(type_raw) = modules.get(r, "itemID").as_i64() else {
                continue;
            };
            let Some(type_id) = known(type_raw) else {
                unknown += 1;
                continue;
            };
            let Some(Placement::Module(rack)) = placement(info, ids, type_id) else {
                unknown += 1;
                continue;
            };
            let index = next_index.entry(rack).or_insert(0);
            let slot = rack.slot(*index);
            *index += 1;
            let state = state_of(modules.get(r, "state").as_i64().unwrap_or(0));
            let mut it = item(type_id, slot, state);
            if let Some(charge) = modules.get(r, "chargeID").as_i64().and_then(known) {
                it.charge = Some(Charge { type_id: charge });
            }
            if let (Some(base), Some(module_id)) = (
                modules.get(r, "baseItemID").as_i64().and_then(known),
                modules.get(r, "ID").as_i64(),
            ) {
                let mut attributes = BTreeMap::new();
                if let Some(mutators) = &mutators {
                    for m in mutators_by_module
                        .get(&module_id)
                        .cloned()
                        .unwrap_or_default()
                    {
                        if let (Some(attr), Some(value)) = (
                            mutators.get(m, "attrID").as_i64(),
                            mutators.get(m, "value").as_f64(),
                        ) {
                            attributes.insert(attr as i32, value);
                        }
                    }
                }
                it.mutation = Some(Mutation { base, attributes });
            }
            fit.items.push(it);
        }

        for r in drones_by_fit.get(&pyfa_id).cloned().unwrap_or_default() {
            if drones.get(r, "projected").as_i64().unwrap_or(0) != 0 {
                continue;
            }
            let Some(type_id) = drones.get(r, "itemID").as_i64().and_then(known) else {
                unknown += 1;
                continue;
            };
            let amount = drones.get(r, "amount").as_i64().unwrap_or(1).max(0) as u32;
            let active =
                (drones.get(r, "amountActive").as_i64().unwrap_or(0).max(0) as u32).min(amount);
            if active > 0 {
                let mut it = item(type_id, Slot::DroneBay, State::Active);
                it.quantity = active;
                fit.items.push(it);
            }
            if amount > active {
                let mut it = item(type_id, Slot::DroneBay, State::Offline);
                it.quantity = amount - active;
                fit.items.push(it);
            }
        }

        if let Some(fighters) = &fighters {
            let mut tube = 0u8;
            for r in fighters_by_fit.get(&pyfa_id).cloned().unwrap_or_default() {
                if fighters.get(r, "projected").as_i64().unwrap_or(0) != 0 {
                    continue;
                }
                let Some(type_id) = fighters.get(r, "itemID").as_i64().and_then(known) else {
                    unknown += 1;
                    continue;
                };
                let active = fighters.get(r, "active").as_i64().unwrap_or(1) != 0;
                let mut it = item(
                    type_id,
                    Slot::FighterTube(tube),
                    if active {
                        State::Active
                    } else {
                        State::Offline
                    },
                );
                tube = tube.saturating_add(1);
                it.quantity = fighters.get(r, "amount").as_i64().unwrap_or(1).max(1) as u32;
                if let (Some(abilities), Some(group_id)) =
                    (&abilities, fighters.get(r, "groupID").as_i64())
                {
                    let set: BTreeSet<i32> = abilities_by_fighter
                        .get(&group_id)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|a| abilities.get(a, "active").as_i64().unwrap_or(0) != 0)
                        .filter_map(|a| abilities.get(a, "effectID").as_i64().map(|e| e as i32))
                        .collect();
                    if !set.is_empty() {
                        it.fighter_abilities = Some(set);
                    }
                }
                fit.items.push(it);
            }
        }

        for r in cargo_by_fit.get(&pyfa_id).cloned().unwrap_or_default() {
            let Some(type_id) = cargo.get(r, "itemID").as_i64().and_then(known) else {
                unknown += 1;
                continue;
            };
            let mut it = item(type_id, Slot::Cargo, State::Offline);
            it.quantity = cargo.get(r, "amount").as_i64().unwrap_or(1).max(1) as u32;
            fit.items.push(it);
        }

        for link in implants_by_fit.get(&pyfa_id).cloned().unwrap_or_default() {
            let Some(implant_id) = fit_implants.get(link, "implantID").as_i64() else {
                continue;
            };
            for r in implant_rows.get(&implant_id).cloned().unwrap_or_default() {
                let Some(type_id) = implants.get(r, "itemID").as_i64().and_then(known) else {
                    unknown += 1;
                    continue;
                };
                let Some(Placement::Implant(slot)) = placement(info, ids, type_id) else {
                    continue;
                };
                let active = implants.get(r, "active").as_i64().unwrap_or(1) != 0;
                fit.items.push(item(
                    type_id,
                    Slot::Implant(slot),
                    if active {
                        State::Online
                    } else {
                        State::Offline
                    },
                ));
            }
        }

        if let Some(boosters) = &boosters {
            for r in boosters_by_fit.get(&pyfa_id).cloned().unwrap_or_default() {
                let Some(type_id) = boosters.get(r, "itemID").as_i64().and_then(known) else {
                    unknown += 1;
                    continue;
                };
                let Some(Placement::Booster(slot)) = placement(info, ids, type_id) else {
                    continue;
                };
                let active = boosters.get(r, "active").as_i64().unwrap_or(1) != 0;
                let mut it = item(
                    type_id,
                    Slot::Booster(slot),
                    if active {
                        State::Online
                    } else {
                        State::Offline
                    },
                );
                if let (Some(effects), Some(booster_id)) =
                    (&side_effects, boosters.get(r, "ID").as_i64())
                {
                    it.booster_side_effects = effects_by_booster
                        .get(&booster_id)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|e| effects.get(e, "active").as_i64().unwrap_or(0) != 0)
                        .filter_map(|e| effects.get(e, "effectID").as_i64().map(|v| v as i32))
                        .collect();
                }
                fit.items.push(it);
            }
        }

        out.push(Imported {
            pyfa_id,
            fit,
            unknown_types: unknown,
        });
    }
    Ok(Report { fits: out, skipped })
}
