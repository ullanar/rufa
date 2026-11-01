use rufa_data::{Bundle, InfoNameSde, InfoSde};

const EFT: &str = "[Rifter, smoke]
Damage Control II
Gyrostabilizer II
Small Armor Repairer II

1MN Afterburner II
Warp Scrambler II
Stasis Webifier I

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
[Empty High slot]

Small Projectile Burst Aerator I
Small Trimark Armor Pump I
Small Trimark Armor Pump I
";

#[test]
fn rifter_has_ehp_and_dps() {
    let Some(dir) = std::env::var_os("RUFA_DATA_DIR") else {
        return;
    };
    let bundle = Bundle::open(std::path::Path::new(&dir)).expect("sde.dat present");
    let sde = bundle.sde().expect("valid sde.dat");
    let names = InfoNameSde::new(&sde, None).expect("name index");
    let fit = rufa_sim::format::eft::load_eft(&names, EFT).expect("EFT parses");
    let fit = rufa_sim::fit_with_skill_level(&sde, fit, 5);

    let info = InfoSde::new(&sde);
    let calc = rufa_sim::calculate(&info, &fit);

    let ehp = sde
        .attribute_name_to_id("ehp")
        .expect("synthetic ehp attribute");
    let dps = sde
        .attribute_name_to_id("damagePerSecondWithoutReload")
        .expect("synthetic dps attribute");
    let ehp = calc.ship.attributes[&ehp].value;
    let dps = calc.ship.attributes[&dps].value;
    assert!(
        calc.violations.as_deref().is_some_and(|v| v.is_empty()),
        "{:?}",
        calc.violations
    );
    assert!(ehp > 3000.0, "ehp {ehp}");
    assert!(dps > 100.0, "dps {dps}");
}
