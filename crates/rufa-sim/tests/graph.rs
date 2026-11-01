use rufa_data::{Bundle, InfoNameSde, InfoSde};
use rufa_sim::attrs::AttrIds;
use rufa_sim::graph::{self, Target};

const EFT: &str = "[Rifter, graph]
Gyrostabilizer II

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

const MISSILE_EFT: &str = "[Kestrel, graph]
Ballistic Control System II

Rocket Launcher II, Caldari Navy Mjolnir Rocket
Rocket Launcher II, Caldari Navy Mjolnir Rocket
";

#[test]
fn autocannon_dps_falls_off_with_distance() {
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
    let panel = calc.ship.attributes[&ids.dps].value;
    let at_zero = graph::dps_at(&ids, &fit, &calc, 0.0, &Target::IDEAL);
    assert!(
        (at_zero - panel).abs() < 1e-6,
        "graph {at_zero} vs panel {panel}"
    );
    let at_ten = graph::dps_at(&ids, &fit, &calc, 10_000.0, &Target::IDEAL);
    assert!(
        at_ten > 0.0 && at_ten < panel * 0.6,
        "10 km {at_ten} vs {panel}"
    );
    let reach = graph::reach(&ids, &fit, &calc);
    assert!(reach > 10_000.0 && reach < 60_000.0, "reach {reach}");
    let curve = graph::dps_curve(&ids, &fit, &calc, &Target::IDEAL, reach, 50);
    assert_eq!(curve.points.len(), 50);
    assert!(curve.points.last().unwrap().1 < panel * 0.01);

    let fit = rufa_sim::format::eft::load_eft(&names, MISSILE_EFT).unwrap();
    let fit = rufa_sim::fit_with_skill_level(&sde, fit, 5);
    let calc = rufa_sim::calculate(&info, &fit);
    let panel = calc.ship.attributes[&ids.dps].value;
    assert!(panel > 0.0);
    let near = graph::dps_at(&ids, &fit, &calc, 5_000.0, &Target::IDEAL);
    assert!(
        (near - panel).abs() < 1e-6,
        "rockets at 5 km {near} vs {panel}"
    );
    let far = graph::dps_at(&ids, &fit, &calc, 60_000.0, &Target::IDEAL);
    assert_eq!(far, 0.0);
    let frigate = Target {
        signature: 35.0,
        velocity: 1000.0,
        resists: [0.0; 4],
    };
    let applied = graph::dps_at(&ids, &fit, &calc, 5_000.0, &frigate);
    assert!(
        applied > 0.0 && applied < panel,
        "applied {applied} vs {panel}"
    );
    let em_proof = Target {
        signature: f64::INFINITY,
        velocity: 0.0,
        resists: [1.0, 0.0, 0.0, 0.0],
    };
    let blocked = graph::dps_at(&ids, &fit, &calc, 5_000.0, &em_proof);
    assert!(blocked < 1e-6, "mjolnir rockets are pure EM: {blocked}");
    let half = Target {
        signature: f64::INFINITY,
        velocity: 0.0,
        resists: [0.5, 0.5, 0.5, 0.5],
    };
    let halved = graph::dps_at(&ids, &fit, &calc, 5_000.0, &half);
    assert!((halved - panel / 2.0).abs() < 1e-6, "{halved} vs {panel}");
}
