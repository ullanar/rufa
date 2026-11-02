use std::time::{Duration, Instant};

use rufa_data::Info;

use crate::attrs::AttrIds;
use crate::cap::CapResult;
use crate::layout::SlotLayout;
use crate::projection::{self, Projected};
use crate::stats::Stats;
use crate::{Calculation, Fit};

const HISTORY_LIMIT: usize = 100;
pub const CAP_HORIZON: f64 = 1800.0;

pub struct Document {
    pub fit: Fit,
    pub calc: Calculation,
    pub stats: Stats,
    pub layout: SlotLayout,
    pub last_calc: Duration,
    pub cap: CapResult,
    pub projections: Vec<Projected>,
    undo: Vec<Fit>,
    redo: Vec<Fit>,
}

impl Document {
    pub fn new(info: &impl Info, ids: &AttrIds, fit: Fit) -> Document {
        let start = Instant::now();
        let calc = crate::calculate(info, &fit);
        Document {
            stats: Stats::from_calc(info, ids, &fit, &calc, &projection::Incoming::default()),
            layout: SlotLayout::from_calc(ids, &calc),
            cap: crate::cap::simulate_fit(ids, &fit, &calc, 0.0, CAP_HORIZON),
            last_calc: start.elapsed(),
            calc,
            fit,
            projections: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn project(&mut self, info: &impl Info, ids: &AttrIds, source: Projected) {
        self.projections.push(source);
        self.recalc(info, ids);
    }

    pub fn unproject(&mut self, info: &impl Info, ids: &AttrIds, index: usize) {
        if index < self.projections.len() {
            self.projections.remove(index);
            self.recalc(info, ids);
        }
    }

    pub fn edit<T>(
        &mut self,
        info: &impl Info,
        ids: &AttrIds,
        edit: impl FnOnce(&mut Fit, &SlotLayout) -> T,
    ) -> T {
        let before = self.fit.clone();
        let out = edit(&mut self.fit, &self.layout);
        self.undo.push(before);
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.recalc(info, ids);
        out
    }

    pub fn undo(&mut self, info: &impl Info, ids: &AttrIds) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.fit, previous));
        self.recalc(info, ids);
        true
    }

    pub fn redo(&mut self, info: &impl Info, ids: &AttrIds) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.fit, next));
        self.recalc(info, ids);
        true
    }

    pub fn recalc(&mut self, info: &impl Info, ids: &AttrIds) {
        let start = Instant::now();
        let incoming = projection::incoming(info, ids, &self.fit, &self.projections);
        self.fit.incoming = incoming.projection.clone();
        self.calc = crate::calculate(info, &self.fit);
        self.stats = Stats::from_calc(info, ids, &self.fit, &self.calc, &incoming);
        self.layout = SlotLayout::from_calc(ids, &self.calc);
        self.cap = crate::cap::simulate_fit(
            ids,
            &self.fit,
            &self.calc,
            incoming.reps.capacitor,
            CAP_HORIZON,
        );
        self.last_calc = start.elapsed();
    }
}
