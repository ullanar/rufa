use rufa_data::Info;

use crate::attrs::AttrIds;
use crate::mining::Mining;
use crate::place::Rack;
use crate::repair::{Remote, Sustained};
use crate::{Calculation, Fit, ItemResult, Slot, State};

#[derive(Debug, Clone, Copy, Default)]
pub struct Resources {
    pub cpu_used: f64,
    pub cpu_total: f64,
    pub power_used: f64,
    pub power_total: f64,
    pub calibration_used: f64,
    pub calibration_total: f64,
    pub drone_bandwidth_used: f64,
    pub drone_bandwidth_total: f64,
    pub drone_bay_used: f64,
    pub drone_bay_total: f64,
    pub drones_active: f64,
    pub drones_max: f64,
    pub turrets_free: f64,
    pub launchers_free: f64,
    pub cargo_used: f64,
    pub cargo_total: f64,
    pub fighter_bay_used: f64,
    pub fighter_bay_total: f64,
    pub fighter_tubes_used: f64,
    pub fighter_tubes_total: f64,
    pub holds: [f64; HOLDS.len()],
}

pub const HOLDS: [(&str, &str); 21] = [
    ("Fleet hangar", "fleetHangarCapacity"),
    ("Maintenance bay", "shipMaintenanceBayCapacity"),
    ("Fuel bay", "specialFuelBayCapacity"),
    ("Ammo hold", "specialAmmoHoldCapacity"),
    ("Ore hold", "specialOreHoldCapacity"),
    ("Mining hold", "generalMiningHoldCapacity"),
    ("Ice hold", "specialIceHoldCapacity"),
    ("Gas hold", "specialGasHoldCapacity"),
    ("Mineral hold", "specialMineralHoldCapacity"),
    ("Salvage hold", "specialSalvageHoldCapacity"),
    ("Material bay", "specialMaterialBayCapacity"),
    ("Ship hold", "specialShipHoldCapacity"),
    ("Small ship hold", "specialSmallShipHoldCapacity"),
    ("Medium ship hold", "specialMediumShipHoldCapacity"),
    ("Large ship hold", "specialLargeShipHoldCapacity"),
    ("Industrial ship hold", "specialIndustrialShipHoldCapacity"),
    ("Command center hold", "specialCommandCenterHoldCapacity"),
    ("Planetary goods hold", "specialPlanetaryCommoditiesHoldCapacity"),
    ("Infrastructure hold", "specialColonyResourcesHoldCapacity"),
    ("Mobile depot hold", "specialMobileDepotHoldCapacity"),
    ("Expedition hold", "specialExpeditionHoldCapacity"),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Layer {
    pub hp: f64,
    pub em: f64,
    pub thermal: f64,
    pub kinetic: f64,
    pub explosive: f64,
    pub ehp: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Defense {
    pub shield: Layer,
    pub armor: Layer,
    pub hull: Layer,
    pub ehp: f64,
    pub shield_recharge_time: f64,
    pub passive_shield_rate: f64,
    pub shield_boost_rate: f64,
    pub armor_repair_rate: f64,
    pub hull_repair_rate: f64,
    pub passive_shield_effective_rate: f64,
    pub shield_effective_boost_rate: f64,
    pub armor_effective_repair_rate: f64,
    pub hull_effective_repair_rate: f64,
    pub sustained: Sustained,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Offense {
    pub dps: f64,
    pub dps_with_reload: f64,
    pub volley: f64,
    pub drone_dps: f64,
    pub fighter_dps: f64,
    pub damage: [f64; 4],
}

pub const DAMAGE_NAMES: [&str; 4] = ["EM", "Thermal", "Kinetic", "Explosive"];

pub const TARGET_CLASSES: [(&str, f64); 7] = [
    ("Pod", 25.0),
    ("Interceptor", 33.0),
    ("Frigate", 38.0),
    ("Destroyer", 83.0),
    ("Cruiser", 130.0),
    ("Battlecruiser", 265.0),
    ("Battleship", 420.0),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Capacitor {
    pub capacity: f64,
    pub recharge_time: f64,
    pub peak_recharge: f64,
    pub peak_load: f64,
    pub peak_delta: f64,
    pub stable_pct: f64,
    pub depletes_in: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Navigation {
    pub max_velocity: f64,
    pub align_time: f64,
    pub signature_radius: f64,
    pub warp_speed: f64,
    pub max_warp_distance: f64,
    pub mass: f64,
    pub agility: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Targeting {
    pub max_targets: f64,
    pub max_range: f64,
    pub scan_resolution: f64,
    pub sensor_strength: f64,
    pub sensor_type: &'static str,
    pub drone_control_range: f64,
    pub lock_times: [f64; TARGET_CLASSES.len()],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub resources: Resources,
    pub defense: Defense,
    pub offense: Offense,
    pub capacitor: Capacitor,
    pub navigation: Navigation,
    pub targeting: Targeting,
    pub remote: Remote,
    pub mining: Mining,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn layer(ship: &ItemResult, hp: i32, em: i32, th: i32, ki: i32, ex: i32, ehp: i32) -> Layer {
    Layer {
        hp: attr(ship, hp),
        em: attr(ship, em),
        thermal: attr(ship, th),
        kinetic: attr(ship, ki),
        explosive: attr(ship, ex),
        ehp: attr(ship, ehp),
    }
}

fn lock_time(scan_resolution: f64, signature: f64) -> f64 {
    if scan_resolution <= 0.0 || signature <= 0.0 {
        return 0.0;
    }
    (40_000.0 / scan_resolution / signature.asinh().powi(2)).min(1800.0)
}

fn damage_split(ids: &AttrIds, fit: &Fit, calc: &Calculation) -> [f64; 4] {
    let mut out = [0.0; 4];
    for (item, result) in fit.items.iter().zip(&calc.items) {
        if result.state < State::Active {
            continue;
        }
        let dps = attr(result, ids.dps);
        if dps <= 0.0 {
            continue;
        }
        let count = match item.slot {
            Slot::DroneBay => f64::from(item.quantity),
            Slot::FighterTube(_) | Slot::FighterBay => 1.0,
            slot if Rack::of_slot(slot).is_some() => 1.0,
            _ => continue,
        };
        let source = result.charge.as_deref().unwrap_or(result);
        let mix = [
            attr(source, ids.em_damage),
            attr(source, ids.thermal_damage),
            attr(source, ids.kinetic_damage),
            attr(source, ids.explosive_damage),
        ];
        let total: f64 = mix.iter().fold(0.0, |sum, part| sum + part);
        if total <= 0.0 {
            continue;
        }
        for (slot, part) in out.iter_mut().zip(mix) {
            *slot += dps * count * part / total;
        }
    }
    out
}

impl Stats {
    pub fn from_calc(
        info: &impl Info,
        ids: &AttrIds,
        fit: &Fit,
        calc: &Calculation,
    ) -> Stats {
        let ship = &calc.ship;
        let a = |id: i32| attr(ship, id);

        let cargo_used: f64 = fit
            .items
            .iter()
            .zip(&calc.items)
            .filter(|(it, _)| it.slot == Slot::Cargo)
            .map(|(it, res)| attr(res, ids.volume) * it.quantity as f64)
            .fold(0.0, |total, volume| total + volume);

        let mut holds = [0.0; HOLDS.len()];
        for (slot, (_, name)) in holds.iter_mut().zip(HOLDS) {
            *slot = info
                .attribute_name_to_id(name)
                .map(|id| attr(ship, id))
                .unwrap_or(0.0);
        }

        let scan_resolution = a(ids.scan_resolution);
        let mut lock_times = [0.0; TARGET_CLASSES.len()];
        for (slot, (_, signature)) in lock_times.iter_mut().zip(TARGET_CLASSES) {
            *slot = lock_time(scan_resolution, signature);
        }

        let warp_need = a(ids.warp_capacitor_need) * a(ids.mass);
        let max_warp_distance = if warp_need > 0.0 {
            a(ids.capacitor_capacity) / warp_need
        } else {
            0.0
        };

        let sensors = [
            (a(ids.scan_radar), "Radar"),
            (a(ids.scan_ladar), "Ladar"),
            (a(ids.scan_magnetometric), "Magnetometric"),
            (a(ids.scan_gravimetric), "Gravimetric"),
        ];
        let (sensor_strength, sensor_type) = sensors
            .into_iter()
            .fold((0.0, ""), |best, s| if s.0 > best.0 { s } else { best });

        Stats {
            resources: Resources {
                cpu_used: a(ids.cpu_load),
                cpu_total: a(ids.cpu_output),
                power_used: a(ids.power_load),
                power_total: a(ids.power_output),
                calibration_used: a(ids.upgrade_load),
                calibration_total: a(ids.upgrade_capacity),
                drone_bandwidth_used: a(ids.drone_bandwidth_load),
                drone_bandwidth_total: a(ids.drone_bandwidth),
                drone_bay_used: a(ids.drone_capacity_load),
                drone_bay_total: a(ids.drone_capacity),
                drones_active: a(ids.drone_active),
                drones_max: attr(&calc.character, ids.max_active_drones),
                turrets_free: a(ids.turret_slots_left),
                launchers_free: a(ids.launcher_slots_left),
                cargo_used,
                cargo_total: a(ids.capacity),
                fighter_bay_used: a(ids.fighter_capacity_load),
                fighter_bay_total: a(ids.fighter_capacity),
                fighter_tubes_used: a(ids.fighter_tubes_used),
                fighter_tubes_total: a(ids.fighter_tubes),
                holds,
            },
            defense: Defense {
                shield: layer(
                    ship,
                    ids.shield_hp,
                    ids.shield_em,
                    ids.shield_thermal,
                    ids.shield_kinetic,
                    ids.shield_explosive,
                    ids.shield_ehp,
                ),
                armor: layer(
                    ship,
                    ids.armor_hp,
                    ids.armor_em,
                    ids.armor_thermal,
                    ids.armor_kinetic,
                    ids.armor_explosive,
                    ids.armor_ehp,
                ),
                hull: layer(
                    ship,
                    ids.hull_hp,
                    ids.hull_em,
                    ids.hull_thermal,
                    ids.hull_kinetic,
                    ids.hull_explosive,
                    ids.hull_ehp,
                ),
                ehp: a(ids.ehp),
                shield_recharge_time: a(ids.shield_recharge_rate) / 1000.0,
                passive_shield_rate: a(ids.passive_shield_recharge_rate),
                shield_boost_rate: a(ids.shield_boost_rate),
                armor_repair_rate: a(ids.armor_repair_rate),
                hull_repair_rate: a(ids.hull_repair_rate),
                passive_shield_effective_rate: a(ids.passive_shield_effective_recharge_rate),
                shield_effective_boost_rate: a(ids.shield_effective_boost_rate),
                armor_effective_repair_rate: a(ids.armor_effective_repair_rate),
                hull_effective_repair_rate: a(ids.hull_effective_repair_rate),
                sustained: crate::repair::sustained(info, ids, fit, calc),
            },
            offense: Offense {
                dps: a(ids.dps),
                dps_with_reload: a(ids.dps_with_reload),
                volley: a(ids.alpha),
                drone_dps: a(ids.drone_dps),
                fighter_dps: a(ids.fighter_dps),
                damage: damage_split(ids, fit, calc),
            },
            capacitor: Capacitor {
                capacity: a(ids.capacitor_capacity),
                recharge_time: a(ids.recharge_rate) / 1000.0,
                peak_recharge: a(ids.capacitor_peak_recharge),
                peak_load: a(ids.capacitor_peak_load),
                peak_delta: a(ids.capacitor_peak_delta),
                stable_pct: a(ids.capacitor_stable_pct),
                depletes_in: a(ids.capacitor_depletes_in),
            },
            navigation: Navigation {
                max_velocity: a(ids.max_velocity),
                align_time: a(ids.align_time),
                signature_radius: a(ids.signature_radius),
                warp_speed: a(ids.base_warp_speed) * a(ids.warp_speed_multiplier),
                max_warp_distance,
                mass: a(ids.mass),
                agility: a(ids.agility),
            },
            targeting: Targeting {
                max_targets: a(ids.max_targets),
                max_range: a(ids.max_target_range),
                scan_resolution,
                sensor_strength,
                sensor_type,
                drone_control_range: attr(&calc.character, ids.drone_control_distance),
                lock_times,
            },
            remote: crate::repair::outgoing(info, ids, fit, calc),
            mining: crate::mining::yields(ids, fit, calc),
        }
    }
}
