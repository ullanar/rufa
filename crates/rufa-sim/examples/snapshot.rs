//! Prints every stat of a set of fits as `key = value` lines, for diffing
//! one engine version against another.

use rufa_data::{Bundle, InfoNameSde, InfoSde, Sde};
use rufa_sim::attrs::AttrIds;
use rufa_sim::document::Document;
use rufa_sim::projection::Projected;
use rufa_sim::stats::{HOLDS, Stats};
use rufa_sim::{Fit, State};

const FITS: &[(&str, &str)] = &[
    (
        "rifter",
        "[Rifter, snap]
Damage Control II
Gyrostabilizer II
Small Armor Repairer II

1MN Afterburner II
Warp Scrambler II
Stasis Webifier I

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S

Small Projectile Burst Aerator I
Small Trimark Armor Pump I
Small Trimark Armor Pump I
",
    ),
    (
        "drake",
        "[Drake, snap]
Ballistic Control System II
Ballistic Control System II
Damage Control II

Large Shield Extender II
Large Shield Extender II
Multispectrum Shield Hardener II
10MN Afterburner II
EM Shield Amplifier II

Heavy Missile Launcher II, Scourge Heavy Missile
Heavy Missile Launcher II, Scourge Heavy Missile
Heavy Missile Launcher II, Scourge Heavy Missile
Heavy Missile Launcher II, Scourge Heavy Missile
Heavy Missile Launcher II, Scourge Heavy Missile
Heavy Missile Launcher II, Scourge Heavy Missile
Heavy Missile Launcher II, Scourge Heavy Missile

Medium Core Defense Field Extender I
Medium Core Defense Field Extender I
Medium Core Defense Field Extender I

Hobgoblin II x5
",
    ),
    (
        "myrmidon-navy",
        "[Myrmidon Navy Issue, snap]

Medium Armor Repairer II
Centum C-Type Explosive Energized Membrane
Multispectrum Energized Membrane II
Multispectrum Energized Membrane II
Damage Control II
Medium Armor Repairer II

50MN Quad LiF Restrained Microwarpdrive
Medium F-RX Compact Capacitor Booster, Navy Cap Booster 800
Medium F-RX Compact Capacitor Booster, Navy Cap Booster 800
Warp Scrambler II
Stasis Webifier II

Heavy Neutron Blaster II, Void M
Heavy Neutron Blaster II, Void M
Heavy Neutron Blaster II, Void M
Heavy Neutron Blaster II, Void M
[Empty High slot]
Armor Command Burst I, Armor Reinforcement Charge

Medium Auxiliary Nano Pump II
Medium Nanobot Accelerator II
Medium Ancillary Current Router I


Caldari Navy Vespa x2
Imperial Navy Praetor x1
Republic Fleet Berserker x2


Mid-grade Asklepian Alpha
Mid-grade Asklepian Beta
Low-grade Asklepian Gamma
Low-grade Asklepian Delta
Low-grade Asklepian Epsilon
Inherent Implants 'Noble' Repair Systems RS-603
Inherent Implants 'Noble' Repair Proficiency RP-903

Strong Exile Booster
Agency 'Hardshell' TB7 Dose III
Federation Defense Booster I
Tetrimon Resistance Booster III
",
    ),
    (
        "guardian",
        "[Guardian, snap]
Damage Control II
Capacitor Power Relay II

Large Remote Armor Repairer II
Large Remote Armor Repairer II
Large Remote Capacitor Transmitter II
10MN Afterburner II

Medium Armor Maintenance Bot II x5
",
    ),
    (
        "hulk",
        "[Hulk, snap]
Damage Control II

Medium Shield Extender II
Medium Shield Extender II
Medium Shield Extender II

Strip Miner I
Strip Miner I
Strip Miner I

Medium Core Defense Field Extender I

Mining Drone II x5
",
    ),
    (
        "vargur",
        "[Vargur, snap]
Gyrostabilizer II
Gyrostabilizer II
Gyrostabilizer II
Tracking Enhancer II
Damage Control II
Reactor Control Unit II

Pithum A-Type Multispectrum Shield Hardener
Pithum A-Type Multispectrum Shield Hardener
Large Shield Booster II
500MN Microwarpdrive II
Tracking Computer II, Optimal Range Script
Tracking Computer II, Optimal Range Script

800mm Repeating Cannon II, Republic Fleet EMP L
800mm Repeating Cannon II, Republic Fleet EMP L
800mm Repeating Cannon II, Republic Fleet EMP L
800mm Repeating Cannon II, Republic Fleet EMP L
Bastion Module I
Heavy Energy Neutralizer II
Small Tractor Beam II
Salvager II

Large Projectile Burst Aerator II
Large Core Defense Field Extender I

Hobgoblin II x5
Warrior II x5
",
    ),
    (
        "nyx",
        "[Nyx, snap]
Capital Armor Repairer II
Capital Armor Repairer II
Multispectrum Energized Membrane II
Multispectrum Energized Membrane II
Damage Control II
Drone Damage Amplifier II
Drone Damage Amplifier II

Capital Cap Battery II
Capital Cap Battery II
Networked Sensor Array
Drone Navigation Computer II
Omnidirectional Tracking Link II
Omnidirectional Tracking Link II

Capital Energy Neutralizer II
Capital Energy Neutralizer II
Heavy Energy Neutralizer II
Drone Link Augmentor II
Burst Jammer II
Cynosural Field Generator I

Capital Drone Durability Enhancer II
Capital Drone Durability Enhancer II
Capital Drone Speed Augmentor II

Dragonfly II x15
Shadow x6
",
    ),
];

fn n(label: &str, value: f64) {
    println!("{label} = {value:.4}");
}

fn dump(prefix: &str, s: &Stats) {
    let r = &s.resources;
    for (name, value) in [
        ("cpu_used", r.cpu_used),
        ("cpu_total", r.cpu_total),
        ("power_used", r.power_used),
        ("power_total", r.power_total),
        ("calibration_used", r.calibration_used),
        ("calibration_total", r.calibration_total),
        ("drone_bandwidth_used", r.drone_bandwidth_used),
        ("drone_bandwidth_total", r.drone_bandwidth_total),
        ("drone_bay_used", r.drone_bay_used),
        ("drone_bay_total", r.drone_bay_total),
        ("drones_active", r.drones_active),
        ("drones_max", r.drones_max),
        ("turrets_free", r.turrets_free),
        ("launchers_free", r.launchers_free),
        ("cargo_used", r.cargo_used),
        ("cargo_total", r.cargo_total),
        ("fighter_bay_used", r.fighter_bay_used),
        ("fighter_bay_total", r.fighter_bay_total),
        ("fighter_tubes_used", r.fighter_tubes_used),
        ("fighter_tubes_total", r.fighter_tubes_total),
    ] {
        n(&format!("{prefix}.resources.{name}"), value);
    }
    for (hold, value) in HOLDS.iter().zip(r.holds) {
        n(&format!("{prefix}.resources.hold.{}", hold.1), value);
    }

    let d = &s.defense;
    for (name, l) in [("shield", d.shield), ("armor", d.armor), ("hull", d.hull)] {
        for (field, value) in [
            ("hp", l.hp),
            ("em", l.em),
            ("thermal", l.thermal),
            ("kinetic", l.kinetic),
            ("explosive", l.explosive),
            ("ehp", l.ehp),
        ] {
            n(&format!("{prefix}.defense.{name}.{field}"), value);
        }
    }
    for (name, value) in [
        ("ehp", d.ehp),
        ("shield_recharge_time", d.shield_recharge_time),
        ("passive_shield_rate", d.passive_shield_rate),
        ("shield_boost_rate", d.shield_boost_rate),
        ("armor_repair_rate", d.armor_repair_rate),
        ("hull_repair_rate", d.hull_repair_rate),
        (
            "passive_shield_effective_rate",
            d.passive_shield_effective_rate,
        ),
        ("shield_effective_boost_rate", d.shield_effective_boost_rate),
        ("armor_effective_repair_rate", d.armor_effective_repair_rate),
        ("hull_effective_repair_rate", d.hull_effective_repair_rate),
        ("sustained.shield", d.sustained.shield),
        ("sustained.armor", d.sustained.armor),
        ("sustained.hull", d.sustained.hull),
    ] {
        n(&format!("{prefix}.defense.{name}"), value);
    }
    println!("{prefix}.defense.sustained.capped = {}", d.sustained.capped);
    for (i, bomb) in d.bombs.iter().enumerate() {
        n(&format!("{prefix}.defense.bombs.{i}"), *bomb);
    }

    let o = &s.offense;
    for (name, value) in [
        ("dps", o.dps),
        ("dps_with_reload", o.dps_with_reload),
        ("volley", o.volley),
        ("drone_dps", o.drone_dps),
        ("fighter_dps", o.fighter_dps),
        ("dps_spooled", o.dps_spooled),
        ("weapons", o.weapons()),
    ] {
        n(&format!("{prefix}.offense.{name}"), value);
    }
    for (i, part) in o.damage.iter().enumerate() {
        n(&format!("{prefix}.offense.damage.{i}"), *part);
    }

    let c = &s.capacitor;
    for (name, value) in [
        ("capacity", c.capacity),
        ("recharge_time", c.recharge_time),
        ("peak_recharge", c.peak_recharge),
        ("peak_load", c.peak_load),
        ("peak_delta", c.peak_delta),
        ("stable_pct", c.stable_pct),
        ("depletes_in", c.depletes_in),
    ] {
        n(&format!("{prefix}.capacitor.{name}"), value);
    }

    let v = &s.navigation;
    for (name, value) in [
        ("max_velocity", v.max_velocity),
        ("align_time", v.align_time),
        ("signature_radius", v.signature_radius),
        ("warp_speed", v.warp_speed),
        ("max_warp_distance", v.max_warp_distance),
        ("mass", v.mass),
        ("agility", v.agility),
    ] {
        n(&format!("{prefix}.navigation.{name}"), value);
    }

    let t = &s.targeting;
    for (name, value) in [
        ("max_targets", t.max_targets),
        ("max_range", t.max_range),
        ("scan_resolution", t.scan_resolution),
        ("sensor_strength", t.sensor_strength),
        ("drone_control_range", t.drone_control_range),
        ("jam_chance", t.jam_chance),
    ] {
        n(&format!("{prefix}.targeting.{name}"), value);
    }
    println!("{prefix}.targeting.sensor_type = {}", t.sensor_type);
    for (i, lock) in t.lock_times.iter().enumerate() {
        n(&format!("{prefix}.targeting.lock_time.{i}"), *lock);
    }

    for (name, rem) in [("remote", &s.remote), ("incoming", &s.incoming)] {
        for (field, value) in [
            ("shield", rem.shield),
            ("armor", rem.armor),
            ("armor_spooled", rem.armor_spooled),
            ("hull", rem.hull),
            ("capacitor", rem.capacitor),
            ("range", rem.range),
            ("drain", rem.drain),
        ] {
            n(&format!("{prefix}.{name}.{field}"), value);
        }
    }

    n(&format!("{prefix}.mining.miner"), s.mining.miner);
    n(&format!("{prefix}.mining.drone"), s.mining.drone);
    n(&format!("{prefix}.mining.drain"), s.mining.drain);
}

fn tube_the_fighters(fit: &mut Fit) {
    let mut tube: u8 = 0;
    for item in fit.items.iter_mut() {
        if format!("{:?}", item.slot).contains("Fighter") {
            item.slot = rufa_sim::Slot::FighterTube(tube);
            item.state = State::Active;
            tube += 1;
        }
    }
}

fn load(sde: &Sde, names: &InfoNameSde, eft: &str) -> Fit {
    let fit = rufa_sim::format::eft::load_eft(names, eft).expect("eft");
    rufa_sim::fit_with_skill_level(sde, fit, 5)
}

fn main() {
    let dir = rufa_data::data_dir();
    let bundle = Bundle::open(&dir).expect("sde.dat in RUFA_DATA_DIR");
    let sde = bundle.sde().expect("valid sde");
    let names = InfoNameSde::new(&sde, None).unwrap();
    let info = InfoSde::new(&sde);
    let ids = AttrIds::resolve(&sde).expect("attrs");

    println!("sde.build = {}", sde.build_number());

    for (name, eft) in FITS {
        let mut fit = load(&sde, &names, eft);
        if *name == "nyx" {
            tube_the_fighters(&mut fit);
        }
        let doc = Document::new(&info, &ids, fit.clone());
        dump(name, &doc.stats);
        n(&format!("{name}.cap.low"), doc.cap.low);
        println!(
            "{name}.cap.depleted_at = {:?}",
            doc.cap.depleted_at.map(|v| (v * 100.0).round() / 100.0)
        );
        for (i, item) in doc.calc.items.iter().enumerate() {
            n(
                &format!("{name}.item.{i}.dps"),
                item.attributes
                    .get(&ids.dps)
                    .map(|a| a.value)
                    .unwrap_or(0.0),
            );
            println!("{name}.item.{i}.state = {:?}", item.state);
        }
    }

    // The breaking change lands here: a projected stack is now one effect
    // carrying a quantity, instead of one effect per item.
    let target = load(&sde, &names, FITS[0].1);
    let mut doc = Document::new(&info, &ids, target);
    let webber = sde.type_name_to_id("Stasis Webifier II").expect("webifier");
    doc.project(
        &info,
        &ids,
        Projected::Module {
            type_id: webber,
            charge: None,
            quantity: 3,
        },
    );
    dump("projected-webs", &doc.stats);

    let logi = load(&sde, &names, FITS[3].1);
    let mut doc = Document::new(&info, &ids, load(&sde, &names, FITS[0].1));
    doc.project(
        &info,
        &ids,
        Projected::Fit {
            name: "guardian".to_string(),
            fit: Box::new(logi),
        },
    );
    dump("projected-logi", &doc.stats);
}
