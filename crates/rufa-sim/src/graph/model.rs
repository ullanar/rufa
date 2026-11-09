//! What each graph plots, reads and offers. Pyfa's definitions, as a table.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Damage,
    Application,
    Ewar,
    Reps,
    Shield,
    Cap,
    Mobility,
    Warp,
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    None,
    Km,
    M,
    S,
    Percent,
    Gj,
    GjPerS,
    Hp,
    HpPerS,
    MPerS,
    Au,
    Mm,
    Mkg,
    GkgMPerS,
    Degrees,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::None => "",
            Unit::Km => "km",
            Unit::M => "m",
            Unit::S => "s",
            Unit::Percent => "%",
            Unit::Gj => "GJ",
            Unit::GjPerS => "GJ/s",
            Unit::Hp => "HP",
            Unit::HpPerS => "HP/s",
            Unit::MPerS => "m/s",
            Unit::Au => "AU",
            Unit::Mm => "mm",
            Unit::Mkg => "Mkg",
            Unit::GkgMPerS => "Gkg m/s",
            Unit::Degrees => "deg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quantity {
    Distance,
    Time,
    TgtSpeed,
    TgtSigRad,
    Dps,
    Volley,
    Damage,
    NeutStr,
    WebStr,
    EcmStr,
    DampStr,
    TdStr,
    GdStr,
    TpStr,
    Rps,
    TotalRep,
    ShieldAmount,
    ShieldRegen,
    CapAmount,
    CapRegen,
    Speed,
    Travelled,
    Momentum,
    BumpSpeed,
    BumpDistance,
    WarpTime,
    LockTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputKey {
    Distance,
    Time,
    TgtSpeed,
    TgtSigRad,
    CapAmount,
    CapAmountT0,
    ShieldAmount,
    ShieldAmountT0,
    Resist,
    TgtMass,
    TgtInertia,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XAxis {
    pub q: Quantity,
    pub unit: Unit,
    pub label: &'static str,
    pub main: InputKey,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YAxis {
    pub q: Quantity,
    pub unit: Unit,
    pub label: &'static str,
    pub selector: &'static str,
}

/// When an input is read, as a condition on the chosen axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum When {
    Always,
    X(&'static [(Quantity, Unit)]),
    Y(&'static [Quantity]),
    Xy((Quantity, Unit), Quantity),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputDef {
    pub key: InputKey,
    pub unit: Unit,
    pub label: &'static str,
    /// What a single value box starts with. None starts empty.
    pub default: Option<f64>,
    pub range: (f64, f64),
    /// What an empty box means, shown as the placeholder.
    pub blank: Option<&'static str>,
    pub when: When,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Toggle {
    Resists,
    Projected,
    LockRange,
    DroneRange,
    DroneMode,
    Capsim,
    AncillaryReload,
    Ehp,
    AmmoTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DroneMode {
    #[default]
    Auto,
    FollowTarget,
    FollowAttacker,
}

impl DroneMode {
    pub const ALL: [DroneMode; 3] = [
        DroneMode::Auto,
        DroneMode::FollowTarget,
        DroneMode::FollowAttacker,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DroneMode::Auto => "Auto",
            DroneMode::FollowTarget => "Follow target",
            DroneMode::FollowAttacker => "Stay with attacker",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AmmoTier {
    T1,
    #[default]
    Navy,
    All,
}

impl AmmoTier {
    pub const ALL: [AmmoTier; 3] = [AmmoTier::T1, AmmoTier::Navy, AmmoTier::All];

    pub fn label(self) -> &'static str {
        match self {
            AmmoTier::T1 => "T1",
            AmmoTier::Navy => "Navy",
            AmmoTier::All => "All",
        }
    }
}

/// The toggles a graph reads. Positive wording: on means applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub resists: bool,
    pub projected: bool,
    pub lock_range: bool,
    pub drone_range: bool,
    pub drone_mode: DroneMode,
    pub capsim: bool,
    pub ancillary_reload: bool,
    pub ehp: bool,
    pub ammo_tier: AmmoTier,
    /// Fold reload time into steady figures, the fit's own setting.
    pub reload: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            resists: false,
            projected: true,
            lock_range: false,
            drone_range: true,
            drone_mode: DroneMode::Auto,
            capsim: true,
            ancillary_reload: true,
            ehp: false,
            ammo_tier: AmmoTier::Navy,
            reload: false,
        }
    }
}

/// A speed as a share of the top speed and a heading in degrees. Zero
/// degrees points at the other ship, ninety is straight across.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub length: f64,
    pub angle: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vectors {
    pub attacker: Vector,
    pub target: Vector,
}

impl Default for Vectors {
    fn default() -> Vectors {
        Vectors {
            attacker: Vector {
                length: 0.0,
                angle: 90.0,
            },
            target: Vector {
                length: 1.0,
                angle: 90.0,
            },
        }
    }
}

const D: Quantity = Quantity::Distance;
const T: Quantity = Quantity::Time;

const DAMAGE_X: [XAxis; 6] = [
    XAxis {
        q: D,
        unit: Unit::Km,
        label: "Distance",
        main: InputKey::Distance,
    },
    XAxis {
        q: T,
        unit: Unit::S,
        label: "Time",
        main: InputKey::Time,
    },
    XAxis {
        q: Quantity::TgtSpeed,
        unit: Unit::MPerS,
        label: "Target speed",
        main: InputKey::TgtSpeed,
    },
    XAxis {
        q: Quantity::TgtSpeed,
        unit: Unit::Percent,
        label: "Target speed",
        main: InputKey::TgtSpeed,
    },
    XAxis {
        q: Quantity::TgtSigRad,
        unit: Unit::M,
        label: "Target signature radius",
        main: InputKey::TgtSigRad,
    },
    XAxis {
        q: Quantity::TgtSigRad,
        unit: Unit::Percent,
        label: "Target signature radius",
        main: InputKey::TgtSigRad,
    },
];
const DAMAGE_Y: [YAxis; 3] = [
    YAxis {
        q: Quantity::Dps,
        unit: Unit::None,
        label: "DPS",
        selector: "DPS",
    },
    YAxis {
        q: Quantity::Volley,
        unit: Unit::None,
        label: "Volley",
        selector: "Volley",
    },
    YAxis {
        q: Quantity::Damage,
        unit: Unit::None,
        label: "Damage inflicted",
        selector: "Damage inflicted",
    },
];
const DAMAGE_INPUTS: [InputDef; 4] = [
    InputDef {
        key: InputKey::Distance,
        unit: Unit::Km,
        label: "Distance",
        default: None,
        range: (0.0, 100.0),
        blank: Some("any"),
        when: When::Always,
    },
    InputDef {
        key: InputKey::Time,
        unit: Unit::S,
        label: "Time",
        default: None,
        range: (0.0, 80.0),
        blank: Some("steady"),
        when: When::Always,
    },
    InputDef {
        key: InputKey::TgtSpeed,
        unit: Unit::Percent,
        label: "Target speed",
        default: Some(100.0),
        range: (0.0, 100.0),
        blank: None,
        when: When::X(&[
            (Quantity::TgtSpeed, Unit::MPerS),
            (Quantity::TgtSpeed, Unit::Percent),
        ]),
    },
    InputDef {
        key: InputKey::TgtSigRad,
        unit: Unit::Percent,
        label: "Target signature",
        default: Some(100.0),
        range: (100.0, 200.0),
        blank: None,
        when: When::X(&[
            (Quantity::TgtSigRad, Unit::M),
            (Quantity::TgtSigRad, Unit::Percent),
        ]),
    },
];
const DAMAGE_TOGGLES: [Toggle; 5] = [
    Toggle::Resists,
    Toggle::Projected,
    Toggle::LockRange,
    Toggle::DroneRange,
    Toggle::DroneMode,
];

const APPLICATION_X: [XAxis; 1] = [XAxis {
    q: D,
    unit: Unit::Km,
    label: "Distance",
    main: InputKey::Distance,
}];
const APPLICATION_Y: [YAxis; 2] = [
    YAxis {
        q: Quantity::Dps,
        unit: Unit::None,
        label: "DPS",
        selector: "DPS",
    },
    YAxis {
        q: Quantity::Volley,
        unit: Unit::None,
        label: "Volley",
        selector: "Volley",
    },
];
const APPLICATION_INPUTS: [InputDef; 1] = [InputDef {
    key: InputKey::Distance,
    unit: Unit::Km,
    label: "Distance",
    default: None,
    range: (0.0, 100.0),
    blank: None,
    when: When::Always,
}];
const APPLICATION_TOGGLES: [Toggle; 3] = [Toggle::Resists, Toggle::Projected, Toggle::AmmoTier];

const EWAR_X: [XAxis; 1] = [XAxis {
    q: D,
    unit: Unit::Km,
    label: "Distance",
    main: InputKey::Distance,
}];
const EWAR_Y: [YAxis; 7] = [
    YAxis {
        q: Quantity::NeutStr,
        unit: Unit::GjPerS,
        label: "Cap neutralized per second",
        selector: "Neuts",
    },
    YAxis {
        q: Quantity::WebStr,
        unit: Unit::Percent,
        label: "Speed reduction",
        selector: "Webs",
    },
    YAxis {
        q: Quantity::EcmStr,
        unit: Unit::None,
        label: "Combined ECM strength",
        selector: "ECM",
    },
    YAxis {
        q: Quantity::DampStr,
        unit: Unit::Percent,
        label: "Lock range reduction",
        selector: "Damps",
    },
    YAxis {
        q: Quantity::TdStr,
        unit: Unit::Percent,
        label: "Turret optimal range reduction",
        selector: "Tracking disruptors",
    },
    YAxis {
        q: Quantity::GdStr,
        unit: Unit::Percent,
        label: "Missile flight range reduction",
        selector: "Guidance disruptors",
    },
    YAxis {
        q: Quantity::TpStr,
        unit: Unit::Percent,
        label: "Signature radius increase",
        selector: "Target painters",
    },
];
const EWAR_INPUTS: [InputDef; 2] = [
    InputDef {
        key: InputKey::Distance,
        unit: Unit::Km,
        label: "Distance",
        default: None,
        range: (0.0, 100.0),
        blank: None,
        when: When::Always,
    },
    InputDef {
        key: InputKey::Resist,
        unit: Unit::Percent,
        label: "Target resistance",
        default: Some(0.0),
        range: (0.0, 100.0),
        blank: None,
        when: When::Always,
    },
];
const EWAR_TOGGLES: [Toggle; 2] = [Toggle::LockRange, Toggle::DroneRange];

const REPS_X: [XAxis; 2] = [
    XAxis {
        q: D,
        unit: Unit::Km,
        label: "Distance",
        main: InputKey::Distance,
    },
    XAxis {
        q: T,
        unit: Unit::S,
        label: "Time",
        main: InputKey::Time,
    },
];
const REPS_Y: [YAxis; 2] = [
    YAxis {
        q: Quantity::Rps,
        unit: Unit::HpPerS,
        label: "Repair speed",
        selector: "Repair speed",
    },
    YAxis {
        q: Quantity::TotalRep,
        unit: Unit::Hp,
        label: "Total repaired",
        selector: "Total repaired",
    },
];
const REPS_INPUTS: [InputDef; 2] = [
    InputDef {
        key: InputKey::Distance,
        unit: Unit::Km,
        label: "Distance",
        default: None,
        range: (0.0, 100.0),
        blank: Some("any"),
        when: When::Always,
    },
    InputDef {
        key: InputKey::Time,
        unit: Unit::S,
        label: "Time",
        default: None,
        range: (0.0, 80.0),
        blank: Some("steady"),
        when: When::Always,
    },
];
const REPS_TOGGLES: [Toggle; 3] = [
    Toggle::LockRange,
    Toggle::DroneRange,
    Toggle::AncillaryReload,
];

const SHIELD_X: [XAxis; 3] = [
    XAxis {
        q: T,
        unit: Unit::S,
        label: "Time",
        main: InputKey::Time,
    },
    XAxis {
        q: Quantity::ShieldAmount,
        unit: Unit::Hp,
        label: "Shield amount",
        main: InputKey::ShieldAmount,
    },
    XAxis {
        q: Quantity::ShieldAmount,
        unit: Unit::Percent,
        label: "Shield amount",
        main: InputKey::ShieldAmount,
    },
];
const SHIELD_Y: [YAxis; 2] = [
    YAxis {
        q: Quantity::ShieldAmount,
        unit: Unit::Hp,
        label: "Shield amount",
        selector: "Shield amount",
    },
    YAxis {
        q: Quantity::ShieldRegen,
        unit: Unit::HpPerS,
        label: "Shield regen",
        selector: "Shield regen",
    },
];
const SHIELD_INPUTS: [InputDef; 3] = [
    InputDef {
        key: InputKey::Time,
        unit: Unit::S,
        label: "Time",
        default: None,
        range: (0.0, 300.0),
        blank: None,
        when: When::X(&[(T, Unit::S)]),
    },
    InputDef {
        key: InputKey::ShieldAmount,
        unit: Unit::Percent,
        label: "Shield amount",
        default: Some(25.0),
        range: (0.0, 100.0),
        blank: None,
        when: When::X(&[
            (Quantity::ShieldAmount, Unit::Hp),
            (Quantity::ShieldAmount, Unit::Percent),
        ]),
    },
    InputDef {
        key: InputKey::ShieldAmountT0,
        unit: Unit::Percent,
        label: "Starting shield",
        default: Some(0.0),
        range: (0.0, 100.0),
        blank: None,
        when: When::X(&[(T, Unit::S)]),
    },
];
const SHIELD_TOGGLES: [Toggle; 1] = [Toggle::Ehp];

const CAP_X: [XAxis; 3] = [
    XAxis {
        q: T,
        unit: Unit::S,
        label: "Time",
        main: InputKey::Time,
    },
    XAxis {
        q: Quantity::CapAmount,
        unit: Unit::Gj,
        label: "Cap amount",
        main: InputKey::CapAmount,
    },
    XAxis {
        q: Quantity::CapAmount,
        unit: Unit::Percent,
        label: "Cap amount",
        main: InputKey::CapAmount,
    },
];
const CAP_Y: [YAxis; 2] = [
    YAxis {
        q: Quantity::CapAmount,
        unit: Unit::Gj,
        label: "Cap amount",
        selector: "Cap amount",
    },
    YAxis {
        q: Quantity::CapRegen,
        unit: Unit::GjPerS,
        label: "Cap regen",
        selector: "Cap regen",
    },
];
const CAP_INPUTS: [InputDef; 3] = [
    InputDef {
        key: InputKey::Time,
        unit: Unit::S,
        label: "Time",
        default: None,
        range: (0.0, 300.0),
        blank: None,
        when: When::X(&[(T, Unit::S)]),
    },
    InputDef {
        key: InputKey::CapAmount,
        unit: Unit::Percent,
        label: "Cap amount",
        default: Some(25.0),
        range: (0.0, 100.0),
        blank: None,
        when: When::X(&[
            (Quantity::CapAmount, Unit::Gj),
            (Quantity::CapAmount, Unit::Percent),
        ]),
    },
    InputDef {
        key: InputKey::CapAmountT0,
        unit: Unit::Percent,
        label: "Starting cap",
        default: Some(100.0),
        range: (0.0, 100.0),
        blank: None,
        when: When::X(&[(T, Unit::S)]),
    },
];
const CAP_TOGGLES: [Toggle; 1] = [Toggle::Capsim];

const MOBILITY_X: [XAxis; 1] = [XAxis {
    q: T,
    unit: Unit::S,
    label: "Time",
    main: InputKey::Time,
}];
const MOBILITY_Y: [YAxis; 5] = [
    YAxis {
        q: Quantity::Speed,
        unit: Unit::MPerS,
        label: "Speed",
        selector: "Speed",
    },
    YAxis {
        q: Quantity::Travelled,
        unit: Unit::Km,
        label: "Distance",
        selector: "Distance",
    },
    YAxis {
        q: Quantity::Momentum,
        unit: Unit::GkgMPerS,
        label: "Momentum",
        selector: "Momentum",
    },
    YAxis {
        q: Quantity::BumpSpeed,
        unit: Unit::MPerS,
        label: "Target speed",
        selector: "Bump speed",
    },
    YAxis {
        q: Quantity::BumpDistance,
        unit: Unit::Km,
        label: "Target distance travelled",
        selector: "Bump distance",
    },
];
const MOBILITY_INPUTS: [InputDef; 3] = [
    InputDef {
        key: InputKey::Time,
        unit: Unit::S,
        label: "Time",
        default: Some(10.0),
        range: (0.0, 30.0),
        blank: None,
        when: When::Always,
    },
    InputDef {
        key: InputKey::TgtMass,
        unit: Unit::Mkg,
        label: "Target mass",
        default: Some(1300.0),
        range: (100.0, 2500.0),
        blank: None,
        when: When::Y(&[Quantity::BumpSpeed, Quantity::BumpDistance]),
    },
    InputDef {
        key: InputKey::TgtInertia,
        unit: Unit::None,
        label: "Target inertia",
        default: Some(0.015),
        range: (0.03, 0.1),
        blank: None,
        when: When::Y(&[Quantity::BumpDistance]),
    },
];

const WARP_X: [XAxis; 2] = [
    XAxis {
        q: D,
        unit: Unit::Au,
        label: "Distance",
        main: InputKey::Distance,
    },
    XAxis {
        q: D,
        unit: Unit::Km,
        label: "Distance",
        main: InputKey::Distance,
    },
];
const WARP_Y: [YAxis; 1] = [YAxis {
    q: Quantity::WarpTime,
    unit: Unit::S,
    label: "Warp time",
    selector: "Warp time",
}];
const WARP_INPUTS: [InputDef; 1] = [InputDef {
    key: InputKey::Distance,
    unit: Unit::Au,
    label: "Distance",
    default: None,
    range: (0.0, 50.0),
    blank: None,
    when: When::Always,
}];

const LOCK_X: [XAxis; 1] = [XAxis {
    q: Quantity::TgtSigRad,
    unit: Unit::M,
    label: "Target signature radius",
    main: InputKey::TgtSigRad,
}];
const LOCK_Y: [YAxis; 1] = [YAxis {
    q: Quantity::LockTime,
    unit: Unit::S,
    label: "Lock time",
    selector: "Lock time",
}];
const LOCK_INPUTS: [InputDef; 1] = [InputDef {
    key: InputKey::TgtSigRad,
    unit: Unit::M,
    label: "Target signature",
    default: None,
    range: (25.0, 500.0),
    blank: None,
    when: When::Always,
}];

impl Kind {
    pub const ALL: [Kind; 9] = [
        Kind::Damage,
        Kind::Application,
        Kind::Ewar,
        Kind::Reps,
        Kind::Shield,
        Kind::Cap,
        Kind::Mobility,
        Kind::Warp,
        Kind::Lock,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Damage => "Damage",
            Kind::Application => "Application",
            Kind::Ewar => "EWAR",
            Kind::Reps => "Reps",
            Kind::Shield => "Shield",
            Kind::Cap => "Cap",
            Kind::Mobility => "Mobility",
            Kind::Warp => "Warp",
            Kind::Lock => "Lock",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Kind::Damage => "Damage stats",
            Kind::Application => "Application profile",
            Kind::Ewar => "Electronic warfare",
            Kind::Reps => "Remote repairs",
            Kind::Shield => "Shield regeneration",
            Kind::Cap => "Capacitor",
            Kind::Mobility => "Mobility",
            Kind::Warp => "Warp time",
            Kind::Lock => "Lock time",
        }
    }

    pub fn x_axes(self) -> &'static [XAxis] {
        match self {
            Kind::Damage => &DAMAGE_X,
            Kind::Application => &APPLICATION_X,
            Kind::Ewar => &EWAR_X,
            Kind::Reps => &REPS_X,
            Kind::Shield => &SHIELD_X,
            Kind::Cap => &CAP_X,
            Kind::Mobility => &MOBILITY_X,
            Kind::Warp => &WARP_X,
            Kind::Lock => &LOCK_X,
        }
    }

    pub fn y_axes(self) -> &'static [YAxis] {
        match self {
            Kind::Damage => &DAMAGE_Y,
            Kind::Application => &APPLICATION_Y,
            Kind::Ewar => &EWAR_Y,
            Kind::Reps => &REPS_Y,
            Kind::Shield => &SHIELD_Y,
            Kind::Cap => &CAP_Y,
            Kind::Mobility => &MOBILITY_Y,
            Kind::Warp => &WARP_Y,
            Kind::Lock => &LOCK_Y,
        }
    }

    pub fn inputs(self) -> &'static [InputDef] {
        match self {
            Kind::Damage => &DAMAGE_INPUTS,
            Kind::Application => &APPLICATION_INPUTS,
            Kind::Ewar => &EWAR_INPUTS,
            Kind::Reps => &REPS_INPUTS,
            Kind::Shield => &SHIELD_INPUTS,
            Kind::Cap => &CAP_INPUTS,
            Kind::Mobility => &MOBILITY_INPUTS,
            Kind::Warp => &WARP_INPUTS,
            Kind::Lock => &LOCK_INPUTS,
        }
    }

    pub fn toggles(self) -> &'static [Toggle] {
        match self {
            Kind::Damage => &DAMAGE_TOGGLES,
            Kind::Application => &APPLICATION_TOGGLES,
            Kind::Ewar => &EWAR_TOGGLES,
            Kind::Reps => &REPS_TOGGLES,
            Kind::Shield => &SHIELD_TOGGLES,
            Kind::Cap => &CAP_TOGGLES,
            Kind::Mobility | Kind::Warp | Kind::Lock => &[],
        }
    }

    pub fn has_targets(self) -> bool {
        matches!(self, Kind::Damage | Kind::Application)
    }

    pub fn has_vectors(self) -> bool {
        matches!(self, Kind::Damage | Kind::Application)
    }

    /// The input the range box drives for this X axis, with the unit the
    /// range is typed in.
    pub fn main_input(self, x: &XAxis) -> Option<&'static InputDef> {
        self.inputs().iter().find(|i| i.key == x.main)
    }

    /// The inputs shown as single values beside the range box.
    pub fn misc_inputs(self, x: &XAxis, y: &YAxis) -> Vec<&'static InputDef> {
        self.inputs()
            .iter()
            .filter(|i| i.key != x.main && i.when.holds(x, y))
            .filter(|i| !(self.has_vectors() && i.key == InputKey::TgtSpeed))
            .collect()
    }

    /// Which toggles the chosen axes read.
    pub fn toggles_for(self, x: &XAxis, y: &YAxis) -> Vec<Toggle> {
        self.toggles()
            .iter()
            .copied()
            .filter(|t| match t {
                Toggle::Capsim => x.q == Quantity::Time && y.q == Quantity::CapAmount,
                _ => true,
            })
            .collect()
    }
}

impl When {
    pub fn holds(self, x: &XAxis, y: &YAxis) -> bool {
        match self {
            When::Always => true,
            When::X(list) => list.iter().any(|(q, u)| *q == x.q && *u == x.unit),
            When::Y(list) => list.contains(&y.q),
            When::Xy((q, u), yq) => q == x.q && u == x.unit && yq == y.q,
        }
    }
}

impl XAxis {
    pub fn title(&self) -> String {
        match self.unit {
            Unit::None => self.label.to_string(),
            unit => format!("{}, {}", self.label, unit.label()),
        }
    }

    pub fn selector(&self) -> String {
        match self.unit {
            Unit::None => self.label.to_string(),
            unit => format!("{}, {}", self.label, unit.label()),
        }
    }
}

impl YAxis {
    pub fn title(&self, options: &Options) -> String {
        let label = self.label;
        let (label, unit) = match (self.q, options.ehp) {
            (Quantity::ShieldAmount, true) => (label, Unit::None),
            (Quantity::ShieldRegen, true) => (label, Unit::None),
            _ => (label, self.unit),
        };
        let effective = options.resists
            && matches!(self.q, Quantity::Dps | Quantity::Volley | Quantity::Damage);
        let mut out = String::new();
        if effective {
            out.push_str("Effective ");
            out.push_str(&label.to_lowercase());
        } else {
            out.push_str(label);
        }
        match (self.q, options.ehp) {
            (Quantity::ShieldAmount, true) => out.push_str(", EHP"),
            (Quantity::ShieldRegen, true) => out.push_str(", EHP/s"),
            _ => {
                if unit != Unit::None {
                    out.push_str(", ");
                    out.push_str(unit.label());
                }
            }
        }
        out
    }
}

/// What the pane asks the calculation for.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub kind: Kind,
    pub x: usize,
    pub y: usize,
    /// The range in the unit of the X axis.
    pub range: (f64, f64),
    /// Single value inputs in their own display units. A missing key or a
    /// None value is a blank box.
    pub inputs: Vec<(InputKey, Option<f64>)>,
    pub vectors: Vectors,
    pub options: Options,
}

impl Request {
    pub fn new(kind: Kind) -> Request {
        let x = &kind.x_axes()[0];
        let range = kind.main_input(x).map(|i| i.range).unwrap_or((0.0, 1.0));
        Request {
            kind,
            x: 0,
            y: 0,
            range,
            inputs: kind.inputs().iter().map(|i| (i.key, i.default)).collect(),
            vectors: Vectors::default(),
            options: Options::default(),
        }
    }

    pub fn x_axis(&self) -> &'static XAxis {
        &self.kind.x_axes()[self.x.min(self.kind.x_axes().len() - 1)]
    }

    pub fn y_axis(&self) -> &'static YAxis {
        &self.kind.y_axes()[self.y.min(self.kind.y_axes().len() - 1)]
    }

    pub fn input(&self, key: InputKey) -> Option<f64> {
        self.inputs
            .iter()
            .find(|(k, _)| *k == key)
            .and_then(|(_, v)| *v)
    }

    pub fn set_input(&mut self, key: InputKey, value: Option<f64>) {
        match self.inputs.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.inputs.push((key, value)),
        }
    }
}

/// One line on the plot.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub source: usize,
    pub target: Option<usize>,
    pub points: Vec<(f64, f64)>,
    /// Draw as horizontal runs with vertical jumps.
    pub step: bool,
    /// Where a new charge takes over on an application profile, by point index.
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub from: usize,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_graph_has_axes_and_a_main_input_for_each_x() {
        for kind in Kind::ALL {
            assert!(!kind.x_axes().is_empty(), "{kind:?}");
            assert!(!kind.y_axes().is_empty(), "{kind:?}");
            for x in kind.x_axes() {
                assert!(kind.main_input(x).is_some(), "{kind:?} {x:?}");
            }
        }
    }

    #[test]
    fn the_target_signature_input_only_shows_on_its_own_axis() {
        let x = &Kind::Damage.x_axes()[0];
        let y = &Kind::Damage.y_axes()[0];
        let keys: Vec<InputKey> = Kind::Damage
            .misc_inputs(x, y)
            .iter()
            .map(|i| i.key)
            .collect();
        assert_eq!(keys, vec![InputKey::Time]);
        let sig = &Kind::Damage.x_axes()[4];
        let keys: Vec<InputKey> = Kind::Damage
            .misc_inputs(sig, y)
            .iter()
            .map(|i| i.key)
            .collect();
        assert_eq!(keys, vec![InputKey::Distance, InputKey::Time]);
    }

    #[test]
    fn the_capsim_toggle_needs_time_against_gigajoules() {
        let time = &Kind::Cap.x_axes()[0];
        let amount = &Kind::Cap.y_axes()[0];
        let regen = &Kind::Cap.y_axes()[1];
        assert_eq!(Kind::Cap.toggles_for(time, amount), vec![Toggle::Capsim]);
        assert!(Kind::Cap.toggles_for(time, regen).is_empty());
    }

    #[test]
    fn resists_rename_the_damage_axis() {
        let y = &Kind::Damage.y_axes()[0];
        let mut options = Options::default();
        assert_eq!(y.title(&options), "DPS");
        options.resists = true;
        assert_eq!(y.title(&options), "Effective dps");
    }
}
