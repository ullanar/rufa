use rufa_data::{Bundle, InfoNameSde, InfoSde};
use rufa_sim::attrs::AttrIds;

const EFT: &str = "[Rifter, cap]
Damage Control II
Gyrostabilizer II
Small Armor Repairer II

1MN Afterburner II
Warp Scrambler II
Stasis Webifier I

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

#[test]
fn simulation_agrees_with_the_engine_estimate() {
    let Some(dir) = std::env::var_os("RUFA_DATA_DIR") else {
        return;
    };
    let bundle = Bundle::open(std::path::Path::new(&dir)).unwrap();
    let sde = bundle.sde().unwrap();
    let ids = AttrIds::resolve(&sde).unwrap();
    let names = InfoNameSde::new(&sde, None).unwrap();
    let info = InfoSde::new(&sde);
    let fit = rufa_sim::format::eft::load_eft(&names, EFT).unwrap();
    let fit = rufa_sim::fit_with_skill_level(&sde, fit, 5);
    let calc = rufa_sim::calculate(&info, &fit);

    let engine = calc.ship.attributes[&ids.capacitor_depletes_in].value;
    assert!(engine > 0.0, "engine says stable: {engine}");
    let drains = rufa_sim::cap::drains(&ids, &fit, &calc);
    assert!(drains.len() >= 4, "{drains:?}");
    let result = rufa_sim::cap::simulate_fit(&ids, &fit, &calc, 0.0, 3600.0);
    let simulated = result.depleted_at.expect("simulation depletes too");
    let ratio = simulated / engine;
    assert!(
        (0.5..2.0).contains(&ratio),
        "simulated {simulated} vs engine {engine}"
    );
    assert!(result.series.len() > 10);
}

const BOOSTED: &str = "[Rifter, boosted]
Damage Control II
Gyrostabilizer II
Small Armor Repairer II

1MN Afterburner II
Warp Scrambler II
Small Capacitor Booster II, Cap Booster 75

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

const UNBOOSTED: &str = "[Rifter, unboosted]
Damage Control II
Gyrostabilizer II
Small Armor Repairer II

1MN Afterburner II
Warp Scrambler II

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

fn run(eft: &str) -> (Vec<rufa_sim::cap::Drain>, rufa_sim::cap::CapResult) {
    let dir = std::env::var_os("RUFA_DATA_DIR").unwrap();
    let bundle = Bundle::open(std::path::Path::new(&dir)).unwrap();
    let sde = bundle.sde().unwrap();
    let ids = AttrIds::resolve(&sde).unwrap();
    let names = InfoNameSde::new(&sde, None).unwrap();
    let info = InfoSde::new(&sde);
    let fit = rufa_sim::format::eft::load_eft(&names, eft).unwrap();
    let fit = rufa_sim::fit_with_skill_level(&sde, fit, 5);
    let calc = rufa_sim::calculate(&info, &fit);
    let drains = rufa_sim::cap::drains(&ids, &fit, &calc);
    let result = rufa_sim::cap::simulate_fit(&ids, &fit, &calc, 0.0, 3600.0);
    (drains, result)
}

#[test]
fn a_cap_booster_injects_and_reloads() {
    if std::env::var_os("RUFA_DATA_DIR").is_none() {
        return;
    }
    let (drains, boosted) = run(BOOSTED);
    let injector = drains
        .iter()
        .find(|d| d.amount < 0.0)
        .expect("the booster injects capacitor");
    assert_eq!(injector.amount, -75.0);
    assert_eq!(injector.clip, Some(5));
    assert_eq!(injector.reload, 10.0);
    // The injection lands when the first cycle ends, not at the start.
    assert_eq!(injector.offset, injector.cycle);

    let (_, plain) = run(UNBOOSTED);
    let without = plain.depleted_at.expect("the plain fit runs dry");
    match boosted.depleted_at {
        Some(with_booster) => assert!(
            with_booster > without,
            "boosted {with_booster} vs plain {without}"
        ),
        None => assert!(boosted.low > plain.low),
    }
}
