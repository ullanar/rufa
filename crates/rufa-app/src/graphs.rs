//! State of the graph pane: the request, the targets, the computed plot.

use std::collections::BTreeSet;
use std::time::Instant;

use iced::Task;
use rufa_sim::graph::plot::{self, Plot, Source};
use rufa_sim::graph::target::{Profile, ResistMode, Target};
use rufa_sim::graph::{
    AmmoTier, Ctx, DroneMode, EffectIds, InputKey, Kind, Options, Request, Toggle, Vector,
};
use rufa_store::TargetProfile;

use crate::{App, Message};

#[derive(Debug, Clone)]
pub enum GraphMsg {
    Kind(Kind),
    NextKind,
    X(usize),
    Y(usize),
    Range(String),
    RangeReset,
    Input(InputKey, String),
    Toggle(Toggle),
    DroneMode(DroneMode),
    AmmoTier(AmmoTier),
    Vector(bool, Vector),
    Target(usize),
    FitTarget(usize),
    RemoveFitTarget(usize),
    ResistMode(usize, ResistMode),
    Mute(usize),
    Pin(Option<f64>),
    Zoom(f64, f64),
    Pan(f64),
    Editor(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FitTarget {
    pub doc: usize,
    pub mode: ResistMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pinned {
    pub x: f64,
    /// Series index, value and the charge name where there is one.
    pub values: Vec<(usize, f64, Option<String>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesInfo {
    pub name: String,
    pub source: usize,
    pub target: Option<usize>,
}

/// What the last plot was computed from.
type PlotKey = (
    Request,
    Vec<u64>,
    Vec<usize>,
    Vec<FitTarget>,
    Vec<TargetProfile>,
);

pub struct GraphState {
    pub req: Request,
    pub remembered: Vec<(Kind, Request)>,
    pub range_text: String,
    pub range_ok: bool,
    pub range_touched: bool,
    pub inputs_text: Vec<(InputKey, String)>,
    pub targets_on: BTreeSet<usize>,
    pub fit_targets: Vec<FitTarget>,
    pub muted: BTreeSet<usize>,
    pub pinned: Option<Pinned>,
    pub plot: Option<Plot>,
    pub series: Vec<SeriesInfo>,
    pub took_ms: f64,
    pub editor_open: bool,
    pub cache: iced::widget::canvas::Cache,
    key: Option<PlotKey>,
}

impl Default for GraphState {
    fn default() -> GraphState {
        let req = Request::new(Kind::Damage);
        let mut state = GraphState {
            range_text: String::new(),
            range_ok: true,
            range_touched: false,
            inputs_text: Vec::new(),
            targets_on: BTreeSet::from([0]),
            fit_targets: Vec::new(),
            muted: BTreeSet::new(),
            pinned: None,
            plot: None,
            series: Vec::new(),
            took_ms: 0.0,
            editor_open: false,
            cache: iced::widget::canvas::Cache::new(),
            remembered: Vec::new(),
            req,
            key: None,
        };
        state.sync_text();
        state
    }
}

/// Reads the toggles back from the `key=value` words in settings.
pub fn options_from(text: &str) -> Options {
    let mut o = Options::default();
    for word in text.split_whitespace() {
        let Some((k, v)) = word.split_once('=') else {
            continue;
        };
        let on = v == "1" || v == "true";
        match k {
            "resists" => o.resists = on,
            "projected" => o.projected = on,
            "lock_range" => o.lock_range = on,
            "drone_range" => o.drone_range = on,
            "capsim" => o.capsim = on,
            "ancillary_reload" => o.ancillary_reload = on,
            "ehp" => o.ehp = on,
            "drone_mode" => {
                o.drone_mode = match v {
                    "target" => DroneMode::FollowTarget,
                    "attacker" => DroneMode::FollowAttacker,
                    _ => DroneMode::Auto,
                }
            }
            "ammo" => {
                o.ammo_tier = match v {
                    "t1" => AmmoTier::T1,
                    "all" => AmmoTier::All,
                    _ => AmmoTier::Navy,
                }
            }
            _ => {}
        }
    }
    o
}

pub fn options_text(o: &Options) -> String {
    let b = |v: bool| if v { "1" } else { "0" };
    format!(
        "resists={} projected={} lock_range={} drone_range={} capsim={} ancillary_reload={} ehp={} drone_mode={} ammo={}",
        b(o.resists),
        b(o.projected),
        b(o.lock_range),
        b(o.drone_range),
        b(o.capsim),
        b(o.ancillary_reload),
        b(o.ehp),
        match o.drone_mode {
            DroneMode::Auto => "auto",
            DroneMode::FollowTarget => "target",
            DroneMode::FollowAttacker => "attacker",
        },
        match o.ammo_tier {
            AmmoTier::T1 => "t1",
            AmmoTier::Navy => "navy",
            AmmoTier::All => "all",
        }
    )
}

/// Formats a number for an input box: no trailing zeros, no exponent.
pub fn num_text(v: f64) -> String {
    if v == v.round() && v.abs() < 1e9 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Parses `lo-hi`, `lo - hi` or a single number meaning zero to it.
pub fn parse_range(text: &str) -> Option<(f64, f64)> {
    let t = text.trim().replace(['–', '—'], "-");
    let parts: Vec<&str> = t
        .split('-')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    match parts[..] {
        [one] => {
            let v: f64 = one.parse().ok()?;
            (v.is_finite() && v >= 0.0).then_some((0.0, v))
        }
        [a, b] => {
            let a: f64 = a.parse().ok()?;
            let b: f64 = b.parse().ok()?;
            (a.is_finite() && b.is_finite()).then_some((a.min(b), a.max(b)))
        }
        _ => None,
    }
}

impl GraphState {
    fn sync_text(&mut self) {
        self.range_text = format!(
            "{} - {}",
            num_text(self.req.range.0),
            num_text(self.req.range.1)
        );
        self.range_ok = true;
        self.inputs_text = self
            .req
            .inputs
            .iter()
            .map(|(k, v)| (*k, v.map(num_text).unwrap_or_default()))
            .collect();
    }

    pub fn input_text(&self, key: InputKey) -> &str {
        self.inputs_text
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, s)| s.as_str())
            .unwrap_or("")
    }

    pub fn profile(p: &TargetProfile) -> Profile {
        Profile {
            name: p.name.clone(),
            resists: p.resists,
            signature: (p.signature > 0.0).then_some(p.signature),
            velocity: p.velocity,
            radius: p.radius,
            hp: (p.hp > 0.0).then_some(p.hp),
        }
    }

    fn switch(&mut self, kind: Kind, options: Options) {
        let current = std::mem::replace(&mut self.req, Request::new(kind));
        match self.remembered.iter_mut().find(|(k, _)| *k == current.kind) {
            Some(slot) => slot.1 = current.clone(),
            None => self.remembered.push((current.kind, current.clone())),
        }
        if let Some((_, saved)) = self.remembered.iter().find(|(k, _)| *k == kind) {
            self.req = saved.clone();
        }
        self.req.options = options;
        self.range_touched = false;
        self.pinned = None;
        self.sync_text();
    }
}

impl App {
    pub fn graph_ctx(&self) -> Option<(Ctx<'_, rufa_data::InfoSde<'static>>, &rufa_data::Game)> {
        let game = self.game.as_ref().ok()?;
        let ids = self.ids.as_ref()?;
        let fx = self.fx.as_ref()?;
        let info = self.info.as_ref()?;
        Some((
            Ctx {
                info,
                ids,
                fx,
                catalog: &game.catalog,
            },
            game,
        ))
    }

    fn graph_save_options(&mut self) {
        self.settings.graph_options = options_text(&self.graph.req.options);
        let _ = self.settings.save(&self.config_dir);
    }

    pub fn graph_update(&mut self, msg: GraphMsg) -> Task<Message> {
        let options = self.graph.req.options;
        match msg {
            GraphMsg::Kind(kind) => {
                if kind != self.graph.req.kind {
                    self.graph.switch(kind, options);
                }
            }
            GraphMsg::NextKind => {
                let i = Kind::ALL
                    .iter()
                    .position(|k| *k == self.graph.req.kind)
                    .unwrap_or(0);
                let next = Kind::ALL[(i + 1) % Kind::ALL.len()];
                self.graph.switch(next, options);
            }
            GraphMsg::X(i) => {
                if i < self.graph.req.kind.x_axes().len() && i != self.graph.req.x {
                    self.graph.req.x = i;
                    let x = self.graph.req.x_axis();
                    self.graph.req.range = self
                        .graph
                        .req
                        .kind
                        .main_input(x)
                        .map(|m| m.range)
                        .unwrap_or((0.0, 100.0));
                    self.graph.range_touched = false;
                    self.graph.pinned = None;
                    self.graph.sync_text();
                }
            }
            GraphMsg::Y(i) => {
                if i < self.graph.req.kind.y_axes().len() {
                    self.graph.req.y = i;
                    self.graph.pinned = None;
                }
            }
            GraphMsg::Range(text) => {
                self.graph.range_text = text;
                match parse_range(&self.graph.range_text) {
                    Some(range) => {
                        self.graph.range_ok = true;
                        self.graph.range_touched = true;
                        self.graph.req.range = range;
                        self.graph.pinned = None;
                    }
                    None => self.graph.range_ok = false,
                }
            }
            GraphMsg::RangeReset => {
                self.graph.range_touched = false;
                self.graph.req.range = self.graph_default_range();
                self.graph.pinned = None;
                self.graph.sync_text();
            }
            GraphMsg::Input(key, text) => {
                let value = text.trim().parse::<f64>().ok().filter(|v| v.is_finite());
                match self.graph.inputs_text.iter_mut().find(|(k, _)| *k == key) {
                    Some(slot) => slot.1 = text.clone(),
                    None => self.graph.inputs_text.push((key, text.clone())),
                }
                if text.trim().is_empty() || value.is_some() {
                    self.graph.req.set_input(key, value);
                    self.graph.pinned = None;
                }
            }
            GraphMsg::Toggle(toggle) => {
                let o = &mut self.graph.req.options;
                match toggle {
                    Toggle::Resists => o.resists = !o.resists,
                    Toggle::Projected => o.projected = !o.projected,
                    Toggle::LockRange => o.lock_range = !o.lock_range,
                    Toggle::DroneRange => o.drone_range = !o.drone_range,
                    Toggle::Capsim => o.capsim = !o.capsim,
                    Toggle::AncillaryReload => o.ancillary_reload = !o.ancillary_reload,
                    Toggle::Ehp => o.ehp = !o.ehp,
                    Toggle::DroneMode | Toggle::AmmoTier => {}
                }
                self.graph.pinned = None;
                self.graph_save_options();
            }
            GraphMsg::DroneMode(mode) => {
                self.graph.req.options.drone_mode = mode;
                self.graph.pinned = None;
                self.graph_save_options();
            }
            GraphMsg::AmmoTier(tier) => {
                self.graph.req.options.ammo_tier = tier;
                self.graph.pinned = None;
                self.graph_save_options();
            }
            GraphMsg::Vector(attacker, v) => {
                let v = Vector {
                    length: v.length.clamp(0.0, 1.0),
                    angle: v.angle.clamp(-180.0, 180.0),
                };
                if attacker {
                    self.graph.req.vectors.attacker = v;
                } else {
                    self.graph.req.vectors.target = v;
                }
                self.graph.pinned = None;
            }
            GraphMsg::Target(i) => {
                if i < self.targets.len() {
                    if !self.graph.targets_on.remove(&i) {
                        self.graph.targets_on.insert(i);
                    }
                    self.graph.pinned = None;
                }
            }
            GraphMsg::FitTarget(doc) => {
                if doc < self.docs.len() && !self.graph.fit_targets.iter().any(|t| t.doc == doc) {
                    self.graph.fit_targets.push(FitTarget {
                        doc,
                        mode: ResistMode::Auto,
                    });
                    self.graph.pinned = None;
                }
            }
            GraphMsg::RemoveFitTarget(i) => {
                if i < self.graph.fit_targets.len() {
                    self.graph.fit_targets.remove(i);
                    self.graph.pinned = None;
                }
            }
            GraphMsg::ResistMode(i, mode) => {
                if let Some(t) = self.graph.fit_targets.get_mut(i) {
                    t.mode = mode;
                    self.graph.pinned = None;
                }
            }
            GraphMsg::Mute(i) => {
                if !self.graph.muted.remove(&i) {
                    self.graph.muted.insert(i);
                }
            }
            GraphMsg::Pin(x) => {
                self.graph.pinned = x.map(|x| self.graph_pin(x));
            }
            GraphMsg::Zoom(factor, at) => {
                let (lo, hi) = self.graph.req.range;
                let at = at.clamp(lo, hi);
                let new_lo = (at - (at - lo) * factor).max(0.0);
                let new_hi = at + (hi - at) * factor;
                if new_hi - new_lo > 1e-6 {
                    self.graph.req.range = (new_lo, new_hi);
                    self.graph.range_touched = true;
                    self.graph.pinned = None;
                    self.graph.sync_text();
                }
            }
            GraphMsg::Pan(share) => {
                let (lo, hi) = self.graph.req.range;
                let shift = (hi - lo) * share;
                let shift = shift.max(-lo);
                self.graph.req.range = (lo + shift, hi + shift);
                self.graph.range_touched = true;
                self.graph.pinned = None;
                self.graph.sync_text();
            }
            GraphMsg::Editor(open) => self.graph.editor_open = open,
        }
        self.graph_refresh();
        Task::none()
    }

    fn graph_sources(&self) -> Vec<Source<'_>> {
        self.docs
            .iter()
            .map(|d| Source {
                fit: &d.doc.fit,
                calc: &d.doc.calc,
            })
            .collect()
    }

    fn graph_profiles(&self) -> Vec<Profile> {
        self.graph
            .targets_on
            .iter()
            .filter_map(|i| self.targets.get(*i))
            .map(GraphState::profile)
            .collect()
    }

    fn graph_targets<'a>(&'a self, profiles: &'a [Profile]) -> Vec<Target<'a>> {
        let mut out: Vec<Target<'a>> = profiles.iter().map(Target::Profile).collect();
        for t in &self.graph.fit_targets {
            if let Some(doc) = self.docs.get(t.doc) {
                out.push(Target::Fit {
                    fit: &doc.doc.fit,
                    calc: &doc.doc.calc,
                    mode: t.mode,
                });
            }
        }
        out
    }

    fn graph_target_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .graph
            .targets_on
            .iter()
            .filter_map(|i| self.targets.get(*i))
            .map(|p| p.name.clone())
            .collect();
        for t in &self.graph.fit_targets {
            if let Some(doc) = self.docs.get(t.doc) {
                names.push(self.fit_title(doc));
            }
        }
        names
    }

    pub fn graph_default_range(&self) -> (f64, f64) {
        let Some((ctx, _)) = self.graph_ctx() else {
            return self.graph.req.range;
        };
        let sources = self.graph_sources();
        plot::default_range(&ctx, &self.graph.req, &sources)
    }

    /// Exact values of every series at `x`, for the pinned readout.
    fn graph_pin(&self, x: f64) -> Pinned {
        let mut values = Vec::new();
        if let Some((ctx, _)) = self.graph_ctx() {
            let sources = self.graph_sources();
            let profiles = self.graph_profiles();
            let targets = self.graph_targets(&profiles);
            for (i, s) in self.series_of_plot().iter().enumerate() {
                let Some(source) = sources.get(s.source) else {
                    continue;
                };
                let target = s.target.and_then(|t| targets.get(t));
                if let Some((y, name)) = plot::point(&ctx, &self.graph.req, source, target, x) {
                    values.push((i, y, name));
                }
            }
        }
        Pinned { x, values }
    }

    fn series_of_plot(&self) -> &[SeriesInfo] {
        &self.graph.series
    }

    /// Recomputes the plot when anything it reads has changed.
    pub fn graph_refresh(&mut self) {
        if self.center != crate::CenterTab::Graph {
            return;
        }
        self.graph.req.options.reload = self.settings.factor_reload;
        let revs: Vec<u64> = self.docs.iter().map(|d| d.doc.rev).collect();
        let on: Vec<usize> = self.graph.targets_on.iter().copied().collect();
        self.graph.fit_targets.retain(|t| t.doc < self.docs.len());
        let key = (
            self.graph.req.clone(),
            revs,
            on,
            self.graph.fit_targets.clone(),
            self.targets.clone(),
        );
        if self.graph.key.as_ref() == Some(&key) {
            return;
        }
        if self.graph.req.kind == Kind::Application && !self.graph.range_touched {
            let range = self.graph_default_range();
            if range != self.graph.req.range {
                self.graph.req.range = range;
                self.graph.sync_text();
            }
        }
        let start = Instant::now();
        let Some((ctx, _)) = self.graph_ctx() else {
            return;
        };
        let sources = self.graph_sources();
        let profiles = self.graph_profiles();
        let targets = self.graph_targets(&profiles);
        let plot = plot::plot(&ctx, &self.graph.req, &sources, &targets);
        let target_names = self.graph_target_names();
        let one_target = targets.len() <= 1;
        let series: Vec<SeriesInfo> = plot
            .series
            .iter()
            .map(|s| {
                let source = self.fit_title(&self.docs[s.source]);
                let name = match s.target {
                    Some(t) if !one_target => {
                        format!(
                            "{source} vs {}",
                            target_names.get(t).cloned().unwrap_or_default()
                        )
                    }
                    _ => source,
                };
                SeriesInfo {
                    name,
                    source: s.source,
                    target: s.target,
                }
            })
            .collect();
        let key = (
            self.graph.req.clone(),
            self.docs.iter().map(|d| d.doc.rev).collect(),
            self.graph.targets_on.iter().copied().collect(),
            self.graph.fit_targets.clone(),
            self.targets.clone(),
        );
        self.graph.plot = Some(plot);
        self.graph.series = series;
        self.graph.key = Some(key);
        self.graph.took_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.graph.cache.clear();
        if let Some(p) = self.graph.pinned.take() {
            self.graph.pinned = Some(self.graph_pin(p.x));
        }
    }

    /// Damage of a fit at point blank against a target profile with its
    /// resists on, for the stats panel.
    pub fn applied_dps(&self, doc: &crate::Doc, profile: &TargetProfile) -> f64 {
        let Some((ctx, _)) = self.graph_ctx() else {
            return 0.0;
        };
        let profile = GraphState::profile(profile);
        let mut req = Request::new(Kind::Damage);
        req.options.resists = true;
        req.options.reload = self.settings.factor_reload;
        req.vectors.target.length = 0.0;
        let source = Source {
            fit: &doc.doc.fit,
            calc: &doc.doc.calc,
        };
        let target = Target::Profile(&profile);
        plot::point(&ctx, &req, &source, Some(&target), 0.0)
            .map(|(y, _)| y)
            .unwrap_or(0.0)
    }
}

pub fn effect_ids(game: &rufa_data::Game) -> EffectIds {
    EffectIds::resolve(&game.catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_parse_in_every_spelling() {
        assert_eq!(parse_range("0-100"), Some((0.0, 100.0)));
        assert_eq!(parse_range(" 10 - 5 "), Some((5.0, 10.0)));
        assert_eq!(parse_range("42"), Some((0.0, 42.0)));
        assert_eq!(parse_range("0 – 100"), Some((0.0, 100.0)));
        assert_eq!(parse_range("abc"), None);
        assert_eq!(parse_range("1-2-3"), None);
    }

    #[test]
    fn options_survive_the_settings_text() {
        let o = Options {
            resists: true,
            drone_mode: DroneMode::FollowAttacker,
            ammo_tier: AmmoTier::All,
            capsim: false,
            ..Options::default()
        };
        let back = options_from(&options_text(&o));
        assert_eq!(back, o);
        assert_eq!(options_from(""), Options::default());
    }

    #[test]
    fn numbers_print_without_noise() {
        assert_eq!(num_text(100.0), "100");
        assert_eq!(num_text(0.015), "0.015");
        assert_eq!(num_text(12.5), "12.5");
    }
}
