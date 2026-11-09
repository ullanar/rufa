use std::collections::HashMap;

use crate::eve;

pub mod category {
    pub const SHIP: i32 = 6;
    pub const MODULE: i32 = 7;
    pub const CHARGE: i32 = 8;
    pub const SKILL: i32 = 16;
    pub const DRONE: i32 = 18;
    pub const IMPLANT: i32 = 20;
    pub const SUBSYSTEM: i32 = 32;
    pub const STRUCTURE: i32 = 65;
    pub const STRUCTURE_MODULE: i32 = 66;
    pub const FIGHTER: i32 = 87;
}

pub mod group {
    pub const SHIELD_BOOSTER: i32 = 40;
    pub const REMOTE_SHIELD_BOOSTER: i32 = 41;
    pub const ARMOR_REPAIR_UNIT: i32 = 62;
    pub const HULL_REPAIR_UNIT: i32 = 63;
    pub const REMOTE_CAPACITOR_TRANSMITTER: i32 = 67;
    pub const REMOTE_ARMOR_REPAIRER: i32 = 325;
    pub const REMOTE_HULL_REPAIRER: i32 = 585;
    pub const ANCILLARY_SHIELD_BOOSTER: i32 = 1156;
    pub const ANCILLARY_ARMOR_REPAIRER: i32 = 1199;
    pub const ANCILLARY_REMOTE_SHIELD_BOOSTER: i32 = 1697;
    pub const ANCILLARY_REMOTE_ARMOR_REPAIRER: i32 = 1698;
    pub const MUTADAPTIVE_REMOTE_ARMOR_REPAIRER: i32 = 2018;
    pub const ECM: i32 = 201;
}

pub mod bomb {
    pub const EM: i32 = 27920;
    pub const THERMAL: i32 = 27916;
    pub const KINETIC: i32 = 27912;
    pub const EXPLOSIVE: i32 = 27918;
    pub const ALL: [i32; 4] = [EM, THERMAL, KINETIC, EXPLOSIVE];
}

pub const MARKET_ROOTS: [i32; 9] = [9, 955, 157, 11, 24, 477, 2202, 2203, 4];

#[derive(Debug, Clone)]
pub struct MarketNode {
    pub id: i32,
    pub name: String,
    pub parent: i32,
    pub children: Vec<i32>,
    pub types: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct HullGroup {
    pub id: i32,
    pub name: String,
    pub hulls: Vec<TypeRef>,
}

#[derive(Debug, Clone)]
pub struct TypeRef {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct AttrLabel {
    pub name: String,
    pub display_name: String,
    pub unit: String,
    pub published: bool,
    pub high_is_good: bool,
}

#[derive(Debug)]
pub struct Catalog {
    market: HashMap<i32, MarketNode>,
    hull_groups: Vec<HullGroup>,
    search: Vec<(String, i32)>,
    skills: Vec<TypeRef>,
    skill_groups: Vec<HullGroup>,
    attr_labels: HashMap<i32, AttrLabel>,
    group_names: HashMap<i32, String>,
    environments: Vec<TypeRef>,
    variations: HashMap<(i32, i32), Vec<TypeRef>>,
    charges_by_group: HashMap<i32, Vec<TypeRef>>,
    effects: HashMap<String, i32>,
}

const BEACON_GROUPS: [i32; 2] = [920, 1983];

impl Catalog {
    pub fn build(raw: &eve::Sde<'_>) -> Catalog {
        let mut market: HashMap<i32, MarketNode> = HashMap::new();
        for g in raw.market_groups().into_iter().flatten() {
            market.insert(
                g.id(),
                MarketNode {
                    id: g.id(),
                    name: g.name().to_string(),
                    parent: g.parent_group_id(),
                    children: Vec::new(),
                    types: Vec::new(),
                },
            );
        }
        let ids: Vec<i32> = market.keys().copied().collect();
        for id in ids {
            let parent = market[&id].parent;
            if let Some(p) = market.get_mut(&parent) {
                p.children.push(id);
            }
        }

        let mut groups: HashMap<i32, (String, i32)> = HashMap::new();
        for g in raw.groups().into_iter().flatten() {
            groups.insert(g.id(), (g.name().to_string(), g.category_id()));
        }

        let mut environments = Vec::new();
        let mut variations: HashMap<(i32, i32), Vec<TypeRef>> = HashMap::new();
        let mut charges_by_group: HashMap<i32, Vec<TypeRef>> = HashMap::new();
        for t in raw.types().into_iter().flatten() {
            if BEACON_GROUPS.contains(&t.group_id()) && (t.published() || t.group_id() == 1983) {
                environments.push(TypeRef {
                    id: t.id(),
                    name: t.name().to_string(),
                });
            }
            if t.published() && t.category_id() == category::CHARGE && t.market_group_id() != 0 {
                charges_by_group
                    .entry(t.group_id())
                    .or_default()
                    .push(TypeRef {
                        id: t.id(),
                        name: t.name().to_string(),
                    });
            }
            if t.published() && t.market_group_id() != 0 {
                variations
                    .entry((t.market_group_id(), t.group_id()))
                    .or_default()
                    .push(TypeRef {
                        id: t.id(),
                        name: t.name().to_string(),
                    });
            }
        }
        environments.sort_by(|a, b| a.name.cmp(&b.name));
        for list in variations.values_mut().chain(charges_by_group.values_mut()) {
            list.sort_by(|a, b| a.name.cmp(&b.name));
        }

        let mut hulls: HashMap<i32, Vec<TypeRef>> = HashMap::new();
        let mut skills_by_group: HashMap<i32, Vec<TypeRef>> = HashMap::new();
        let mut search = Vec::new();
        let mut skills = Vec::new();
        for t in raw.types().into_iter().flatten() {
            if !t.published() {
                continue;
            }
            let r = TypeRef {
                id: t.id(),
                name: t.name().to_string(),
            };
            match t.category_id() {
                category::SHIP => hulls.entry(t.group_id()).or_default().push(r.clone()),
                category::SKILL => {
                    skills.push(r.clone());
                    skills_by_group
                        .entry(t.group_id())
                        .or_default()
                        .push(r.clone());
                }
                _ => {}
            }
            if t.market_group_id() != 0 {
                if let Some(node) = market.get_mut(&t.market_group_id()) {
                    node.types.push(t.id());
                }
                if matches!(
                    t.category_id(),
                    category::SHIP
                        | category::MODULE
                        | category::CHARGE
                        | category::DRONE
                        | category::IMPLANT
                        | category::SUBSYSTEM
                        | category::STRUCTURE
                        | category::STRUCTURE_MODULE
                        | category::FIGHTER
                ) {
                    search.push((t.name().to_lowercase(), t.id()));
                }
            }
        }
        for node in market.values_mut() {
            node.children.sort();
        }
        let mut hull_groups: Vec<HullGroup> = hulls
            .into_iter()
            .map(|(id, mut list)| {
                list.sort_by(|a, b| a.name.cmp(&b.name));
                HullGroup {
                    id,
                    name: groups.get(&id).map(|g| g.0.clone()).unwrap_or_default(),
                    hulls: list,
                }
            })
            .collect();
        hull_groups.sort_by(|a, b| a.name.cmp(&b.name));
        search.sort();
        skills.sort_by(|a, b| a.name.cmp(&b.name));

        let mut skill_groups: Vec<HullGroup> = skills_by_group
            .into_iter()
            .map(|(id, mut list)| {
                list.sort_by(|a, b| a.name.cmp(&b.name));
                HullGroup {
                    id,
                    name: groups.get(&id).map(|g| g.0.clone()).unwrap_or_default(),
                    hulls: list,
                }
            })
            .collect();
        skill_groups.sort_by(|a, b| a.name.cmp(&b.name));

        let mut units: HashMap<i32, String> = HashMap::new();
        for u in raw.dogma_units().into_iter().flatten() {
            units.insert(u.id(), u.display_name().unwrap_or("").to_string());
        }
        let mut attr_labels = HashMap::new();
        for a in raw.dogma_attributes().into_iter().flatten() {
            attr_labels.insert(
                a.id(),
                AttrLabel {
                    name: a.name().to_string(),
                    display_name: a.display_name().unwrap_or("").to_string(),
                    unit: units.get(&a.unit_id()).cloned().unwrap_or_default(),
                    published: a.published(),
                    high_is_good: a.high_is_good(),
                },
            );
        }
        let group_names = groups
            .iter()
            .map(|(id, (name, _))| (*id, name.clone()))
            .collect();

        let mut catalog = Catalog {
            market,
            hull_groups,
            search,
            skills,
            skill_groups,
            attr_labels,
            group_names,
            environments,
            variations,
            charges_by_group,
            effects: raw
                .dogma_effects()
                .into_iter()
                .flatten()
                .map(|e| (e.name().to_string(), e.id()))
                .collect(),
        };
        catalog.sort_market_children();
        catalog
    }

    fn sort_market_children(&mut self) {
        let ids: Vec<i32> = self.market.keys().copied().collect();
        for id in ids {
            let mut children = self.market[&id].children.clone();
            children.sort_by(|a, b| self.market[a].name.cmp(&self.market[b].name));
            self.market.get_mut(&id).unwrap().children = children;
        }
    }

    pub fn market_node(&self, id: i32) -> Option<&MarketNode> {
        self.market.get(&id)
    }

    pub fn hull_groups(&self) -> &[HullGroup] {
        &self.hull_groups
    }

    pub fn skills(&self) -> &[TypeRef] {
        &self.skills
    }

    pub fn skill_groups(&self) -> &[HullGroup] {
        &self.skill_groups
    }

    pub fn environments(&self) -> &[TypeRef] {
        &self.environments
    }

    pub fn charges_in_group(&self, group_id: i32) -> &[TypeRef] {
        self.charges_by_group
            .get(&group_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn variations(&self, market_group_id: i32, group_id: i32) -> &[TypeRef] {
        self.variations
            .get(&(market_group_id, group_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn attr_label(&self, attribute_id: i32) -> Option<&AttrLabel> {
        self.attr_labels.get(&attribute_id)
    }

    pub fn effect_id(&self, name: &str) -> Option<i32> {
        self.effects.get(name).copied()
    }

    pub fn group_name(&self, group_id: i32) -> &str {
        self.group_names
            .get(&group_id)
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<i32> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        self.search
            .iter()
            .filter(|(name, _)| words.iter().all(|w| name.contains(w.as_str())))
            .map(|(_, id)| *id)
            .take(limit)
            .collect()
    }
}
