//! What a graph shoots at: a profile of numbers or another calculated fit.

use crate::attrs::AttrIds;
use crate::{Calculation, Fit, ItemResult};

/// A target described by hand. None for signature or HP means unlimited.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub resists: [f64; 4],
    pub signature: Option<f64>,
    pub velocity: f64,
    pub radius: f64,
    pub hp: Option<f64>,
}

impl Profile {
    pub fn ideal() -> Profile {
        Profile {
            name: "Ideal target".to_string(),
            resists: [0.0; 4],
            signature: None,
            velocity: 0.0,
            radius: 0.0,
            hp: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ResistMode {
    #[default]
    Auto,
    Shield,
    Armor,
    Hull,
    Weighted,
}

impl ResistMode {
    pub const ALL: [ResistMode; 5] = [
        ResistMode::Auto,
        ResistMode::Shield,
        ResistMode::Armor,
        ResistMode::Hull,
        ResistMode::Weighted,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ResistMode::Auto => "Auto",
            ResistMode::Shield => "Shield",
            ResistMode::Armor => "Armor",
            ResistMode::Hull => "Hull",
            ResistMode::Weighted => "Weighted average",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    Profile(&'a Profile),
    Fit {
        fit: &'a Fit,
        calc: &'a Calculation,
        mode: ResistMode,
    },
}

/// The numbers the application math reads off a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetData {
    pub resists: [f64; 4],
    pub signature: f64,
    pub velocity: f64,
    pub radius: f64,
    pub hp: f64,
    /// Which layer Auto picked on a fit, for the tooltip.
    pub layer: Option<&'static str>,
    pub is_fit: bool,
}

fn attr(item: &ItemResult, id: i32) -> f64 {
    item.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn layer_resists(ship: &ItemResult, ids: [i32; 4]) -> [f64; 4] {
    [
        1.0 - attr(ship, ids[0]),
        1.0 - attr(ship, ids[1]),
        1.0 - attr(ship, ids[2]),
        1.0 - attr(ship, ids[3]),
    ]
}

struct Layers {
    hp: [f64; 3],
    resists: [[f64; 4]; 3],
}

fn layers(ids: &AttrIds, ship: &ItemResult) -> Layers {
    Layers {
        hp: [
            attr(ship, ids.shield_hp),
            attr(ship, ids.armor_hp),
            attr(ship, ids.hull_hp),
        ],
        resists: [
            layer_resists(
                ship,
                [
                    ids.shield_em,
                    ids.shield_thermal,
                    ids.shield_kinetic,
                    ids.shield_explosive,
                ],
            ),
            layer_resists(
                ship,
                [
                    ids.armor_em,
                    ids.armor_thermal,
                    ids.armor_kinetic,
                    ids.armor_explosive,
                ],
            ),
            layer_resists(
                ship,
                [
                    ids.hull_em,
                    ids.hull_thermal,
                    ids.hull_kinetic,
                    ids.hull_explosive,
                ],
            ),
        ],
    }
}

/// Effective HP of a layer under an even damage split.
fn uniform_ehp(hp: f64, resists: &[f64; 4]) -> f64 {
    let taken: f64 = resists.iter().map(|r| 0.25 * (1.0 - r)).sum();
    if taken <= 0.0 {
        f64::INFINITY
    } else {
        hp / taken
    }
}

/// Which layer a fit fights from: the one with the most effective HP, the
/// best resists and the repair to keep it. Pyfa's score.
fn auto_layer(ids: &AttrIds, ship: &ItemResult, layers: &Layers) -> usize {
    let ehp: Vec<f64> = (0..3)
        .map(|i| uniform_ehp(layers.hp[i], &layers.resists[i]))
        .collect();
    let total: f64 = ehp.iter().filter(|e| e.is_finite()).sum::<f64>().max(1e-9);
    let factor: Vec<f64> = (0..3)
        .map(|i| {
            if layers.hp[i] <= 0.0 {
                1.0
            } else {
                ehp[i] / layers.hp[i]
            }
        })
        .collect();
    let best = factor.iter().cloned().fold(0.0, f64::max).max(1e-9);
    let repair = [
        attr(ship, ids.shield_boost_rate),
        attr(ship, ids.armor_repair_rate),
        attr(ship, ids.hull_repair_rate),
    ];
    let passive = attr(ship, ids.passive_shield_recharge_rate);
    let mut scored: Vec<(f64, usize)> = (0..3)
        .map(|i| {
            let share = if ehp[i].is_finite() {
                ehp[i] / total
            } else {
                0.0
            };
            let mut score = 100.0 * share.powf(1.5)
                + 25.0 * (factor[i] / best).powf(1.5)
                + 10_000.0 * repair[i] * factor[i] / total;
            if i == 0 {
                score += 5_000.0 * passive * factor[0] / total;
            }
            (score, i)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    scored[0].1
}

pub fn fit_resists(
    ids: &AttrIds,
    calc: &Calculation,
    mode: ResistMode,
) -> ([f64; 4], &'static str) {
    const NAMES: [&str; 3] = ["shield", "armor", "hull"];
    let ship = &calc.ship;
    let layers = layers(ids, ship);
    match mode {
        ResistMode::Shield => (layers.resists[0], NAMES[0]),
        ResistMode::Armor => (layers.resists[1], NAMES[1]),
        ResistMode::Hull => (layers.resists[2], NAMES[2]),
        ResistMode::Weighted => {
            let total: f64 = layers.hp.iter().sum();
            let mut out = [0.0; 4];
            for (t, slot) in out.iter_mut().enumerate() {
                let ehp: f64 = (0..3)
                    .map(|i| layers.hp[i] / (1.0 - layers.resists[i][t]).max(1e-9))
                    .sum();
                *slot = if ehp > 0.0 { 1.0 - total / ehp } else { 0.0 };
            }
            (out, "average")
        }
        ResistMode::Auto => {
            let i = auto_layer(ids, ship, &layers);
            (layers.resists[i], NAMES[i])
        }
    }
}

impl Target<'_> {
    pub fn data(&self, ids: &AttrIds) -> TargetData {
        match self {
            Target::Profile(p) => TargetData {
                resists: p.resists,
                signature: p.signature.filter(|s| *s > 0.0).unwrap_or(f64::INFINITY),
                velocity: p.velocity.max(0.0),
                radius: p.radius.max(0.0),
                hp: p.hp.filter(|h| *h > 0.0).unwrap_or(f64::INFINITY),
                layer: None,
                is_fit: false,
            },
            Target::Fit { calc, mode, .. } => {
                let ship = &calc.ship;
                let (resists, layer) = fit_resists(ids, calc, *mode);
                TargetData {
                    resists,
                    signature: attr(ship, ids.signature_radius).max(1e-9),
                    velocity: attr(ship, ids.max_velocity),
                    radius: attr(ship, ids.radius),
                    hp: attr(ship, ids.shield_hp)
                        + attr(ship, ids.armor_hp)
                        + attr(ship, ids.hull_hp),
                    layer: Some(layer),
                    is_fit: true,
                }
            }
        }
    }

    pub fn name(&self) -> String {
        match self {
            Target::Profile(p) => p.name.clone(),
            Target::Fit { fit, .. } => fit.name.clone().unwrap_or_default(),
        }
    }

    pub fn calc(&self) -> Option<&Calculation> {
        match self {
            Target::Profile(_) => None,
            Target::Fit { calc, .. } => Some(calc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ideal_profile_has_no_signature_and_no_hp_limit() {
        let p = Profile::ideal();
        let ids = crate::attrs::AttrIds::placeholder();
        let d = Target::Profile(&p).data(&ids);
        assert!(d.signature.is_infinite());
        assert!(d.hp.is_infinite());
        assert_eq!(d.velocity, 0.0);
        assert_eq!(d.resists, [0.0; 4]);
    }

    #[test]
    fn uniform_ehp_doubles_at_half_resists() {
        assert!((uniform_ehp(1000.0, &[0.5; 4]) - 2000.0).abs() < 1e-9);
        assert_eq!(uniform_ehp(1000.0, &[0.0; 4]), 1000.0);
    }
}
