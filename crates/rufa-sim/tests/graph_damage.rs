use rufa_data::{Game, InfoNameSde, InfoSde};
use rufa_sim::attrs::AttrIds;
use rufa_sim::graph::damage::{against, point, prepare, series};
use rufa_sim::graph::target::{Profile, Target};
use rufa_sim::graph::{Ctx, EffectIds, InputKey, Kind, Request};

const RIFTER: &str = "[Rifter, graph]
Gyrostabilizer II

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

const KESTREL: &str = "[Kestrel, graph]
Ballistic Control System II

Rocket Launcher II, Caldari Navy Mjolnir Rocket
Rocket Launcher II, Caldari Navy Mjolnir Rocket
";

const WEBBED: &str = "[Rifter, web]
Gyrostabilizer II

Stasis Webifier II

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

struct World {
    game: &'static Game,
    info: InfoSde<'static>,
    ids: AttrIds,
    fx: EffectIds,
}

fn world() -> Option<World> {
    let dir = std::env::var_os("RUFA_DATA_DIR")?;
    let game = Game::load(std::path::Path::new(&dir)).ok()?;
    let ids = AttrIds::resolve(&game.sde).unwrap();
    Some(World {
        game,
        info: game.info(),
        ids,
        fx: EffectIds::resolve(&game.catalog),
    })
}

impl World {
    fn ctx(&self) -> Ctx<'_, InfoSde<'static>> {
        Ctx {
            info: &self.info,
            ids: &self.ids,
            fx: &self.fx,
            catalog: &self.game.catalog,
        }
    }

    fn fit(&self, eft: &str) -> (rufa_sim::Fit, rufa_sim::Calculation) {
        let names = InfoNameSde::new(&self.game.sde, None).unwrap();
        let fit = rufa_sim::format::eft::load_eft(&names, eft).unwrap();
        let fit = rufa_sim::fit_with_skill_level(&self.game.sde, fit, 5);
        let calc = rufa_sim::calculate(&self.info, &fit);
        (fit, calc)
    }
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-9)
}

#[test]
fn autocannons_at_point_blank_match_the_panel_with_wrecking_shots() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(RIFTER);
    let panel = calc.ship.attributes[&w.ids.dps].value;
    let src = prepare(&ctx, &fit, &calc, false);
    assert_eq!(src.dealers.len(), 2, "{:?}", src.dealers);
    let ideal = Profile::ideal();
    let tgt = against(&ctx, Target::Profile(&ideal));
    let req = Request::new(Kind::Damage);
    let at_zero = point(&ctx, &req, &src, &tgt, 0.0).unwrap();
    assert!(
        close(at_zero, panel * 1.01505, 1e-6),
        "graph {at_zero} vs panel {panel}"
    );
    let at_ten = point(&ctx, &req, &src, &tgt, 10.0).unwrap();
    assert!(
        at_ten > 0.0 && at_ten < panel * 0.6,
        "10 km {at_ten} vs {panel}"
    );
    let (pts, step) = series(&ctx, &req, &src, &tgt).unwrap();
    assert!(!step);
    assert!(pts.len() > 50);
    assert!(pts.windows(2).all(|p| p[0].0 <= p[1].0));
    assert!(pts.last().unwrap().1 < panel * 0.01, "{:?}", pts.last());
}

#[test]
fn rockets_land_in_full_inside_their_flight_and_not_at_all_beyond_it() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(KESTREL);
    let panel = calc.ship.attributes[&w.ids.dps].value;
    assert!(panel > 0.0);
    let src = prepare(&ctx, &fit, &calc, false);
    let ideal = Profile::ideal();
    let tgt = against(&ctx, Target::Profile(&ideal));
    let req = Request::new(Kind::Damage);
    let near = point(&ctx, &req, &src, &tgt, 3.0).unwrap();
    assert!(
        close(near, panel, 1e-6),
        "rockets at 3 km {near} vs {panel}"
    );
    let far = point(&ctx, &req, &src, &tgt, 60.0).unwrap();
    assert_eq!(far, 0.0);
}

#[test]
fn the_volley_from_attributes_agrees_with_the_engine() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    for eft in [RIFTER, KESTREL] {
        let (fit, calc) = w.fit(eft);
        let src = prepare(&ctx, &fit, &calc, false);
        for dealer in &src.dealers {
            let engine = calc.items[dealer.index].attributes[&w.ids.volley].value;
            assert!(
                close(dealer.volley.total(), engine, 1e-6),
                "{eft}: volley {} vs engine {engine}",
                dealer.volley.total()
            );
            let cycle = dealer.cycles.as_ref().unwrap().average();
            let dps = calc.items[dealer.index].attributes[&w.ids.dps].value;
            assert!(
                close(dealer.volley.total() / cycle, dps, 1e-6),
                "{eft}: {} / {cycle} vs engine {dps}",
                dealer.volley.total()
            );
        }
    }
}

#[test]
fn a_fast_target_shrinks_rocket_damage_and_resists_cut_it_further() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(KESTREL);
    let src = prepare(&ctx, &fit, &calc, false);
    let frigate = Profile {
        name: "frigate".to_string(),
        resists: [0.5, 0.0, 0.0, 0.0],
        signature: Some(35.0),
        velocity: 3000.0,
        radius: 30.0,
        hp: None,
    };
    let tgt = against(&ctx, Target::Profile(&frigate));
    let mut req = Request::new(Kind::Damage);
    req.options.projected = false;
    let full = point(&ctx, &req, &src, &tgt, 3.0).unwrap();
    let ideal = Profile::ideal();
    let ideal_tgt = against(&ctx, Target::Profile(&ideal));
    let ideal_dps = point(&ctx, &req, &src, &ideal_tgt, 3.0).unwrap();
    assert!(
        full < ideal_dps * 0.9,
        "fast target {full} vs ideal {ideal_dps}"
    );
    req.options.resists = true;
    let resisted = point(&ctx, &req, &src, &tgt, 3.0).unwrap();
    assert!(
        close(resisted, full * 0.5, 1e-6),
        "EM rockets at 50% EM {resisted} vs {full}"
    );
    req.vectors.target.length = 0.0;
    req.options.resists = false;
    let still = point(&ctx, &req, &src, &tgt, 3.0).unwrap();
    assert!(
        close(still, ideal_dps, 1e-6),
        "sitting target {still} vs {ideal_dps}"
    );
}

#[test]
fn a_web_slows_the_target_and_raises_tracking_damage() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(WEBBED);
    let src = prepare(&ctx, &fit, &calc, false);
    assert!(src.tackle.webs.len() == 1, "{:?}", src.tackle);
    let frigate = Profile {
        name: "frigate".to_string(),
        resists: [0.0; 4],
        signature: Some(35.0),
        velocity: 1500.0,
        radius: 30.0,
        hp: None,
    };
    let tgt = against(&ctx, Target::Profile(&frigate));
    let mut req = Request::new(Kind::Damage);
    req.options.projected = false;
    let bare = point(&ctx, &req, &src, &tgt, 2.0).unwrap();
    req.options.projected = true;
    let webbed = point(&ctx, &req, &src, &tgt, 2.0).unwrap();
    assert!(webbed > bare * 1.05, "webbed {webbed} vs bare {bare}");
    let beyond = point(&ctx, &req, &src, &tgt, 40.0).unwrap();
    req.options.projected = false;
    let beyond_bare = point(&ctx, &req, &src, &tgt, 40.0).unwrap();
    assert!(close(beyond, beyond_bare, 1e-9));
}

#[test]
fn damage_over_time_climbs_in_steps_and_dps_holds() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(RIFTER);
    let src = prepare(&ctx, &fit, &calc, true);
    let ideal = Profile::ideal();
    let tgt = against(&ctx, Target::Profile(&ideal));
    let mut req = Request::new(Kind::Damage);
    req.x = 1;
    req.y = 2;
    req.range = (0.0, 30.0);
    let (pts, step) = series(&ctx, &req, &src, &tgt).unwrap();
    assert!(step);
    assert_eq!(pts.first().map(|p| p.0), Some(0.0));
    assert_eq!(pts.last().map(|p| p.0), Some(30.0));
    assert!(pts.windows(2).all(|p| p[0].1 <= p[1].1), "{pts:?}");
    let at = |pts: &[(f64, f64)], x: f64| {
        pts.iter()
            .filter(|p| p.0 == x)
            .map(|p| p.1)
            .fold(0.0, f64::max)
    };
    let volley = src.dealers.iter().map(|d| d.volley.total()).sum::<f64>() * 1.01505;
    assert!(
        close(at(&pts, 0.0), volley, 1e-6),
        "first volley {} vs {volley}",
        at(&pts, 0.0)
    );
    req.y = 0;
    let (dps, _) = series(&ctx, &req, &src, &tgt).unwrap();
    let panel = calc.ship.attributes[&w.ids.dps].value * 1.01505;
    assert!(
        close(at(&dps, 0.0), panel, 1e-6),
        "dps at 0 {} vs {panel}",
        at(&dps, 0.0)
    );
    req.y = 2;
    req.x = 0;
    assert!(
        series(&ctx, &req, &src, &tgt).is_none(),
        "damage needs a time"
    );
    req.set_input(InputKey::Time, Some(10.0));
    let (at_ten, _) = series(&ctx, &req, &src, &tgt).unwrap();
    assert!(at_ten[0].1 > volley * 2.0, "{:?}", at_ten[0]);
}
