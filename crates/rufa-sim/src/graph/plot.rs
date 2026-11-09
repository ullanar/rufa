//! One entry point for every graph: sources and targets in, series out.

use rufa_data::Info;

use super::model::{Kind, Request, Series};
use super::target::Target;
use super::{Ctx, ammo, damage, ewar, motion, regen, reps};
use crate::{Calculation, Fit};

/// A fit on the graph.
pub struct Source<'a> {
    pub fit: &'a Fit,
    pub calc: &'a Calculation,
}

/// Why a graph has nothing to draw, in words for the pane.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub series: Vec<Series>,
    pub empty: Option<&'static str>,
}

fn pairs(req: &Request, sources: usize, targets: usize) -> Vec<(usize, Option<usize>)> {
    if req.kind.has_targets() {
        (0..sources)
            .flat_map(|s| (0..targets).map(move |t| (s, Some(t))))
            .collect()
    } else {
        (0..sources).map(|s| (s, None)).collect()
    }
}

/// Every series the request asks for.
pub fn plot<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    sources: &[Source<'_>],
    targets: &[Target<'_>],
) -> Plot {
    let mut out = Plot {
        series: Vec::new(),
        empty: None,
    };
    if sources.is_empty() {
        out.empty = Some("Open a fit to graph it");
        return out;
    }
    if req.kind.has_targets() && targets.is_empty() {
        out.empty = Some("Pick a target");
        return out;
    }
    let mut blank = false;
    for (s, t) in pairs(req, sources.len(), targets.len()) {
        let src = &sources[s];
        let (points, step, segments) = match req.kind {
            Kind::Damage => {
                let prepared = damage::prepare(ctx, src.fit, src.calc, req.options.reload);
                let tgt = damage::against(ctx, targets[t.unwrap_or(0)]);
                match damage::series(ctx, req, &prepared, &tgt) {
                    Some((p, step)) => (p, step, Vec::new()),
                    None => {
                        blank = true;
                        continue;
                    }
                }
            }
            Kind::Application => {
                let prepared = ammo::prepare(ctx, req, src.fit, src.calc, targets[t.unwrap_or(0)]);
                let (p, segments) = ammo::series(&prepared, req);
                (p, false, segments)
            }
            Kind::Ewar => {
                let prepared = ewar::prepare(ctx, req, src.fit, src.calc);
                if prepared.is_empty() {
                    continue;
                }
                let (p, step) = ewar::series(&prepared, req);
                (p, step, Vec::new())
            }
            Kind::Reps => {
                let prepared = reps::prepare(ctx, req, src.fit, src.calc);
                if prepared.is_empty() {
                    continue;
                }
                match reps::series(&prepared, req) {
                    Some((p, step)) => (p, step, Vec::new()),
                    None => {
                        blank = true;
                        continue;
                    }
                }
            }
            Kind::Shield | Kind::Cap => match regen::series(ctx, req, src.fit, src.calc) {
                Some((p, step)) => (p, step, Vec::new()),
                None => continue,
            },
            Kind::Mobility | Kind::Warp | Kind::Lock => {
                match motion::series(ctx, req, src.fit, src.calc) {
                    Some((p, step)) => (p, step, Vec::new()),
                    None => {
                        blank = true;
                        continue;
                    }
                }
            }
        };
        if points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            continue;
        }
        out.series.push(Series {
            source: s,
            target: t,
            points,
            step,
            segments,
        });
    }
    if out.series.is_empty() {
        out.empty = Some(match (req.kind, blank) {
            (Kind::Damage | Kind::Application, false) => "No weapons on these fits",
            (Kind::Damage, true) => "Enter a time to see damage inflicted",
            (Kind::Reps, true) => "Enter a time to see total repair",
            (Kind::Reps, false) => "No remote repairers on these fits",
            (Kind::Ewar, _) => "No electronic warfare on these fits",
            (Kind::Mobility, true) => "Enter the target mass and inertia",
            _ => "Nothing to draw",
        });
    }
    out
}

/// The exact Y of one series at `x`, with the charge name on an
/// application profile.
pub fn point<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    source: &Source<'_>,
    target: Option<&Target<'_>>,
    x: f64,
) -> Option<(f64, Option<String>)> {
    match req.kind {
        Kind::Damage => {
            let prepared = damage::prepare(ctx, source.fit, source.calc, req.options.reload);
            let tgt = damage::against(ctx, *target?);
            damage::point(ctx, req, &prepared, &tgt, x).map(|y| (y, None))
        }
        Kind::Application => {
            let prepared = ammo::prepare(ctx, req, source.fit, source.calc, *target?);
            let d = x * 1000.0;
            Some((
                ammo::value(&prepared, req, d),
                ammo::charge_name(&prepared, d),
            ))
        }
        Kind::Ewar => {
            let prepared = ewar::prepare(ctx, req, source.fit, source.calc);
            Some((ewar::point(&prepared, req, x), None))
        }
        Kind::Reps => {
            let prepared = reps::prepare(ctx, req, source.fit, source.calc);
            reps::point(&prepared, req, x).map(|y| (y, None))
        }
        Kind::Shield | Kind::Cap => {
            regen::point(ctx, req, source.fit, source.calc, x).map(|y| (y, None))
        }
        Kind::Mobility | Kind::Warp | Kind::Lock => {
            motion::point(ctx, req, source.fit, source.calc, x).map(|y| (y, None))
        }
    }
}

/// The default X range of a request for these sources.
pub fn default_range<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    sources: &[Source<'_>],
) -> (f64, f64) {
    match req.kind {
        Kind::Application => {
            let fits: Vec<(&Fit, &Calculation)> = sources.iter().map(|s| (s.fit, s.calc)).collect();
            ammo::default_range(ctx, req, &fits)
        }
        kind => kind
            .main_input(req.x_axis())
            .map(|i| i.range)
            .unwrap_or((0.0, 100.0)),
    }
}
