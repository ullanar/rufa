mod fmt;
mod theme;
mod ui;
mod view;

use std::path::PathBuf;

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{image, pane_grid};
use iced::{Element, Subscription, Task, clipboard};
use rufa_data::Game;
use rufa_data::icons::Kind;
use rufa_sim::attrs::AttrIds;
use rufa_sim::document::Document;
use rufa_sim::projection::Projected;
use rufa_sim::{Fit, State, edit};
use rufa_store::{Character, ImplantSet, PriceCache, SavedFit, Settings, TargetProfile};

fn main() -> iced::Result {
    let mut application = iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(theme::of)
        .style(theme::root)
        .subscription(App::subscription)
        .window_size((1440.0, 900.0))
        .default_font(theme::UI)
        .antialiasing(true);
    for font in theme::FONTS {
        application = application.font(font);
    }
    application.run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Browser,
    Fit,
    Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftTab {
    Ships,
    Market,
    Character,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterTab {
    Fit,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphKind {
    DpsOverDistance,
    CapOverTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharChoice {
    All(u8),
    Saved(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamagePreset {
    Uniform,
    Em,
    Thermal,
    Kinetic,
    Explosive,
}

impl DamagePreset {
    pub const ALL: [DamagePreset; 5] = [
        DamagePreset::Uniform,
        DamagePreset::Em,
        DamagePreset::Thermal,
        DamagePreset::Kinetic,
        DamagePreset::Explosive,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DamagePreset::Uniform => "Uniform",
            DamagePreset::Em => "EM",
            DamagePreset::Thermal => "Thermal",
            DamagePreset::Kinetic => "Kinetic",
            DamagePreset::Explosive => "Explosive",
        }
    }

    pub fn profile(self) -> rufa_sim::DamageProfile {
        let (em, thermal, kinetic, explosive) = match self {
            DamagePreset::Uniform => (0.25, 0.25, 0.25, 0.25),
            DamagePreset::Em => (1.0, 0.0, 0.0, 0.0),
            DamagePreset::Thermal => (0.0, 1.0, 0.0, 0.0),
            DamagePreset::Kinetic => (0.0, 0.0, 1.0, 0.0),
            DamagePreset::Explosive => (0.0, 0.0, 0.0, 1.0),
        };
        rufa_sim::DamageProfile {
            em,
            thermal,
            kinetic,
            explosive,
        }
    }

    pub fn of(profile: &rufa_sim::DamageProfile) -> Option<DamagePreset> {
        DamagePreset::ALL.into_iter().find(|p| {
            let q = p.profile();
            (q.em - profile.em).abs() < 1e-9
                && (q.thermal - profile.thermal).abs() < 1e-9
                && (q.kinetic - profile.kinetic).abs() < 1e-9
                && (q.explosive - profile.explosive).abs() < 1e-9
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvChoice {
    pub id: i32,
    pub name: String,
}

impl std::fmt::Display for EnvChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PaneResized(pane_grid::ResizeEvent),
    LeftTab(LeftTab),
    ShipSearch(String),
    ShipGroup(Option<i32>),
    NewFit(i32),
    OpenSaved(PathBuf),
    MarketSearch(String),
    MarketGroup(Option<i32>),
    AddItem(i32),
    SelectTab(usize),
    CloseTab(usize),
    RenameFit(String),
    CycleState(usize),
    ToggleOverload(usize),
    ToggleOffline(usize),
    RemoveItem(usize),
    SelectItem(Option<usize>),
    Quantity(usize, u32),
    LoadCharge(usize, Option<i32>),
    DamageProfile(DamagePreset),
    ProjectMode(bool),
    ProjectItem(i32),
    ProjectFit(usize),
    Environment(EnvChoice),
    RemoveProjection(usize),
    SetMode(Option<i32>),
    ReplaceItem(usize, i32),
    DeleteSaved(PathBuf),
    Mutate(usize, i32),
    MutationValue(usize, i32, f64),
    RevertMutation(usize),
    ToggleSideEffect(usize, i32),
    ToggleAbility(usize, i32),
    SkillLevel(u8),
    ChooseCharacter(CharChoice),
    NewCharacter,
    DeleteCharacter,
    RenameCharacter(String),
    SetSkill(i32, u8),
    SkillSearch(String),
    AllSkillsTo(u8),
    Undo,
    Redo,
    Save,
    CopyEft,
    PasteEft,
    ImportText(Option<String>),
    Key(keyboard::Event),
    Downloaded(Result<String, String>),
    CenterTab(CenterTab),
    GraphKind(GraphKind),
    GraphSignature(String),
    GraphVelocity(String),
    TargetProfile(usize),
    TargetName(String),
    TargetResist(usize, String),
    SaveTargetProfile,
    DeleteTargetProfile(usize),
    PricesFetched(Result<std::collections::BTreeMap<i32, f64>, String>),
    RefreshPrices,
    IconsFetched(Vec<(Kind, i32, PathBuf)>),
    LatestTag(Result<String, String>),
    UpdateSde,
    ApplyImplantSet(usize),
    EsiClientId(String),
    EsiLogin,
    EsiResult(Result<rufa_data::esi::Pilot, String>),
    SaveImplantSet,
    DeleteImplantSet(usize),
    Nothing,
}

pub struct Doc {
    pub doc: Document,
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

pub struct App {
    pub downloading: bool,
    pub game: Result<&'static Game, String>,
    pub ids: Option<AttrIds>,
    pub settings: Settings,
    pub config_dir: PathBuf,
    pub docs: Vec<Doc>,
    pub active: usize,
    pub left: LeftTab,
    pub ship_search: String,
    pub ship_group: Option<i32>,
    pub saved: Vec<SavedFit>,
    pub market_search: String,
    pub market_group: Option<i32>,
    pub market_results: Vec<i32>,
    pub selected_item: Option<usize>,
    pub status: String,
    pub panes: pane_grid::State<Pane>,
    pub characters: Vec<Character>,
    pub current: CharChoice,
    pub skill_search: String,
    pub project_mode: bool,
    pub center: CenterTab,
    pub graph_kind: GraphKind,
    pub graph_signature: String,
    pub graph_velocity: String,
    pub targets: Vec<TargetProfile>,
    pub target: usize,
    pub prices: PriceCache,
    pub prices_pending: bool,
    pub sde_update: Option<String>,
    pub implant_sets: Vec<ImplantSet>,
    pub target_name: String,
    pub target_resists: [String; 4],
    pub esi_pending: bool,
    pub icons: std::collections::HashMap<(Kind, i32), image::Handle>,
    pub icons_pending: std::collections::HashSet<(Kind, i32)>,
}

impl App {
    fn boot() -> (App, Task<Message>) {
        let data_dir = rufa_data::data_dir();
        let config_dir = rufa_store::config_dir();
        let settings = Settings::load(&config_dir);
        let characters = rufa_store::list_characters(&config_dir).unwrap_or_default();
        let current = characters
            .iter()
            .position(|c| !settings.character.is_empty() && c.name == settings.character)
            .map(CharChoice::Saved)
            .unwrap_or(CharChoice::All(settings.skill_level));
        let panes = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: settings.browser_ratio.clamp(0.1, 0.6),
            a: Box::new(pane_grid::Configuration::Pane(Pane::Browser)),
            b: Box::new(pane_grid::Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: settings.fit_ratio.clamp(0.2, 0.9),
                a: Box::new(pane_grid::Configuration::Pane(Pane::Fit)),
                b: Box::new(pane_grid::Configuration::Pane(Pane::Stats)),
            }),
        });
        let mut app = App {
            downloading: false,
            game: Err("not loaded".to_string()),
            ids: None,
            settings,
            config_dir,
            docs: Vec::new(),
            active: 0,
            left: LeftTab::Ships,
            ship_search: String::new(),
            ship_group: None,
            saved: Vec::new(),
            market_search: String::new(),
            market_group: None,
            market_results: Vec::new(),
            selected_item: None,
            status: String::new(),
            panes,
            characters,
            current,
            skill_search: String::new(),
            project_mode: false,
            center: CenterTab::Fit,
            graph_kind: GraphKind::DpsOverDistance,
            graph_signature: String::new(),
            graph_velocity: String::new(),
            targets: Vec::new(),
            target: 0,
            prices: PriceCache::default(),
            prices_pending: false,
            sde_update: None,
            implant_sets: Vec::new(),
            target_name: String::new(),
            target_resists: Default::default(),
            esi_pending: false,
            icons: Default::default(),
            icons_pending: Default::default(),
        };
        app.implant_sets = rufa_store::implants::load(&app.config_dir);
        app.targets = rufa_store::targets::load(&app.config_dir);
        app.prices = PriceCache::load(&app.config_dir);
        if rufa_data::update::needs_download(&data_dir) {
            app.downloading = true;
            app.status = format!("Downloading game data into {}", data_dir.display());
            let task = Task::perform(
                async move { rufa_data::update::download(&data_dir, |_| {}).map_err(|e| e.to_string()) },
                Message::Downloaded,
            );
            return (app, task);
        }
        app.load_game(&data_dir);
        app.open_args();
        let prices = Task::batch([app.fetch_prices(), app.fetch_icons()]);
        let check = Task::perform(
            async { rufa_data::update::latest_tag().map_err(|e| e.to_string()) },
            Message::LatestTag,
        );
        (app, Task::batch([prices, check]))
    }

    fn open_args(&mut self) {
        for arg in std::env::args_os().skip(1) {
            if arg == "--market" {
                self.left = LeftTab::Market;
            } else if arg == "--skills" {
                self.left = LeftTab::Character;
            } else if arg == "--graph" {
                self.center = CenterTab::Graph;
            } else if arg == "--graph=cap" {
                self.center = CenterTab::Graph;
                self.graph_kind = GraphKind::CapOverTime;
            } else {
                self.open_path(PathBuf::from(arg));
            }
        }
    }

    fn load_game(&mut self, data_dir: &std::path::Path) {
        self.game = Game::load(data_dir).map_err(|e| format!("{}: {e}", data_dir.display()));
        self.ids = match &self.game {
            Ok(game) => AttrIds::resolve(&game.sde).ok(),
            Err(_) => None,
        };
        let tag = rufa_data::update::installed_tag(data_dir).unwrap_or_default();
        self.status = match (&self.game, &self.ids) {
            (Ok(game), Some(_)) => format!("SDE build {} {tag} loaded", game.build_number()),
            (Ok(_), None) => "SDE loaded but too old for this build".to_string(),
            (Err(e), _) => format!("No game data: {e}"),
        };
        self.saved = match &self.game {
            Ok(game) => rufa_store::saved_fits(&game.info(), &self.config_dir).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
    }

    fn open_path(&mut self, path: PathBuf) {
        let Some((game, _)) = self.ready() else {
            return;
        };
        let is_eft = path.extension().is_some_and(|e| e == "eft");
        let result = if is_eft {
            std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| self.parse_eft(game, &text))
                .map(|fit| (fit, None))
        } else {
            rufa_store::load_fit(&game.info(), &path)
                .map(|fit| (fit, Some(path.clone())))
                .map_err(|e| e.to_string())
        };
        match result {
            Ok((mut fit, saved_path)) => {
                fit.character.skills = self.skills(game);
                self.open_fit(fit, saved_path);
            }
            Err(e) => self.status = format!("Cannot open {}: {e}", path.display()),
        }
    }

    fn parse_eft(&self, game: &'static Game, text: &str) -> Result<Fit, String> {
        let info = rufa_data::InfoNameSde::new(&game.sde, game.names.as_ref())
            .map_err(|e| format!("names.dat mismatch: {e}"))?;
        let trimmed = text.trim();
        let mut fit = if looks_like_dna(trimmed) {
            rufa_sim::format::dna::load_dna(&game.info(), trimmed)
                .map_err(|e| format!("DNA: {e:?}"))?
        } else {
            rufa_sim::format::eft::load_eft(&info, text).map_err(|e| format!("{e:?}"))?
        };
        rufa_sim::format::post_load(&game.info(), &mut fit);
        Ok(fit)
    }

    fn title(&self) -> String {
        match self.docs.get(self.active) {
            Some(doc) => format!("{} - Rufa", self.fit_title(doc)),
            None => "Rufa".to_string(),
        }
    }

    pub fn fit_title(&self, doc: &Doc) -> String {
        let hull = self.type_name(doc.doc.fit.ship.type_id);
        match doc.doc.fit.name.as_deref().filter(|n| !n.is_empty()) {
            Some(name) => format!("{hull}: {name}"),
            None => hull.to_string(),
        }
    }

    pub fn type_name(&self, type_id: i32) -> &str {
        match &self.game {
            Ok(game) => game.type_name(type_id),
            Err(_) => "",
        }
    }

    fn ready(&self) -> Option<(&'static Game, AttrIds)> {
        match (&self.game, self.ids) {
            (Ok(game), Some(ids)) => Some((game, ids)),
            _ => None,
        }
    }

    fn skills(&self, game: &Game) -> std::collections::BTreeMap<i32, u8> {
        match self.current {
            CharChoice::All(level) => {
                edit::all_skills(game.catalog.skills().iter().map(|s| s.id), level)
            }
            CharChoice::Saved(i) => self
                .characters
                .get(i)
                .map(|c| c.skills.clone())
                .unwrap_or_default(),
        }
    }

    fn apply_skills(&mut self) {
        if let Some((game, ids)) = self.ready() {
            let skills = self.skills(game);
            for doc in &mut self.docs {
                doc.doc.fit.character.skills = skills.clone();
                doc.doc.recalc(&game.info(), &ids);
            }
        }
    }

    fn remember_character(&mut self) {
        match self.current {
            CharChoice::All(level) => {
                self.settings.skill_level = level;
                self.settings.character.clear();
            }
            CharChoice::Saved(i) => {
                self.settings.character = self
                    .characters
                    .get(i)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
            }
        }
        let _ = self.settings.save(&self.config_dir);
    }

    fn save_current_character(&mut self) {
        if let CharChoice::Saved(i) = self.current
            && let Some(c) = self.characters.get_mut(i)
            && let Err(e) = rufa_store::save_character(&self.config_dir, c)
        {
            self.status = format!("Cannot save character: {e}");
        }
    }

    fn open_fit(&mut self, fit: Fit, path: Option<PathBuf>) {
        let Some((game, ids)) = self.ready() else {
            return;
        };
        let doc = Document::new(&game.info(), &ids, fit);
        self.status = format!(
            "Calculated in {:.2} ms",
            doc.last_calc.as_secs_f64() * 1000.0
        );
        self.docs.push(Doc {
            doc,
            path,
            dirty: false,
        });
        self.active = self.docs.len() - 1;
        self.selected_item = None;
    }

    fn stale_price_types(&self) -> Vec<i32> {
        let wanted = self.docs.iter().flat_map(|d| {
            std::iter::once(d.doc.fit.ship.type_id)
                .chain(d.doc.fit.items.iter().map(|it| it.type_id))
                .chain(
                    d.doc
                        .fit
                        .items
                        .iter()
                        .filter_map(|it| it.charge.as_ref().map(|c| c.type_id)),
                )
        });
        self.prices.stale(wanted)
    }

    pub fn icon(&self, kind: Kind, type_id: i32) -> Option<image::Handle> {
        self.icons.get(&(kind, type_id)).cloned()
    }

    fn wanted_icons(&self) -> Vec<(Kind, i32)> {
        let mut wanted = Vec::new();
        for d in &self.docs {
            wanted.push((Kind::Render, d.doc.fit.ship.type_id));
            for it in &d.doc.fit.items {
                wanted.push((Kind::Icon, it.type_id));
                if let Some(c) = &it.charge {
                    wanted.push((Kind::Icon, c.type_id));
                }
            }
        }
        wanted.sort();
        wanted.dedup();
        wanted.retain(|k| !self.icons.contains_key(k) && !self.icons_pending.contains(k));
        wanted
    }

    fn fetch_icons(&mut self) -> Task<Message> {
        let dir = rufa_data::data_dir();
        let mut missing = Vec::new();
        for key in self.wanted_icons() {
            let file = rufa_data::icons::path(&dir, key.0, key.1);
            if file.is_file() {
                self.icons.insert(key, image::Handle::from_path(file));
            } else {
                missing.push(key);
            }
        }
        if missing.is_empty() {
            return Task::none();
        }
        self.icons_pending.extend(missing.iter().copied());
        Task::perform(
            async move { rufa_data::icons::fetch(&dir, &missing) },
            Message::IconsFetched,
        )
    }

    fn fetch_prices(&mut self) -> Task<Message> {
        if self.prices_pending {
            return Task::none();
        }
        let types = self.stale_price_types();
        if types.is_empty() {
            return Task::none();
        }
        self.prices_pending = true;
        Task::perform(
            async move { rufa_data::price::fetch(rufa_data::price::REGION_THE_FORGE, &types) },
            Message::PricesFetched,
        )
    }

    pub fn fit_price(&self, doc: &Doc) -> Option<f64> {
        let mut total = self.prices.get(doc.doc.fit.ship.type_id)?;
        for item in &doc.doc.fit.items {
            total += self.prices.get(item.type_id).unwrap_or(0.0) * item.quantity as f64;
            if let Some(charge) = &item.charge {
                total += self.prices.get(charge.type_id).unwrap_or(0.0);
            }
        }
        Some(total)
    }

    fn edit_active(
        &mut self,
        f: impl FnOnce(&mut Fit, &rufa_sim::layout::SlotLayout) -> Result<(), edit::EditError>,
    ) {
        let Some((game, ids)) = self.ready() else {
            return;
        };
        let Some(doc) = self.docs.get_mut(self.active) else {
            return;
        };
        match doc.doc.edit(&game.info(), &ids, f) {
            Ok(()) => {
                doc.dirty = true;
                self.status = format!(
                    "Calculated in {:.2} ms",
                    doc.doc.last_calc.as_secs_f64() * 1000.0
                );
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                let (browser, fit) = self.pane_ratios();
                self.settings.browser_ratio = browser;
                self.settings.fit_ratio = fit;
                let _ = self.settings.save(&self.config_dir);
            }
            Message::LeftTab(tab) => self.left = tab,
            Message::ShipSearch(s) => self.ship_search = s,
            Message::ShipGroup(g) => self.ship_group = g,
            Message::NewFit(type_id) => {
                if let Some((game, _)) = self.ready() {
                    let fit = edit::new_fit(type_id, self.skills(game));
                    self.open_fit(fit, None);
                }
            }
            Message::OpenSaved(path) => {
                if let Some((game, _)) = self.ready() {
                    if let Some(i) = self
                        .docs
                        .iter()
                        .position(|d| d.path.as_deref() == Some(&path))
                    {
                        self.active = i;
                    } else {
                        match rufa_store::load_fit(&game.info(), &path) {
                            Ok(mut fit) => {
                                fit.character.skills = self.skills(game);
                                self.open_fit(fit, Some(path));
                            }
                            Err(e) => self.status = format!("Cannot open fit: {e}"),
                        }
                    }
                }
            }
            Message::MarketSearch(s) => {
                self.market_results = match &self.game {
                    Ok(game) if s.trim().len() >= 2 => game.catalog.search(&s, 200),
                    _ => Vec::new(),
                };
                self.market_search = s;
            }
            Message::MarketGroup(g) => self.market_group = g,
            Message::AddItem(type_id) if self.project_mode => {
                return self.update(Message::ProjectItem(type_id));
            }
            Message::AddItem(type_id) => {
                if self.docs.is_empty() {
                    self.status = "Open a fit first: pick a hull on the Ships tab".to_string();
                } else if let Some((game, ids)) = self.ready() {
                    let targets = self.charge_targets(game, ids, type_id);
                    if targets.is_empty() {
                        self.edit_active(|fit, layout| {
                            edit::add_checked(fit, &game.info(), &ids, layout, type_id).map(|_| ())
                        });
                    } else {
                        self.edit_active(|fit, _| {
                            for i in targets {
                                edit::set_charge(fit, i, Some(type_id))?;
                            }
                            Ok(())
                        });
                    }
                }
            }
            Message::LoadCharge(i, charge) => {
                self.edit_active(|fit, _| edit::set_charge(fit, i, charge))
            }
            Message::DamageProfile(preset) => self.edit_active(|fit, _| {
                fit.environment.damage_profile = preset.profile();
                Ok(())
            }),
            Message::ProjectMode(on) => self.project_mode = on,
            Message::ProjectItem(type_id) => {
                let source = Projected::Module {
                    type_id,
                    charge: None,
                    quantity: 1,
                };
                self.project(source);
            }
            Message::ProjectFit(tab) => {
                if let Some(source) = self.docs.get(tab) {
                    let name = self.fit_title(source);
                    let fit = Box::new(source.doc.fit.clone());
                    self.project(Projected::Fit { name, fit });
                }
            }
            Message::Environment(choice) => {
                if let Some((game, ids)) = self.ready()
                    && let Some(doc) = self.docs.get_mut(self.active)
                {
                    doc.doc
                        .projections
                        .retain(|p| !matches!(p, Projected::Beacon(_)));
                    if choice.id != 0 {
                        doc.doc.projections.push(Projected::Beacon(choice.id));
                    }
                    doc.doc.recalc(&game.info(), &ids);
                }
            }
            Message::RemoveProjection(i) => {
                if let Some((game, ids)) = self.ready()
                    && let Some(doc) = self.docs.get_mut(self.active)
                {
                    doc.doc.unproject(&game.info(), &ids, i);
                }
            }
            Message::SetMode(mode) => self.edit_active(|fit, _| {
                fit.ship.mode = mode;
                Ok(())
            }),
            Message::ReplaceItem(i, type_id) => self.edit_active(|fit, _| {
                let item = fit.items.get_mut(i).ok_or(edit::EditError::NoSuchItem(i))?;
                item.type_id = type_id;
                Ok(())
            }),
            Message::Mutate(i, mutaplasmid) => {
                if let Some((game, _)) = self.ready() {
                    self.edit_active(|fit, _| {
                        edit::mutate(fit, &game.sde, &game.info(), i, mutaplasmid)
                    });
                }
            }
            Message::MutationValue(i, attribute, value) => {
                self.edit_active(|fit, _| edit::set_mutation_value(fit, i, attribute, value));
            }
            Message::RevertMutation(i) => self.edit_active(|fit, _| edit::revert_mutation(fit, i)),
            Message::ToggleSideEffect(i, effect) => {
                self.edit_active(|fit, _| edit::toggle_side_effect(fit, i, effect))
            }
            Message::ToggleAbility(i, effect) => {
                if let Some((game, _)) = self.ready() {
                    self.edit_active(|fit, _| edit::toggle_ability(fit, &game.info(), i, effect));
                }
            }
            Message::DeleteSaved(path) => {
                if let Some((game, _)) = self.ready() {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            self.status = format!("Deleted {}", path.display());
                            for doc in &mut self.docs {
                                if doc.path.as_deref() == Some(&path) {
                                    doc.path = None;
                                    doc.dirty = true;
                                }
                            }
                            self.saved = rufa_store::saved_fits(&game.info(), &self.config_dir)
                                .unwrap_or_default();
                        }
                        Err(e) => self.status = format!("Cannot delete: {e}"),
                    }
                }
            }
            Message::SelectTab(i) => {
                if i < self.docs.len() {
                    self.active = i;
                    self.selected_item = None;
                }
            }
            Message::CloseTab(i) => {
                if i < self.docs.len() {
                    self.docs.remove(i);
                    if self.active >= self.docs.len() {
                        self.active = self.docs.len().saturating_sub(1);
                    }
                    self.selected_item = None;
                }
            }
            Message::RenameFit(name) => {
                let name = if name.is_empty() { None } else { Some(name) };
                if let Some(doc) = self.docs.get_mut(self.active) {
                    doc.doc.fit.name = name;
                    doc.dirty = true;
                }
            }
            Message::CycleState(i) => {
                let max = self.max_state(i);
                self.edit_active(|fit, _| {
                    let current = fit
                        .items
                        .get(i)
                        .map(|it| it.state)
                        .unwrap_or(State::Offline);
                    edit::set_state(fit, i, edit::cycle_state(current, max))
                });
            }
            Message::ToggleOverload(i) => {
                let max = self.max_state(i);
                self.edit_active(|fit, _| {
                    let current = fit
                        .items
                        .get(i)
                        .map(|it| it.state)
                        .unwrap_or(State::Offline);
                    edit::set_state(fit, i, edit::toggle_overload(current, max))
                });
            }
            Message::ToggleOffline(i) => {
                self.edit_active(|fit, _| {
                    let current = fit
                        .items
                        .get(i)
                        .map(|it| it.state)
                        .unwrap_or(State::Offline);
                    edit::set_state(fit, i, edit::toggle_offline(current))
                });
            }
            Message::RemoveItem(i) => {
                self.selected_item = None;
                self.edit_active(|fit, _| edit::remove(fit, i));
            }
            Message::SelectItem(i) => self.selected_item = i,
            Message::Quantity(i, q) => self.edit_active(|fit, _| edit::set_quantity(fit, i, q)),
            Message::SkillLevel(level) => {
                return self.update(Message::ChooseCharacter(CharChoice::All(level)));
            }
            Message::ChooseCharacter(choice) => {
                if let CharChoice::Saved(i) = choice
                    && i >= self.characters.len()
                {
                    return Task::none();
                }
                self.current = choice;
                self.remember_character();
                self.apply_skills();
            }
            Message::NewCharacter => {
                let n = self.characters.len() + 1;
                let mut c = Character::new(format!("Character {n}"));
                if let Err(e) = rufa_store::save_character(&self.config_dir, &mut c) {
                    self.status = format!("Cannot create character: {e}");
                    return Task::none();
                }
                self.characters.push(c);
                return self.update(Message::ChooseCharacter(CharChoice::Saved(
                    self.characters.len() - 1,
                )));
            }
            Message::DeleteCharacter => {
                if let CharChoice::Saved(i) = self.current
                    && i < self.characters.len()
                {
                    let c = self.characters.remove(i);
                    let _ = rufa_store::delete_character(&c);
                    return self.update(Message::ChooseCharacter(CharChoice::All(
                        self.settings.skill_level,
                    )));
                }
            }
            Message::RenameCharacter(name) => {
                if let CharChoice::Saved(i) = self.current
                    && let Some(c) = self.characters.get_mut(i)
                {
                    c.name = name;
                    self.save_current_character();
                    self.remember_character();
                }
            }
            Message::SetSkill(type_id, level) => {
                if let CharChoice::Saved(i) = self.current
                    && let Some(c) = self.characters.get_mut(i)
                {
                    if level == 0 {
                        c.skills.remove(&type_id);
                    } else {
                        c.skills.insert(type_id, level.min(5));
                    }
                    self.save_current_character();
                    self.apply_skills();
                }
            }
            Message::SkillSearch(s) => self.skill_search = s,
            Message::AllSkillsTo(level) => {
                if let (CharChoice::Saved(i), Ok(game)) = (self.current, &self.game)
                    && let Some(c) = self.characters.get_mut(i)
                {
                    c.skills = if level == 0 {
                        Default::default()
                    } else {
                        edit::all_skills(game.catalog.skills().iter().map(|s| s.id), level)
                    };
                    self.save_current_character();
                    self.apply_skills();
                }
            }
            Message::Undo => {
                if let Some((game, ids)) = self.ready()
                    && let Some(doc) = self.docs.get_mut(self.active)
                    && doc.doc.undo(&game.info(), &ids)
                {
                    doc.dirty = true;
                }
            }
            Message::Redo => {
                if let Some((game, ids)) = self.ready()
                    && let Some(doc) = self.docs.get_mut(self.active)
                    && doc.doc.redo(&game.info(), &ids)
                {
                    doc.dirty = true;
                }
            }
            Message::Save => {
                if let Some((game, _)) = self.ready()
                    && let Some(doc) = self.docs.get_mut(self.active)
                {
                    let hull = game.type_name(doc.doc.fit.ship.type_id).to_string();
                    match rufa_store::save_fit(
                        &game.info(),
                        &self.config_dir,
                        &doc.doc.fit,
                        doc.path.as_deref(),
                        &hull,
                    ) {
                        Ok(path) => {
                            self.status = format!("Saved {}", path.display());
                            doc.path = Some(path);
                            doc.dirty = false;
                            self.saved = rufa_store::saved_fits(&game.info(), &self.config_dir)
                                .unwrap_or_default();
                        }
                        Err(e) => self.status = format!("Save failed: {e}"),
                    }
                }
            }
            Message::CopyEft => {
                if let Some((game, _)) = self.ready()
                    && let Some(doc) = self.docs.get(self.active)
                {
                    match rufa_sim::format::eft::save_eft(&game.info(), &doc.doc.fit) {
                        Ok(text) => {
                            self.status = "EFT copied to clipboard".to_string();
                            return clipboard::write(text);
                        }
                        Err(e) => self.status = format!("EFT export failed: {e:?}"),
                    }
                }
            }
            Message::PasteEft => return clipboard::read().map(Message::ImportText),
            Message::ImportText(text) => {
                let Some(text) = text else {
                    self.status = "Clipboard is empty".to_string();
                    return Task::none();
                };
                if let Some((game, _)) = self.ready() {
                    match self.parse_eft(game, &text) {
                        Ok(mut fit) => {
                            fit.character.skills = self.skills(game);
                            self.open_fit(fit, None);
                            self.status = "Fit imported from clipboard".to_string();
                        }
                        Err(e) => self.status = format!("Import failed: {e}"),
                    }
                }
            }
            Message::Key(event) => return self.on_key(event),
            Message::CenterTab(tab) => self.center = tab,
            Message::GraphKind(kind) => self.graph_kind = kind,
            Message::GraphSignature(s) => self.graph_signature = s,
            Message::TargetProfile(i) => {
                if i < self.targets.len() {
                    self.target = i;
                    let p = &self.targets[i];
                    self.graph_signature = if p.signature > 0.0 {
                        format!("{}", p.signature)
                    } else {
                        String::new()
                    };
                    self.graph_velocity = if p.velocity > 0.0 {
                        format!("{}", p.velocity)
                    } else {
                        String::new()
                    };
                }
            }
            Message::TargetName(s) => self.target_name = s,
            Message::TargetResist(i, s) => {
                if i < 4 {
                    self.target_resists[i] = s;
                }
            }
            Message::SaveTargetProfile => {
                let name = self.target_name.trim().to_string();
                if name.is_empty() {
                    self.status = "Give the target profile a name".to_string();
                    return Task::none();
                }
                let mut resists = [0.0; 4];
                for (i, s) in self.target_resists.iter().enumerate() {
                    let v: f64 = s.trim().trim_end_matches('%').parse().unwrap_or(0.0);
                    resists[i] = if v > 1.0 { v / 100.0 } else { v }.clamp(0.0, 1.0);
                }
                let profile = TargetProfile {
                    name: name.clone(),
                    resists,
                    signature: self.graph_signature.trim().parse().unwrap_or(0.0),
                    velocity: self.graph_velocity.trim().parse().unwrap_or(0.0),
                };
                match self.targets.iter().position(|p| p.name == name) {
                    Some(i) => self.targets[i] = profile,
                    None => self.targets.push(profile),
                }
                self.target = self
                    .targets
                    .iter()
                    .position(|p| p.name == name)
                    .unwrap_or(0);
                let _ = rufa_store::targets::save(&self.config_dir, &self.targets);
            }
            Message::DeleteTargetProfile(i) => {
                let builtin = rufa_store::targets::builtin();
                if i < self.targets.len() && !builtin.iter().any(|b| b.name == self.targets[i].name)
                {
                    self.targets.remove(i);
                    if self.target >= self.targets.len() {
                        self.target = 0;
                    }
                    let _ = rufa_store::targets::save(&self.config_dir, &self.targets);
                }
            }
            Message::PricesFetched(result) => {
                self.prices_pending = false;
                match result {
                    Ok(prices) => {
                        self.prices.insert_all(&prices);
                        let _ = self.prices.save(&self.config_dir);
                    }
                    Err(e) => self.status = format!("Prices: {e}"),
                }
            }
            Message::RefreshPrices => return self.fetch_prices(),
            Message::IconsFetched(list) => {
                for (kind, type_id, file) in list {
                    self.icons_pending.remove(&(kind, type_id));
                    self.icons
                        .insert((kind, type_id), image::Handle::from_path(file));
                }
                self.icons_pending.clear();
            }
            Message::LatestTag(result) => {
                if let Ok(latest) = result {
                    let installed = rufa_data::update::installed_tag(&rufa_data::data_dir());
                    if installed.as_deref() != Some(latest.as_str()) {
                        self.sde_update = Some(latest);
                    }
                }
            }
            Message::EsiClientId(id) => {
                self.settings.esi_client_id = id.trim().to_string();
                let _ = self.settings.save(&self.config_dir);
            }
            Message::EsiLogin => {
                let client_id = self.settings.esi_client_id.clone();
                if client_id.is_empty() {
                    self.status = "Set the ESI client id first: register an application at developers.eveonline.com".to_string();
                    return Task::none();
                }
                if self.esi_pending {
                    return Task::none();
                }
                self.esi_pending = true;
                self.status = "Waiting for the EVE login in the browser".to_string();
                return Task::perform(
                    async move { rufa_data::esi::login(&client_id) },
                    Message::EsiResult,
                );
            }
            Message::EsiResult(result) => {
                self.esi_pending = false;
                match result {
                    Ok(pilot) => return self.adopt_pilot(pilot),
                    Err(e) => self.status = format!("EVE login failed: {e}"),
                }
            }
            Message::ApplyImplantSet(i) => {
                if let (Some(set), Some((game, ids))) =
                    (self.implant_sets.get(i).cloned(), self.ready())
                {
                    self.edit_active(|fit, layout| {
                        for implant in set.implants {
                            edit::add(fit, &game.info(), &ids, layout, implant)?;
                        }
                        Ok(())
                    });
                }
            }
            Message::SaveImplantSet => {
                if let Some(doc) = self.docs.get(self.active) {
                    let implants: Vec<i32> = doc
                        .doc
                        .fit
                        .items
                        .iter()
                        .filter(|it| matches!(it.slot, rufa_sim::Slot::Implant(_)))
                        .map(|it| it.type_id)
                        .collect();
                    if implants.is_empty() {
                        self.status = "No implants on this fit to save".to_string();
                        return Task::none();
                    }
                    let name = format!(
                        "{} set {}",
                        self.fit_title(doc),
                        self.implant_sets.len() + 1
                    );
                    self.implant_sets.push(ImplantSet { name, implants });
                    let _ = rufa_store::implants::save(&self.config_dir, &self.implant_sets);
                }
            }
            Message::DeleteImplantSet(i) => {
                if i < self.implant_sets.len() {
                    self.implant_sets.remove(i);
                    let _ = rufa_store::implants::save(&self.config_dir, &self.implant_sets);
                }
            }
            Message::UpdateSde => {
                if !self.downloading {
                    self.downloading = true;
                    let data_dir = rufa_data::data_dir();
                    self.status = format!("Updating game data in {}", data_dir.display());
                    return Task::perform(
                        async move {
                            rufa_data::update::download(&data_dir, |_| {})
                                .map_err(|e| e.to_string())
                        },
                        Message::Downloaded,
                    );
                }
            }
            Message::GraphVelocity(s) => self.graph_velocity = s,
            Message::Downloaded(result) => {
                self.downloading = false;
                match result {
                    Ok(_) => {
                        let data_dir = rufa_data::data_dir();
                        let first_load = self.game.is_err();
                        self.load_game(&data_dir);
                        self.sde_update = None;
                        if first_load {
                            self.open_args();
                        } else if let Some((game, ids)) = self.ready() {
                            for doc in &mut self.docs {
                                doc.doc.recalc(&game.info(), &ids);
                            }
                        }
                    }
                    Err(e) => {
                        self.status = format!(
                            "Download failed: {e}. Put sde.dat into {} and restart.",
                            rufa_data::data_dir().display()
                        )
                    }
                }
            }
            Message::Nothing => {}
        }
        Task::batch([self.fetch_prices(), self.fetch_icons()])
    }

    fn charge_targets(&self, game: &'static Game, ids: AttrIds, type_id: i32) -> Vec<usize> {
        let Some(doc) = self.docs.get(self.active) else {
            return Vec::new();
        };
        let info = game.info();
        let accepts = |i: usize| {
            let item = &doc.doc.fit.items[i];
            rufa_sim::place::Rack::of_slot(item.slot).is_some()
                && rufa_sim::place::accepts_charge(&info, &ids, item.type_id, type_id)
        };
        if let Some(i) = self.selected_item.filter(|i| *i < doc.doc.fit.items.len()) {
            return if accepts(i) { vec![i] } else { Vec::new() };
        }
        (0..doc.doc.fit.items.len())
            .filter(|i| accepts(*i))
            .collect()
    }

    fn project(&mut self, source: Projected) {
        if let Some((game, ids)) = self.ready()
            && let Some(doc) = self.docs.get_mut(self.active)
        {
            doc.doc.project(&game.info(), &ids, source);
            self.status = format!(
                "Calculated in {:.2} ms",
                doc.doc.last_calc.as_secs_f64() * 1000.0
            );
        } else {
            self.status = "Open a fit first".to_string();
        }
    }

    pub fn projection_label(&self, p: &Projected) -> String {
        match p {
            Projected::Beacon(id) => self.type_name(*id).to_string(),
            Projected::Module {
                type_id, quantity, ..
            } if *quantity > 1 => {
                format!("{} x{quantity}", self.type_name(*type_id))
            }
            Projected::Module { type_id, .. } => self.type_name(*type_id).to_string(),
            Projected::Fit { name, .. } => format!("fit: {name}"),
        }
    }

    fn adopt_pilot(&mut self, pilot: rufa_data::esi::Pilot) -> Task<Message> {
        let index = match self.characters.iter().position(|c| c.name == pilot.name) {
            Some(i) => {
                self.characters[i].skills = pilot.skills;
                i
            }
            None => {
                let mut c = Character::new(pilot.name.clone());
                c.skills = pilot.skills;
                self.characters.push(c);
                self.characters.len() - 1
            }
        };
        if let Err(e) = rufa_store::save_character(&self.config_dir, &mut self.characters[index]) {
            self.status = format!("Cannot save character: {e}");
            return Task::none();
        }
        self.status = format!("Skills of {} imported from ESI", pilot.name);
        self.update(Message::ChooseCharacter(CharChoice::Saved(index)))
    }

    fn pane_ratios(&self) -> (f32, f32) {
        use pane_grid::Node;
        match self.panes.layout() {
            Node::Split { ratio, b, .. } => {
                let inner = match b.as_ref() {
                    Node::Split { ratio, .. } => *ratio,
                    Node::Pane(_) => self.settings.fit_ratio,
                };
                (*ratio, inner)
            }
            Node::Pane(_) => (self.settings.browser_ratio, self.settings.fit_ratio),
        }
    }

    pub fn graph_target(&self) -> rufa_sim::graph::Target {
        let signature = self
            .graph_signature
            .trim()
            .parse()
            .ok()
            .filter(|v: &f64| *v > 0.0);
        let velocity = self
            .graph_velocity
            .trim()
            .parse()
            .ok()
            .filter(|v: &f64| *v >= 0.0);
        rufa_sim::graph::Target {
            signature: signature.unwrap_or(f64::INFINITY),
            velocity: velocity.unwrap_or(0.0),
            resists: self
                .targets
                .get(self.target)
                .map(|p| p.resists)
                .unwrap_or([0.0; 4]),
        }
    }

    pub fn describe_violation(&self, v: &rufa_sim::Violation) -> String {
        use rufa_sim::Target;
        let target = match v.target {
            Target::Ship => self
                .type_name(self.active_fit().map(|f| f.ship.type_id).unwrap_or(0))
                .to_string(),
            Target::Item { index } | Target::Charge { index } => self
                .active_fit()
                .and_then(|f| f.items.get(index))
                .map(|it| self.type_name(it.type_id).to_string())
                .unwrap_or_default(),
        };
        let rule = edit::rule_text(&v.rule);
        format!("{target}: {rule}")
    }

    fn active_fit(&self) -> Option<&Fit> {
        self.docs.get(self.active).map(|d| &d.doc.fit)
    }

    fn max_state(&self, i: usize) -> State {
        self.docs
            .get(self.active)
            .and_then(|d| d.doc.calc.items.get(i))
            .map(|r| r.max_state)
            .unwrap_or(State::Offline)
    }

    fn on_key(&mut self, event: keyboard::Event) -> Task<Message> {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
            return Task::none();
        };
        if !modifiers.contains(Modifiers::CTRL) {
            if let (Key::Named(keyboard::key::Named::Delete), Some(i)) = (&key, self.selected_item)
            {
                return self.update(Message::RemoveItem(i));
            }
            return Task::none();
        }
        let Key::Character(c) = &key else {
            return Task::none();
        };
        let message = match c.as_str() {
            "z" => Message::Undo,
            "y" => Message::Redo,
            "s" => Message::Save,
            "c" => Message::CopyEft,
            "v" => Message::PasteEft,
            "w" => Message::CloseTab(self.active),
            _ => return Task::none(),
        };
        self.update(message)
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().map(Message::Key)
    }

    fn view(&self) -> Element<'_, Message> {
        view::root(self)
    }
}

fn looks_like_dna(text: &str) -> bool {
    let body = text.strip_prefix("fitting:").unwrap_or(text);
    !body.contains('\n')
        && body.contains(':')
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || c == ':' || c == ';' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Option<App> {
        std::env::var_os("RUFA_DATA_DIR")?;
        let (mut app, _) = App::boot();
        app.game.as_ref().ok()?;
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        app.config_dir = std::env::temp_dir().join(format!("rufa-test-{}-{n}", std::process::id()));
        app.settings.skill_level = 5;
        Some(app)
    }

    fn type_id(app: &App, name: &str) -> i32 {
        app.game
            .as_ref()
            .unwrap()
            .sde
            .type_name_to_id(name)
            .expect(name)
    }

    #[test]
    fn fit_a_rifter_through_messages() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let dcu = type_id(&app, "Damage Control II");
        let ac = type_id(&app, "200mm AutoCannon II");
        let hobgoblin = type_id(&app, "Hobgoblin II");

        let _ = app.update(Message::NewFit(rifter));
        assert_eq!(app.docs.len(), 1);
        let base_cpu = app.docs[0].doc.stats.resources.cpu_used;

        let _ = app.update(Message::AddItem(dcu));
        let _ = app.update(Message::AddItem(ac));
        let _ = app.update(Message::AddItem(ac));
        let _ = app.update(Message::AddItem(hobgoblin));
        let _ = app.update(Message::AddItem(hobgoblin));
        let emp = type_id(&app, "EMP S");
        let _ = app.update(Message::AddItem(emp));
        let doc = &app.docs[0].doc;
        assert_eq!(doc.fit.items.len(), 4, "{:?}", doc.fit.items);
        assert!(doc.fit.items[1].charge.is_some() && doc.fit.items[2].charge.is_some());
        assert!(doc.fit.items[0].charge.is_none());
        assert!(doc.stats.resources.cpu_used > base_cpu);
        assert!(doc.stats.offense.dps > 0.0);
        assert!(
            doc.stats.offense.drone_dps > 0.0,
            "drones {:?}",
            doc.fit.items[3]
        );
        assert_eq!(doc.fit.items[3].quantity, 2);

        let _ = app.update(Message::CycleState(1));
        assert_eq!(app.docs[0].doc.fit.items[1].state, State::Online);
        let dps_online = app.docs[0].doc.stats.offense.dps;
        let _ = app.update(Message::Undo);
        assert_eq!(app.docs[0].doc.fit.items[1].state, State::Active);
        assert!(app.docs[0].doc.stats.offense.dps > dps_online);
        let _ = app.update(Message::Redo);
        assert_eq!(app.docs[0].doc.fit.items[1].state, State::Online);

        let _ = app.update(Message::RemoveItem(0));
        assert_eq!(app.docs[0].doc.fit.items.len(), 3);

        let _ = app.update(Message::SkillLevel(0));
        assert!(app.docs[0].doc.stats.offense.dps < dps_online);
    }

    #[test]
    fn view_builds_with_a_selected_item_and_profile_changes_ehp() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let dcu = type_id(&app, "Damage Control II");
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(dcu));
        let _ = app.update(Message::SelectItem(Some(0)));
        let _ = app.view();
        let _ = app.update(Message::CenterTab(CenterTab::Graph));
        let _ = app.update(Message::GraphSignature("35".to_string()));
        let _ = app.view();
        let _ = app.update(Message::GraphKind(GraphKind::CapOverTime));
        let _ = app.view();
        let _ = app.update(Message::CenterTab(CenterTab::Fit));
        let uniform = app.docs[0].doc.stats.defense.ehp;
        let _ = app.update(Message::DamageProfile(DamagePreset::Em));
        let em = app.docs[0].doc.stats.defense.ehp;
        assert!(em > uniform, "em {em} uniform {uniform}");
        assert_eq!(
            DamagePreset::of(&app.docs[0].doc.fit.environment.damage_profile),
            Some(DamagePreset::Em)
        );
    }

    #[test]
    fn market_search_finds_modules() {
        let Some(mut app) = app() else { return };
        let _ = app.update(Message::MarketSearch("damage control".to_string()));
        assert!(app.market_results.len() >= 3, "{:?}", app.market_results);
        let names: Vec<&str> = app
            .market_results
            .iter()
            .map(|id| app.type_name(*id))
            .collect();
        assert!(names.contains(&"Damage Control II"), "{names:?}");
    }

    #[test]
    fn saved_character_skills_drive_the_fit() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let ac = type_id(&app, "200mm AutoCannon II");
        let emp = type_id(&app, "EMP S");
        let gunnery = type_id(&app, "Gunnery");
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(ac));
        let _ = app.update(Message::AddItem(emp));
        let all5 = app.docs[0].doc.stats.offense.dps;

        let _ = app.update(Message::NewCharacter);
        assert_eq!(app.current, CharChoice::Saved(0));
        let none = app.docs[0].doc.stats.offense.dps;
        assert!(none < all5, "no skills {none} vs all5 {all5}");
        let _ = app.update(Message::AllSkillsTo(5));
        assert!((app.docs[0].doc.stats.offense.dps - all5).abs() < 1e-6);
        let _ = app.update(Message::SetSkill(gunnery, 3));
        let gunnery3 = app.docs[0].doc.stats.offense.dps;
        assert!(gunnery3 < all5 && gunnery3 > 0.0, "{gunnery3} vs {all5}");

        let reloaded = rufa_store::list_characters(&app.config_dir).unwrap();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].skills.get(&gunnery), Some(&3));

        let _ = app.update(Message::DeleteCharacter);
        assert_eq!(app.current, CharChoice::All(5));
        assert!(
            rufa_store::list_characters(&app.config_dir)
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn projections_change_the_target() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let web = type_id(&app, "Stasis Webifier II");
        let _ = app.update(Message::NewFit(rifter));
        let speed = app.docs[0].doc.stats.navigation.max_velocity;

        let _ = app.update(Message::ProjectMode(true));
        let _ = app.update(Message::AddItem(web));
        assert_eq!(
            app.docs[0].doc.fit.items.len(),
            0,
            "project mode must not fit the web"
        );
        assert_eq!(app.docs[0].doc.projections.len(), 1);
        let webbed = app.docs[0].doc.stats.navigation.max_velocity;
        assert!(webbed < speed * 0.6, "webbed {webbed} vs {speed}");
        let _ = app.update(Message::RemoveProjection(0));
        assert!((app.docs[0].doc.stats.navigation.max_velocity - speed).abs() < 1e-6);
        let _ = app.update(Message::ProjectMode(false));

        let env = app
            .game
            .as_ref()
            .unwrap()
            .catalog
            .environments()
            .iter()
            .find(|e| e.name == "Class 6 Black Hole Effects")
            .unwrap();
        let _ = app.update(Message::Environment(EnvChoice {
            id: env.id,
            name: env.name.clone(),
        }));
        let in_hole = app.docs[0].doc.stats.navigation.max_velocity;
        assert!(
            in_hole > speed * 1.3,
            "black hole speed {in_hole} vs {speed}"
        );
        let _ = app.update(Message::Environment(EnvChoice {
            id: 0,
            name: "None".into(),
        }));
        assert!((app.docs[0].doc.stats.navigation.max_velocity - speed).abs() < 1e-6);

        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(web));
        let _ = app.update(Message::SelectTab(0));
        let _ = app.update(Message::ProjectFit(1));
        assert!(app.docs[0].doc.stats.navigation.max_velocity < speed * 0.6);
    }

    #[test]
    fn tactical_modes_and_variations() {
        let Some(mut app) = app() else { return };
        let game: &'static Game = app.game.as_ref().unwrap();
        let confessor = type_id(&app, "Confessor");
        let defense = type_id(&app, "Confessor Defense Mode");
        let _ = app.update(Message::NewFit(confessor));
        let plain = app.docs[0].doc.stats.defense.ehp;
        let _ = app.update(Message::SetMode(Some(defense)));
        assert!(app.docs[0].doc.stats.defense.ehp > plain);
        let modes: Vec<i32> = game
            .sde
            .get_type(confessor)
            .unwrap()
            .mode_type_ids()
            .into_iter()
            .flatten()
            .collect();
        assert!(modes.contains(&defense));

        let dcu = type_id(&app, "Damage Control II");
        let ty = game.sde.get_type(dcu).unwrap();
        let variants = game.catalog.variations(ty.market_group_id(), ty.group_id());
        assert!(
            variants.iter().any(|v| v.name == "Damage Control I"),
            "{variants:?}"
        );
        let _ = app.update(Message::AddItem(dcu));
        let dcu1 = type_id(&app, "Damage Control I");
        let _ = app.update(Message::ReplaceItem(0, dcu1));
        assert_eq!(app.docs[0].doc.fit.items[0].type_id, dcu1);
    }

    #[test]
    fn charge_list_of_an_autocannon_has_small_projectile_ammo() {
        let Some(app) = app() else { return };
        let game: &'static Game = app.game.as_ref().unwrap();
        let ids = app.ids.unwrap();
        let ac = type_id(&app, "200mm AutoCannon II");
        let groups = rufa_sim::place::charge_groups(&game.info(), &ids, ac);
        assert!(!groups.is_empty());
        let names: Vec<&str> = groups
            .iter()
            .flat_map(|g| game.catalog.charges_in_group(*g))
            .filter(|c| rufa_sim::place::accepts_charge(&game.info(), &ids, ac, c.id))
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"EMP S"), "{names:?}");
        assert!(!names.contains(&"EMP M"), "{names:?}");
    }

    #[test]
    fn mutations_side_effects_and_abilities() {
        let Some(mut app) = app() else { return };
        let game: &'static Game = app.game.as_ref().unwrap();
        let rifter = type_id(&app, "Rifter");
        let web = type_id(&app, "Stasis Webifier II");
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(web));
        let plasmids = edit::mutaplasmids_for(&game.sde, web);
        assert!(!plasmids.is_empty(), "no mutaplasmids for a T2 web");
        let decayed = *plasmids
            .iter()
            .find(|p| game.type_name(**p).starts_with("Decayed"))
            .unwrap_or(&plasmids[0]);
        let ranges = edit::mutation_ranges(&game.sde, &game.info(), decayed, web);
        assert!(!ranges.is_empty());
        let _ = app.update(Message::Mutate(0, decayed));
        let item = &app.docs[0].doc.fit.items[0];
        assert_ne!(item.type_id, web);
        assert_eq!(item.mutation.as_ref().map(|m| m.base), Some(web));
        let speed_factor = ranges
            .iter()
            .find(|r| {
                game.catalog
                    .attr_label(r.attribute_id)
                    .map(|l| l.name.as_str())
                    == Some("speedFactor")
            })
            .expect("web strength rolls");
        let _ = app.update(Message::MutationValue(
            0,
            speed_factor.attribute_id,
            speed_factor.max,
        ));
        let strongest = app.docs[0].doc.calc.items[0].attributes[&speed_factor.attribute_id].value;
        assert!(
            (strongest - speed_factor.max).abs() < 1e-6,
            "{strongest} vs {}",
            speed_factor.max
        );
        let _ = app.update(Message::SelectItem(Some(0)));
        let _ = app.view();
        let _ = app.update(Message::RevertMutation(0));
        assert_eq!(app.docs[0].doc.fit.items[0].type_id, web);

        let booster = type_id(&app, "Standard Blue Pill Booster");
        let _ = app.update(Message::AddItem(booster));
        let effects = edit::side_effects(&game.info(), booster);
        assert!(!effects.is_empty(), "blue pill has side effects");
        let before = app.docs[0].doc.stats.resources.cpu_total;
        let i = app.docs[0].doc.fit.items.len() - 1;
        for (effect, _) in &effects {
            let _ = app.update(Message::ToggleSideEffect(i, *effect));
        }
        assert_eq!(
            app.docs[0].doc.fit.items[i].booster_side_effects.len(),
            effects.len()
        );
        let _ = before;

        let thanatos = type_id(&app, "Thanatos");
        let einherji = type_id(&app, "Einherji II");
        let _ = app.update(Message::NewFit(thanatos));
        let _ = app.update(Message::AddItem(einherji));
        let abilities = edit::fighter_abilities(&game.info(), einherji);
        assert!(abilities.len() >= 2, "{abilities:?}");
        let dps_default = app.docs[1].doc.stats.offense.fighter_dps;
        assert!(dps_default > 0.0);
        let (attack, _, _) = abilities
            .iter()
            .find(|(_, name, _)| {
                name.to_lowercase().contains("attack") || name.to_lowercase().contains("turret")
            })
            .cloned()
            .unwrap_or(abilities[0].clone());
        let _ = app.update(Message::ToggleAbility(0, attack));
        assert!(app.docs[1].doc.fit.items[0].fighter_abilities.is_some());
    }

    #[test]
    fn prices_and_target_profiles() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let ac = type_id(&app, "200mm AutoCannon II");
        let emp = type_id(&app, "EMP S");
        app.prices = PriceCache::default();
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(ac));
        let _ = app.update(Message::AddItem(emp));
        assert!(app.prices_pending, "a fetch starts for the stale types");
        assert_eq!(app.fit_price(&app.docs[0]), None);
        let mut prices = std::collections::BTreeMap::new();
        prices.insert(rifter, 1_000_000.0);
        prices.insert(ac, 500_000.0);
        prices.insert(emp, 100.0);
        let _ = app.update(Message::PricesFetched(Ok(prices)));
        assert!(!app.prices_pending);
        assert_eq!(app.fit_price(&app.docs[0]), Some(1_500_100.0));
        assert!(app.stale_price_types().is_empty());
        let reloaded = PriceCache::load(&app.config_dir);
        assert_eq!(reloaded.get(ac), Some(500_000.0));

        let full = app.docs[0].doc.stats.offense.dps;
        let i = app
            .targets
            .iter()
            .position(|p| p.name == "Battleship")
            .unwrap();
        let _ = app.update(Message::TargetProfile(i));
        let target = app.graph_target();
        assert_eq!(target.resists, [0.3; 4]);
        let applied = rufa_sim::graph::dps_at(
            &app.ids.unwrap(),
            &app.docs[0].doc.fit,
            &app.docs[0].doc.calc,
            0.0,
            &target,
        );
        assert!((applied - full * 0.7).abs() < 1e-6, "{applied} vs {full}");
        let _ = app.view();
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn implant_sets_save_and_apply() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let implant = type_id(&app, "High-grade Snake Alpha");
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(implant));
        let boosted = app.docs[0].doc.stats.navigation.max_velocity;
        let _ = app.update(Message::SaveImplantSet);
        assert_eq!(app.implant_sets.len(), 1);
        assert_eq!(rufa_store::implants::load(&app.config_dir).len(), 1);
        let _ = app.update(Message::NewFit(rifter));
        let plain = app.docs[1].doc.stats.navigation.max_velocity;
        assert!(plain < boosted);
        let _ = app.update(Message::ApplyImplantSet(0));
        assert!((app.docs[1].doc.stats.navigation.max_velocity - boosted).abs() < 1e-6);
        let _ = app.update(Message::ApplyImplantSet(0));
        assert_eq!(
            app.docs[1].doc.fit.items.len(),
            1,
            "applying twice keeps one implant per slot"
        );
        let _ = app.update(Message::DeleteImplantSet(0));
        assert!(app.implant_sets.is_empty());
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn dna_paste_opens_a_fit() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let dcu = type_id(&app, "Damage Control II");
        let ac = type_id(&app, "200mm AutoCannon II");
        assert!(looks_like_dna("587:2048;1:2889;3::"));
        assert!(!looks_like_dna("[Rifter, x]\nDamage Control II"));
        let dna = format!("{rifter}:{dcu};1:{ac};3::");
        let _ = app.update(Message::ImportText(Some(dna)));
        assert_eq!(app.docs.len(), 1, "{}", app.status);
        assert_eq!(app.docs[0].doc.fit.items.len(), 4);
    }

    #[test]
    fn target_profile_editor_saves_and_deletes() {
        let Some(mut app) = app() else { return };
        let before = app.targets.len();
        let _ = app.update(Message::TargetName("Kiter".to_string()));
        let _ = app.update(Message::TargetResist(0, "50%".to_string()));
        let _ = app.update(Message::TargetResist(1, "0.25".to_string()));
        let _ = app.update(Message::GraphSignature("40".to_string()));
        let _ = app.update(Message::GraphVelocity("2000".to_string()));
        let _ = app.update(Message::SaveTargetProfile);
        assert_eq!(app.targets.len(), before + 1);
        let saved = &app.targets[app.target];
        assert_eq!(saved.name, "Kiter");
        assert_eq!(saved.resists, [0.5, 0.25, 0.0, 0.0]);
        assert_eq!(saved.signature, 40.0);
        let reloaded = rufa_store::targets::load(&app.config_dir);
        assert!(reloaded.iter().any(|p| p.name == "Kiter"));
        let _ = app.update(Message::DeleteTargetProfile(0));
        assert_eq!(app.targets.len(), before + 1, "built-in profiles stay");
        let i = app.targets.iter().position(|p| p.name == "Kiter").unwrap();
        let _ = app.update(Message::DeleteTargetProfile(i));
        assert_eq!(app.targets.len(), before);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn an_esi_pilot_becomes_a_character() {
        let Some(mut app) = app() else { return };
        let gunnery = type_id(&app, "Gunnery");
        let mut skills = std::collections::BTreeMap::new();
        skills.insert(gunnery, 4u8);
        let pilot = rufa_data::esi::Pilot {
            character_id: 1,
            name: "Test Pilot".to_string(),
            skills: skills.clone(),
        };
        let _ = app.update(Message::EsiResult(Ok(pilot.clone())));
        assert_eq!(app.characters.len(), 1);
        assert_eq!(app.current, CharChoice::Saved(0));
        assert_eq!(app.characters[0].skills, skills);
        let _ = app.update(Message::EsiResult(Ok(pilot)));
        assert_eq!(app.characters.len(), 1, "same pilot updates in place");
        assert_eq!(
            rufa_store::list_characters(&app.config_dir).unwrap().len(),
            1
        );
        let _ = app.update(Message::EsiLogin);
        assert!(app.status.contains("client id"), "{}", app.status);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn modules_the_hull_rejects_are_refused() {
        let Some(mut app) = app() else { return };
        let kestrel = type_id(&app, "Kestrel");
        let ac = type_id(&app, "200mm AutoCannon II");
        let large_rig = type_id(&app, "Large Trimark Armor Pump I");
        let capital = type_id(&app, "Capital Armor Repairer I");
        let launcher = type_id(&app, "Rocket Launcher II");
        let _ = app.update(Message::NewFit(kestrel));
        for _ in 0..4 {
            let _ = app.update(Message::AddItem(launcher));
        }
        assert_eq!(app.docs[0].doc.fit.items.len(), 4, "{}", app.status);
        let _ = app.update(Message::NewFit(kestrel));
        let _ = app.update(Message::AddItem(ac));
        assert_eq!(
            app.docs[1].doc.fit.items.len(),
            0,
            "no turret hardpoints: {}",
            app.status
        );
        assert!(app.status.contains("does not fit"), "{}", app.status);
        let _ = app.update(Message::AddItem(large_rig));
        assert_eq!(app.docs[1].doc.fit.items.len(), 0, "{}", app.status);
        let _ = app.update(Message::AddItem(capital));
        assert_eq!(app.docs[1].doc.fit.items.len(), 0, "{}", app.status);
        let bcs = type_id(&app, "Ballistic Control System II");
        for _ in 0..2 {
            let _ = app.update(Message::AddItem(bcs));
        }
        let _ = app.update(Message::AddItem(launcher));
        let _ = app.update(Message::AddItem(launcher));
        let _ = app.update(Message::AddItem(launcher));
        let _ = app.update(Message::AddItem(launcher));
        assert_eq!(
            app.docs[1].doc.fit.items.len(),
            6,
            "over CPU is allowed, only shown: {}",
            app.status
        );
    }

    #[test]
    fn rack_full_is_reported_not_applied() {
        let Some(mut app) = app() else { return };
        let rifter = type_id(&app, "Rifter");
        let dcu = type_id(&app, "Damage Control II");
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(dcu));
        let _ = app.update(Message::AddItem(dcu));
        assert_eq!(app.docs[0].doc.fit.items.len(), 1);
        assert!(
            app.status.contains("too many of this group"),
            "{}",
            app.status
        );
        let gyro = type_id(&app, "Gyrostabilizer II");
        for _ in 0..5 {
            let _ = app.update(Message::AddItem(gyro));
        }
        assert_eq!(app.docs[0].doc.fit.items.len(), 4);
        assert!(app.status.contains("no free Low slot"), "{}", app.status);
    }

    #[test]
    fn save_and_reopen_round_trip() {
        let Some(mut app) = app() else { return };
        let dir = app.config_dir.clone();
        let rifter = type_id(&app, "Rifter");
        let ac = type_id(&app, "200mm AutoCannon II");
        let _ = app.update(Message::NewFit(rifter));
        let _ = app.update(Message::AddItem(ac));
        let _ = app.update(Message::RenameFit("round trip".to_string()));
        let _ = app.update(Message::Save);
        assert_eq!(app.saved.len(), 1, "{}", app.status);
        let path = app.saved[0].path.clone();
        let _ = app.update(Message::CloseTab(0));
        let _ = app.update(Message::OpenSaved(path));
        assert_eq!(app.docs.len(), 1);
        assert_eq!(app.docs[0].doc.fit.name.as_deref(), Some("round trip"));
        assert_eq!(app.docs[0].doc.fit.items.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
