use std::time::Instant;

use rufa_data::{Bundle, InfoNameSde, InfoSde};

const FITS: &[(&str, &str)] = &[
    (
        "Rifter",
        "[Rifter, bench]
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
        "Vargur",
        "[Vargur, bench]
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
        "Nyx",
        "[Nyx, bench]
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

fn main() {
    let dir = rufa_data::data_dir();
    let bundle = Bundle::open(&dir).expect("sde.dat in RUFA_DATA_DIR");
    let sde = bundle.sde().expect("valid sde");
    let names = InfoNameSde::new(&sde, None).unwrap();
    let info = InfoSde::new(&sde);
    let rounds: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);

    for (name, eft) in FITS {
        let fit = match rufa_sim::format::eft::load_eft(&names, eft) {
            Ok(fit) => fit,
            Err(e) => {
                println!("{name}: EFT failed: {e:?}");
                continue;
            }
        };
        let fit = rufa_sim::fit_with_skill_level(&sde, fit, 5);
        let start = Instant::now();
        let mut calc = None;
        for _ in 0..rounds {
            calc = Some(rufa_sim::calculate(&info, &fit));
        }
        let per_call = start.elapsed().as_secs_f64() * 1e6 / rounds as f64;
        let violations = calc
            .as_ref()
            .and_then(|c| c.violations.as_ref())
            .map(|v| v.len())
            .unwrap_or(0);
        println!(
            "{name}: {per_call:.0} us per calculate, {} items, {violations} violations",
            fit.items.len()
        );
    }
}
