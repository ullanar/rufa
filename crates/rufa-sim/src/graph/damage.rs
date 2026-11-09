//! The Damage Stats graph: what lands on a target by distance, time, speed
//! or signature.

use rufa_data::Info;

use super::dealers::{self, Dealer, Shot, SourceData};
use super::dmg::Dmg;
use super::model::{Quantity, Request, Unit};
use super::sample;
use super::tackle::{self, Motion, Tackle};
use super::target::{Target, TargetData};
use super::timeline::{self, Timeline};
use super::{Ctx, InputKey};
use crate::{Calculation, Fit};

pub const TIME_LIMIT: f64 = 2500.0;
const SAMPLES: usize = 50;
const DEPTH: u32 = 2;

/// One source fit prepared for any number of points.
pub struct Prepared<'a> {
    pub fit: &'a Fit,
    pub calc: &'a Calculation,
    pub src: SourceData,
    pub dealers: Vec<Dealer>,
    pub tackle: Tackle,
}

pub fn prepare<'a, I: Info>(
    ctx: &Ctx<'_, I>,
    fit: &'a Fit,
    calc: &'a Calculation,
    reload: bool,
) -> Prepared<'a> {
    Prepared {
        fit,
        calc,
        src: dealers::source_data(ctx, fit, calc),
        dealers: dealers::dealers(ctx, fit, calc, reload),
        tackle: tackle::collect(ctx, fit, calc),
    }
}

/// A target prepared against one source.
pub struct Against<'a> {
    pub target: Target<'a>,
    pub data: TargetData,
    pub motion: Motion,
}

pub fn against<'a, I: Info>(ctx: &Ctx<'_, I>, target: Target<'a>) -> Against<'a> {
    Against {
        data: target.data(ctx.ids),
        motion: tackle::motion(ctx, &target),
        target,
    }
}

/// The attacker and target vectors in metres per second and degrees.
struct Geometry {
    atk_speed: f64,
    atk_angle: f64,
    tgt_speed: f64,
    tgt_angle: f64,
}

fn geometry(req: &Request, src: &SourceData, tgt: &TargetData) -> Geometry {
    Geometry {
        atk_speed: req.vectors.attacker.length.clamp(0.0, 1.0) * src.max_velocity,
        atk_angle: req.vectors.attacker.angle,
        tgt_speed: req.vectors.target.length.clamp(0.0, 1.0) * tgt.velocity,
        tgt_angle: req.vectors.target.angle,
    }
}

struct Eval<'a, 'b, I: Info> {
    ctx: &'a Ctx<'a, I>,
    req: &'a Request,
    src: &'a Prepared<'b>,
    tgt: &'a Against<'b>,
    geometry: Geometry,
    timeline: Option<Timeline>,
}

impl<I: Info> Eval<'_, '_, I> {
    fn tgt_fit(&self) -> Option<(&Fit, &Calculation)> {
        match self.tgt.target {
            Target::Fit { fit, calc, .. } => Some((fit, calc)),
            Target::Profile(_) => None,
        }
    }

    /// Target speed and signature at `d` once the attacker's tackle lands.
    fn tackled(&self, d: Option<f64>, speed: f64, sig: f64) -> (f64, f64) {
        if !self.req.options.projected {
            return (speed, sig);
        }
        let options = &self.req.options;
        let tackled = self
            .src
            .tackle
            .speed(&self.src.src, &self.tgt.motion, d, speed, options);
        let factor =
            self.src
                .tackle
                .signature(&self.src.src, &self.tgt.motion, d, tackled, options);
        (tackled, sig * factor)
    }

    fn value(&self, d: Option<f64>, time: Option<f64>, tgt_speed: f64, tgt_sig: f64) -> f64 {
        let (tgt_speed, tgt_sig) = self.tackled(d, tgt_speed, tgt_sig);
        let shot = Shot {
            atk_speed: self.geometry.atk_speed,
            atk_angle: self.geometry.atk_angle,
            distance: d,
            tgt_speed,
            tgt_angle: self.geometry.tgt_angle,
            tgt_sig,
        };
        self.sum(&shot, time)
    }

    fn sum(&self, shot: &Shot, time: Option<f64>) -> f64 {
        let y = self.req.y_axis().q;
        let resists = if self.req.options.resists {
            self.tgt.data.resists
        } else {
            [0.0; 4]
        };
        let mut total = Dmg::default();
        for (i, dealer) in self.src.dealers.iter().enumerate() {
            let app = dealers::application(
                self.ctx,
                dealer,
                &self.src.src,
                &self.tgt.data,
                self.tgt_fit(),
                shot,
                &self.req.options,
            );
            if app <= 0.0 {
                continue;
            }
            let dmg = match (time, &self.timeline) {
                (Some(t), Some(tl)) => {
                    let line = &tl.lines[i];
                    match y {
                        Quantity::Dps => line.dps_at(t),
                        Quantity::Volley => line.volley_at(t),
                        _ => line.damage_at(t),
                    }
                }
                _ => match y {
                    Quantity::Dps => dealer.dps,
                    Quantity::Volley => dealer.volley.scale(1.0 + dealer.spool.0),
                    _ => Dmg::default(),
                },
            };
            let dmg = self.breacher_cap(dealer, dmg);
            total = total.plus(dmg.scale(app));
        }
        total.applied(resists)
    }

    /// A breacher tick cannot take more than its share of the target's HP.
    fn breacher_cap(&self, dealer: &Dealer, dmg: Dmg) -> Dmg {
        if let dealers::Weapon::Breacher {
            absolute, relative, ..
        } = dealer.weapon
            && absolute > 0.0
            && self.tgt.data.hp.is_finite()
        {
            let per_tick = absolute.min(relative * self.tgt.data.hp);
            return dmg.scale(per_tick / absolute);
        }
        dmg
    }
}

fn needs_time(req: &Request) -> bool {
    req.y_axis().q == Quantity::Damage
}

fn distance_input(req: &Request) -> Option<f64> {
    req.input(InputKey::Distance).map(|km| km * 1000.0)
}

fn time_input(req: &Request) -> Option<f64> {
    req.input(InputKey::Time).map(|t| t.clamp(0.0, TIME_LIMIT))
}

/// Points of one source against one target. None when a needed input is
/// blank.
pub fn series<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    src: &Prepared<'_>,
    tgt: &Against<'_>,
) -> Option<(Vec<(f64, f64)>, bool)> {
    let x = req.x_axis();
    let geometry = geometry(req, &src.src, &tgt.data);
    let (lo, hi) = (req.range.0.min(req.range.1), req.range.0.max(req.range.1));
    let misc_time = time_input(req);
    if x.q != Quantity::Time && needs_time(req) && misc_time.is_none() {
        return None;
    }
    let until = match x.q {
        Quantity::Time => hi.clamp(0.0, TIME_LIMIT),
        _ => misc_time.unwrap_or(0.0),
    };
    let timeline = (x.q == Quantity::Time || misc_time.is_some())
        .then(|| timeline::build(&src.dealers, until));
    let eval = Eval {
        ctx,
        req,
        src,
        tgt,
        geometry,
        timeline,
    };
    let base_speed = eval.geometry.tgt_speed;
    let base_sig = tgt.data.signature;
    let sig_pct = req.input(InputKey::TgtSigRad).unwrap_or(100.0) / 100.0;
    match x.q {
        Quantity::Distance => {
            let pts = sample::sample(lo * 1000.0, hi * 1000.0, SAMPLES, DEPTH, |d| {
                eval.value(Some(d), misc_time, base_speed, base_sig * sig_pct)
            });
            Some((
                pts.into_iter().map(|(d, y)| (d / 1000.0, y)).collect(),
                false,
            ))
        }
        Quantity::Time => {
            let d = distance_input(req);
            let lo = lo.clamp(0.0, TIME_LIMIT);
            let hi = hi.clamp(0.0, TIME_LIMIT);
            let tl = eval.timeline.as_ref().expect("time axis builds a timeline");
            let mut times = tl.changes(lo, hi, needs_time(req));
            times.insert(0, lo);
            times.push(hi);
            times.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            let points: Vec<(f64, f64)> = times
                .iter()
                .map(|t| (*t, eval.value(d, Some(*t), base_speed, base_sig * sig_pct)))
                .collect();
            Some((sample::steps(&points, lo, hi), true))
        }
        Quantity::TgtSpeed => {
            let d = distance_input(req);
            let to_ms = |v: f64| match x.unit {
                Unit::Percent => v / 100.0 * tgt.data.velocity,
                _ => v,
            };
            let pts = sample::sample(lo, hi, SAMPLES, DEPTH, |v| {
                eval.value(d, misc_time, to_ms(v), base_sig * sig_pct)
            });
            Some((pts, false))
        }
        Quantity::TgtSigRad => {
            let d = distance_input(req);
            let to_m = |v: f64| match x.unit {
                Unit::Percent => v / 100.0 * tgt.data.signature,
                _ => v,
            };
            if x.unit == Unit::Percent && !tgt.data.signature.is_finite() {
                let y = eval.value(d, misc_time, base_speed, f64::INFINITY);
                return Some((vec![(lo, y), (hi, y)], false));
            }
            let pts = sample::sample(lo, hi, SAMPLES, DEPTH, |v| {
                eval.value(d, misc_time, base_speed, to_m(v))
            });
            Some((pts, false))
        }
        _ => None,
    }
}

/// The exact value at one X, for the readout.
pub fn point<I: Info>(
    ctx: &Ctx<'_, I>,
    req: &Request,
    src: &Prepared<'_>,
    tgt: &Against<'_>,
    x_value: f64,
) -> Option<f64> {
    let x = req.x_axis();
    let geometry = geometry(req, &src.src, &tgt.data);
    let misc_time = time_input(req);
    if x.q != Quantity::Time && needs_time(req) && misc_time.is_none() {
        return None;
    }
    let until = match x.q {
        Quantity::Time => x_value.clamp(0.0, TIME_LIMIT),
        _ => misc_time.unwrap_or(0.0),
    };
    let timeline = (x.q == Quantity::Time || misc_time.is_some())
        .then(|| timeline::build(&src.dealers, until));
    let eval = Eval {
        ctx,
        req,
        src,
        tgt,
        geometry,
        timeline,
    };
    let base_speed = eval.geometry.tgt_speed;
    let base_sig = tgt.data.signature * req.input(InputKey::TgtSigRad).unwrap_or(100.0) / 100.0;
    Some(match x.q {
        Quantity::Distance => eval.value(Some(x_value * 1000.0), misc_time, base_speed, base_sig),
        Quantity::Time => eval.value(
            distance_input(req),
            Some(x_value.clamp(0.0, TIME_LIMIT)),
            base_speed,
            base_sig,
        ),
        Quantity::TgtSpeed => {
            let v = match x.unit {
                Unit::Percent => x_value / 100.0 * tgt.data.velocity,
                _ => x_value,
            };
            eval.value(distance_input(req), misc_time, v, base_sig)
        }
        Quantity::TgtSigRad => {
            let s = match x.unit {
                Unit::Percent => x_value / 100.0 * tgt.data.signature,
                _ => x_value,
            };
            eval.value(distance_input(req), misc_time, base_speed, s)
        }
        _ => return None,
    })
}
