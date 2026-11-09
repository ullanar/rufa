use rufa_data::{Game, InfoNameSde, InfoSde};
use rufa_sim::attrs::AttrIds;
use rufa_sim::graph::plot::{Source, plot, point};
use rufa_sim::graph::target::{Profile, Target};
use rufa_sim::graph::{Ctx, EffectIds, InputKey, Kind, Quantity, Request};

const RIFTER: &str = "[Rifter, graph]
Gyrostabilizer II

1MN Afterburner II
Stasis Webifier II

200mm AutoCannon II, EMP S
200mm AutoCannon II, EMP S
";

const KESTREL: &str = "[Kestrel, graph]
Ballistic Control System II

Rocket Launcher II, Caldari Navy Mjolnir Rocket
Rocket Launcher II, Caldari Navy Mjolnir Rocket
";

const LOGI: &str = "[Scimitar, reps]
Damage Control II

Large Remote Shield Booster II
Large Remote Shield Booster II
Medium Remote Armor Repairer II
";

const NEUT: &str = "[Curse, neut]
Damage Control II

Medium Energy Neutralizer II
Medium Energy Neutralizer II
Stasis Webifier II
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

fn attr(calc: &rufa_sim::Calculation, id: i32) -> f64 {
    calc.ship.attributes[&id].value
}

#[test]
fn capacitor_and_shield_curves_match_the_panel_figures() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(RIFTER);
    let src = [Source {
        fit: &fit,
        calc: &calc,
    }];
    let mut req = Request::new(Kind::Cap);
    req.x = 2;
    req.y = 1;
    req.range = (0.0, 100.0);
    let p = plot(&ctx, &req, &src, &[]);
    assert_eq!(p.series.len(), 1, "{:?}", p.empty);
    let peak = p.series[0].points.iter().map(|p| p.1).fold(0.0, f64::max);
    let panel = attr(&calc, w.ids.capacitor_peak_recharge);
    assert!(
        close(peak, panel, 1e-3),
        "peak regen {peak} vs panel {panel}"
    );
    let (y, _) = point(&ctx, &req, &src[0], None, 25.0).unwrap();
    assert!(close(y, panel, 1e-9), "regen at 25% {y} vs {panel}");

    req = Request::new(Kind::Cap);
    req.options.capsim = false;
    req.set_input(InputKey::CapAmountT0, Some(0.0));
    let (at_period, _) = point(
        &ctx,
        &req,
        &src[0],
        None,
        attr(&calc, w.ids.recharge_rate) / 1000.0,
    )
    .unwrap();
    let capacity = attr(&calc, w.ids.capacitor_capacity);
    assert!(
        at_period > capacity * 0.98 && at_period < capacity * 0.99,
        "{at_period} of {capacity}"
    );
    req.options.capsim = true;
    req.set_input(InputKey::CapAmountT0, Some(100.0));
    let p = plot(&ctx, &req, &src, &[]);
    let pts = &p.series[0].points;
    assert!(pts.len() > 50, "{}", pts.len());
    assert!(pts.iter().all(|p| p.1 <= capacity + 1e-6 && p.1 >= 0.0));
    assert!(pts.windows(2).all(|p| p[0].0 <= p[1].0 + 1e-9));
    assert!(
        pts.iter().any(|p| p.1 < capacity * 0.99),
        "something must drain the cap"
    );

    let mut req = Request::new(Kind::Shield);
    req.x = 2;
    req.y = 1;
    let p = plot(&ctx, &req, &src, &[]);
    let peak = p.series[0].points.iter().map(|p| p.1).fold(0.0, f64::max);
    let panel = attr(&calc, w.ids.passive_shield_recharge_rate);
    assert!(
        close(peak, panel, 1e-3),
        "shield peak {peak} vs panel {panel}"
    );
}

#[test]
fn mobility_warp_and_lock_follow_the_panel_values() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(RIFTER);
    let src = [Source {
        fit: &fit,
        calc: &calc,
    }];
    let req = Request::new(Kind::Mobility);
    let align = attr(&calc, w.ids.align_time);
    let vmax = attr(&calc, w.ids.max_velocity);
    let (at_align, _) = point(&ctx, &req, &src[0], None, align).unwrap();
    assert!(
        close(at_align, vmax * 0.75, 1e-6),
        "speed at align {at_align} vs {}",
        vmax * 0.75
    );
    let mut req = Request::new(Kind::Mobility);
    req.y = 3;
    assert!(point(&ctx, &req, &src[0], None, 5.0).unwrap().0 > 0.0);

    let req = Request::new(Kind::Warp);
    let p = plot(&ctx, &req, &src, &[]);
    let pts = &p.series[0].points;
    assert!(pts.windows(2).all(|p| p[0].1 <= p[1].1 + 1e-9));
    let (twenty, _) = point(&ctx, &req, &src[0], None, 20.0).unwrap();
    let warp = attr(&calc, w.ids.base_warp_speed) * attr(&calc, w.ids.warp_speed_multiplier);
    assert!(
        twenty > 19.0 / warp && twenty < 19.0 / warp + 30.0,
        "{twenty} s at {warp} AU/s"
    );

    let req = Request::new(Kind::Lock);
    let (forty, _) = point(&ctx, &req, &src[0], None, 40.0).unwrap();
    let scan = attr(&calc, w.ids.scan_resolution);
    let expected = 40_000.0 / (scan * 40f64.asinh().powi(2));
    assert!(close(forty, expected, 1e-9));
}

#[test]
fn remote_reps_declare_the_panel_rate_and_fall_off_with_range() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(LOGI);
    let src = [Source {
        fit: &fit,
        calc: &calc,
    }];
    let req = Request::new(Kind::Reps);
    let (close_in, _) = point(&ctx, &req, &src[0], None, 1.0).unwrap();
    let panel = rufa_sim::repair::outgoing(&w.info, &w.ids, &fit, &calc);
    assert!(
        close(close_in, panel.hp(), 1e-6),
        "reps at 1 km {close_in} vs panel {}",
        panel.hp()
    );
    let (far, _) = point(&ctx, &req, &src[0], None, 200.0).unwrap();
    assert_eq!(far, 0.0);
    let mut req = Request::new(Kind::Reps);
    req.x = 1;
    req.y = 1;
    req.range = (0.0, 60.0);
    let p = plot(&ctx, &req, &src, &[]);
    assert!(p.series[0].step);
    let pts = &p.series[0].points;
    assert!(pts.windows(2).all(|p| p[0].1 <= p[1].1 + 1e-9), "{pts:?}");
    assert!(
        pts.last().unwrap().1 > panel.hp() * 50.0,
        "{:?}",
        pts.last()
    );
}

#[test]
fn ewar_strength_of_neuts_and_webs() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(NEUT);
    let src = [Source {
        fit: &fit,
        calc: &calc,
    }];
    let req = Request::new(Kind::Ewar);
    let (neut, _) = point(&ctx, &req, &src[0], None, 1.0).unwrap();
    assert!(neut > 0.0, "{neut}");
    let per_module: f64 = fit
        .items
        .iter()
        .zip(&calc.items)
        .filter(|(_, r)| r.attributes.contains_key(&w.ids.energy_neutralizer_amount))
        .map(|(_, r)| {
            r.attributes[&w.ids.energy_neutralizer_amount].value
                / (r.attributes[&w.ids.duration].value / 1000.0)
        })
        .sum();
    assert!(close(neut, per_module, 1e-6), "neut {neut} vs {per_module}");
    let mut req = Request::new(Kind::Ewar);
    req.y = 1;
    let (web, _) = point(&ctx, &req, &src[0], None, 1.0).unwrap();
    assert!(web > 50.0 && web < 70.0, "web {web}");
    req.set_input(InputKey::Resist, Some(50.0));
    let (resisted, _) = point(&ctx, &req, &src[0], None, 1.0).unwrap();
    assert!(close(resisted, web / 2.0, 1e-9), "{resisted} vs {web}");
    assert_eq!(req.y_axis().q, Quantity::WebStr);
}

#[test]
fn the_application_profile_picks_longer_ammo_further_out() {
    let Some(w) = world() else { return };
    let ctx = w.ctx();
    let (fit, calc) = w.fit(RIFTER);
    let src = [Source {
        fit: &fit,
        calc: &calc,
    }];
    let ideal = Profile::ideal();
    let targets = [Target::Profile(&ideal)];
    let mut req = Request::new(Kind::Application);
    req.range = rufa_sim::graph::plot::default_range(&ctx, &req, &src);
    assert!(req.range.1 > 5.0 && req.range.1 < 100.0, "{:?}", req.range);
    let p = plot(&ctx, &req, &src, &targets);
    assert_eq!(p.series.len(), 1, "{:?}", p.empty);
    let s = &p.series[0];
    assert!(s.segments.len() >= 2, "segments {:?}", s.segments);
    assert!(s.points.windows(2).all(|p| p[0].0 <= p[1].0 + 1e-9));
    let (y0, name0) = point(&ctx, &req, &src[0], Some(&targets[0]), 0.5).unwrap();
    let panel = attr(&calc, w.ids.dps);
    assert!(
        y0 >= panel * 0.9,
        "best ammo up close {y0} vs loaded {panel}"
    );
    assert!(name0.is_some());
    let (y_far, name_far) =
        point(&ctx, &req, &src[0], Some(&targets[0]), req.range.1 * 0.8).unwrap();
    assert!(y_far > 0.0 && y_far < y0, "{y_far} vs {y0}");
    assert_ne!(name0, name_far, "the charge should change along the range");

    let (fit, calc) = w.fit(KESTREL);
    let src = [Source {
        fit: &fit,
        calc: &calc,
    }];
    let mut req = Request::new(Kind::Application);
    req.range = rufa_sim::graph::plot::default_range(&ctx, &req, &src);
    let p = plot(&ctx, &req, &src, &targets);
    assert_eq!(p.series.len(), 1, "{:?}", p.empty);
    let (y, name) = point(&ctx, &req, &src[0], Some(&targets[0]), 2.0).unwrap();
    assert!(y > 0.0, "{y} {name:?}");
}
