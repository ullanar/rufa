//! Repair the fit hands out, and the share of its own repair the capacitor pays for.

use rufa_data::Info;
use rufa_data::catalog::group;

use crate::attrs::AttrIds;
use crate::place::Rack;
use crate::{Calculation, Fit, ItemResult, Slot, State};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Remote {
    pub shield: f64,
    pub armor: f64,
    pub armor_spooled: f64,
    pub hull: f64,
    pub capacitor: f64,
    pub range: f64,
    pub drain: f64,
}

impl Remote {
    pub fn plus(self, other: Remote) -> Remote {
        let range = match (self.range, other.range) {
            (0.0, r) | (r, 0.0) => r,
            (a, b) => a.min(b),
        };
        Remote {
            shield: self.shield + other.shield,
            armor: self.armor + other.armor,
            armor_spooled: self.armor_spooled + other.armor_spooled,
            hull: self.hull + other.hull,
            capacitor: self.capacitor + other.capacitor,
            range,
            drain: self.drain + other.drain,
        }
    }

    pub fn hp(self) -> f64 {
        self.shield + self.armor + self.hull
    }

    pub fn any(self) -> bool {
        self.hp() > 0.0 || self.capacitor > 0.0
    }

    pub fn spools(self) -> bool {
        self.armor_spooled > self.armor + 1e-9
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sustained {
    pub shield: f64,
    pub armor: f64,
    pub hull: f64,
    pub capped: bool,
}

fn attr(result: &ItemResult, id: i32) -> f64 {
    result.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn seconds(result: &ItemResult, ids: &AttrIds) -> f64 {
    attr(result, ids.duration) / 1000.0
}

fn group_of(info: &impl Info, type_id: i32) -> i32 {
    info.get_type(type_id).map(|t| t.group_id()).unwrap_or(0)
}

pub fn outgoing(info: &impl Info, ids: &AttrIds, fit: &Fit, calc: &Calculation) -> Remote {
    let mut out = Remote::default();
    let mut range = f64::INFINITY;
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active {
            continue;
        }
        let cycle = seconds(result, ids);
        if cycle <= 0.0 {
            continue;
        }

        if item.slot == Slot::DroneBay {
            let count = f64::from(item.quantity);
            let armor = attr(result, ids.armor_damage_amount) * count / cycle;
            out.shield += attr(result, ids.shield_bonus) * count / cycle;
            out.armor += armor;
            out.armor_spooled += armor;
            out.hull += attr(result, ids.structure_damage_amount) * count / cycle;
            continue;
        }

        if Rack::of_slot(item.slot).is_none() {
            continue;
        }
        let kind = group_of(info, item.type_id);
        let amount = match kind {
            group::REMOTE_SHIELD_BOOSTER | group::ANCILLARY_REMOTE_SHIELD_BOOSTER => {
                out.shield += attr(result, ids.shield_bonus) / cycle;
                attr(result, ids.shield_bonus)
            }
            group::REMOTE_ARMOR_REPAIRER
            | group::ANCILLARY_REMOTE_ARMOR_REPAIRER
            | group::MUTADAPTIVE_REMOTE_ARMOR_REPAIRER => {
                let charged =
                    kind == group::ANCILLARY_REMOTE_ARMOR_REPAIRER && item.charge.is_some();
                let multiplier = if charged {
                    attr(result, ids.charged_armor_damage_multiplier).max(1.0)
                } else {
                    1.0
                };
                let armor = attr(result, ids.armor_damage_amount) * multiplier / cycle;
                let spool = 1.0 + attr(result, ids.repair_multiplier_bonus_max);
                out.armor += armor;
                out.armor_spooled += armor * spool;
                armor * cycle
            }
            group::REMOTE_HULL_REPAIRER => {
                out.hull += attr(result, ids.structure_damage_amount) / cycle;
                attr(result, ids.structure_damage_amount)
            }
            group::REMOTE_CAPACITOR_TRANSMITTER => {
                out.capacitor += attr(result, ids.power_transfer_amount) / cycle;
                attr(result, ids.power_transfer_amount)
            }
            _ => continue,
        };
        if amount <= 0.0 {
            continue;
        }
        out.drain += attr(result, ids.capacitor_need) / cycle;
        let reach = attr(result, ids.max_range);
        if reach > 0.0 {
            range = range.min(reach);
        }
    }
    out.range = if range.is_finite() { range } else { 0.0 };
    out
}

struct Repairer {
    shield: f64,
    armor: f64,
    hull: f64,
    cap_per_second: f64,
}

/// The repair the capacitor keeps running once the burst reserve is gone.
pub fn sustained(info: &impl Info, ids: &AttrIds, fit: &Fit, calc: &Calculation) -> Sustained {
    let ship = &calc.ship;
    let burst = Sustained {
        shield: attr(ship, ids.shield_boost_rate),
        armor: attr(ship, ids.armor_repair_rate),
        hull: attr(ship, ids.hull_repair_rate),
        capped: false,
    };
    let stable = attr(ship, ids.capacitor_stable_pct) > 0.0;
    if stable {
        return burst;
    }

    let mut repairers: Vec<Repairer> = Vec::new();
    let mut other_load = attr(ship, ids.capacitor_peak_load);
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active || Rack::of_slot(item.slot).is_none() {
            continue;
        }
        let cycle = seconds(result, ids);
        let cap_per_second = if cycle > 0.0 {
            attr(result, ids.capacitor_need) / cycle
        } else {
            0.0
        };
        if cycle <= 0.0 || cap_per_second <= 0.0 {
            continue;
        }
        let kind = group_of(info, item.type_id);
        let (shield, armor, hull) = match kind {
            group::SHIELD_BOOSTER | group::ANCILLARY_SHIELD_BOOSTER => {
                (attr(result, ids.shield_bonus) / cycle, 0.0, 0.0)
            }
            group::ARMOR_REPAIR_UNIT | group::ANCILLARY_ARMOR_REPAIRER => {
                let charged = kind == group::ANCILLARY_ARMOR_REPAIRER && item.charge.is_some();
                let multiplier = if charged {
                    attr(result, ids.charged_armor_damage_multiplier).max(1.0)
                } else {
                    1.0
                };
                (
                    0.0,
                    attr(result, ids.armor_damage_amount) * multiplier / cycle,
                    0.0,
                )
            }
            group::HULL_REPAIR_UNIT => {
                (0.0, 0.0, attr(result, ids.structure_damage_amount) / cycle)
            }
            _ => continue,
        };
        if shield + armor + hull <= 0.0 {
            continue;
        }
        other_load -= cap_per_second;
        repairers.push(Repairer {
            shield,
            armor,
            hull,
            cap_per_second,
        });
    }
    if repairers.is_empty() {
        return burst;
    }

    repairers.sort_by(|a, b| {
        let left = (a.shield + a.armor + a.hull) / a.cap_per_second;
        let right = (b.shield + b.armor + b.hull) / b.cap_per_second;
        right.total_cmp(&left)
    });

    let recharge = attr(ship, ids.capacitor_peak_recharge);
    let mut used = other_load.max(0.0);
    let mut out = Sustained {
        shield: 0.0,
        armor: 0.0,
        hull: 0.0,
        capped: true,
    };
    for repairer in &repairers {
        if used >= recharge {
            break;
        }
        let share = ((recharge - used) / repairer.cap_per_second).clamp(0.0, 1.0);
        out.shield += repairer.shield * share;
        out.armor += repairer.armor * share;
        out.hull += repairer.hull * share;
        used += repairer.cap_per_second;
    }
    out.shield = out.shield.min(burst.shield);
    out.armor = out.armor.min(burst.armor);
    out.hull = out.hull.min(burst.hull);
    out
}
