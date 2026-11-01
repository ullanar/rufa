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
    let result = rufa_sim::cap::simulate_fit(&ids, &fit, &calc, 3600.0);
    let simulated = result.depleted_at.expect("simulation depletes too");
    let ratio = simulated / engine;
    assert!(
        (0.5..2.0).contains(&ratio),
        "simulated {simulated} vs engine {engine}"
    );
    assert!(result.series.len() > 10);
}
