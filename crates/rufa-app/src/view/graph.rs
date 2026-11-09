use iced::widget::Widget;
use iced::widget::canvas::{self, Frame, Path, Stroke, Text, stroke};
use iced::widget::{
    button, canvas as canvas_widget, column, container, pick_list, row, space, stack, text,
    text_input,
};
use iced::{Color, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use rufa_sim::graph::target::ResistMode;
use rufa_sim::graph::{AmmoTier, DroneMode, Kind, Quantity, Toggle, Unit, Vector};

use crate::graphs::{GraphMsg, Pinned, SeriesInfo};
use crate::ui::Add;
use crate::ui::{self, El};
use crate::{App, Message, fmt, theme};

const COLORS: [Color; 6] = theme::SERIES;
const DASHES: [&[f32]; 4] = [&[], &[6.0, 3.0], &[2.0, 3.0], &[6.0, 3.0, 2.0, 3.0]];

fn gm(m: GraphMsg) -> Message {
    Message::Graph(m)
}

pub fn view(app: &App) -> El<'_> {
    if app.ids.is_none() {
        return text("No game data").boxed();
    }
    let g = &app.graph;
    let kind = g.req.kind;
    let mut body = column![strip(app), axis_row(app)].spacing(ui::GAP_ROW);
    body = body.add(plot_area(app));
    if kind.has_targets() {
        body = body.add(target_strip(app));
        if g.editor_open {
            body = body.add(profile_editor(app));
        }
    }
    let x = g.req.x_axis();
    let y = g.req.y_axis();
    let misc = kind.misc_inputs(x, y);
    if !misc.is_empty() || kind.has_vectors() {
        body = body.add(input_strip(app, &misc));
    }
    let toggles = kind.toggles_for(x, y);
    if !toggles.is_empty() {
        body = body.add(option_strip(app, &toggles));
    }
    body.height(Length::Fill).boxed()
}

fn strip(app: &App) -> El<'_> {
    ui::segmented(
        Kind::ALL
            .iter()
            .map(|k| {
                (
                    k.label().to_string(),
                    app.graph.req.kind == *k,
                    gm(GraphMsg::Kind(*k)),
                )
            })
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AxisChoice {
    index: usize,
    label: String,
}

impl std::fmt::Display for AxisChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn axis_row(app: &App) -> El<'_> {
    let g = &app.graph;
    let kind = g.req.kind;
    let mut line = ui::line().spacing(ui::PAD).align_y(iced::Alignment::Center);
    let ys: Vec<AxisChoice> = kind
        .y_axes()
        .iter()
        .enumerate()
        .map(|(i, a)| AxisChoice {
            index: i,
            label: a.selector.to_string(),
        })
        .collect();
    let y_pick: El<'_> = if ys.len() > 1 {
        let picked = ys.get(g.req.y).cloned();
        pick_list(picked, ys, ToString::to_string)
            .on_select(|c: AxisChoice| gm(GraphMsg::Y(c.index)))
            .text_size(ui::SMALL)
            .padding(ui::field(3.0, 8.0))
            .style(theme::picker)
            .boxed()
    } else {
        ui::small(ys[0].label.clone()).boxed()
    };
    line = line.add(
        row![ui::label("Y"), y_pick]
            .spacing(ui::GAP)
            .align_y(iced::Alignment::Center),
    );
    let xs: Vec<AxisChoice> = kind
        .x_axes()
        .iter()
        .enumerate()
        .map(|(i, a)| AxisChoice {
            index: i,
            label: a.selector(),
        })
        .collect();
    let x_pick: El<'_> = if xs.len() > 1 {
        let picked = xs.get(g.req.x).cloned();
        pick_list(picked, xs, ToString::to_string)
            .on_select(|c: AxisChoice| gm(GraphMsg::X(c.index)))
            .text_size(ui::SMALL)
            .padding(ui::field(3.0, 8.0))
            .style(theme::picker)
            .boxed()
    } else {
        ui::small(xs[0].label.clone()).boxed()
    };
    line = line.add(
        row![ui::label("X"), x_pick]
            .spacing(ui::GAP)
            .align_y(iced::Alignment::Center),
    );
    let unit = g.req.x_axis().unit.label();
    let field = text_input("lo - hi", g.range_text.clone())
        .on_input(|s| gm(GraphMsg::Range(s)))
        .size(ui::SMALL)
        .padding(ui::field(3.0, 6.0))
        .width(110)
        .style(if g.range_ok {
            theme::input
        } else {
            theme::input_bad
        });
    let mut range = row![ui::label("RANGE"), field]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
    if !unit.is_empty() {
        range = range.add(ui::small(unit));
    }
    range = range.add(
        button(text("\u{21ba}").size(ui::SMALL))
            .padding(ui::pill_tight(ui::GAP_ROW))
            .style(theme::ghost)
            .on_press(gm(GraphMsg::RangeReset)),
    );
    line = line.add(range);
    if g.took_ms > 0.0 {
        line = line.add(ui::label(format!("{:.1} MS", g.took_ms)));
    }
    line.wrap().boxed()
}

struct Drawn {
    points: Vec<(f64, f64)>,
    color: Color,
    dash: &'static [f32],
    step: bool,
    muted: bool,
    segments: Vec<(usize, Color, String)>,
}

fn series_color(info: &SeriesInfo) -> Color {
    COLORS[info.source % COLORS.len()]
}

fn series_dash(info: &SeriesInfo) -> &'static [f32] {
    DASHES[info.target.unwrap_or(0) % DASHES.len()]
}

fn ammo_color(name: &str, palette: &mut Vec<(String, Color)>) -> Color {
    if let Some((_, c)) = palette.iter().find(|(n, _)| n == name) {
        return *c;
    }
    let c = COLORS[palette.len() % COLORS.len()];
    palette.push((name.to_string(), c));
    c
}

fn plot_area(app: &App) -> El<'_> {
    let g = &app.graph;
    let Some(plot) = &g.plot else {
        return container(ui::muted("Computing"))
            .center(Length::Fill)
            .boxed();
    };
    if let Some(reason) = plot.empty {
        return container(ui::muted(reason)).center(Length::Fill).boxed();
    }
    let mut palette: Vec<(String, Color)> = Vec::new();
    let application = g.req.kind == Kind::Application;
    let drawn: Vec<Drawn> = plot
        .series
        .iter()
        .zip(&g.series)
        .enumerate()
        .map(|(i, (s, info))| Drawn {
            points: s.points.clone(),
            color: series_color(info),
            dash: series_dash(info),
            step: s.step,
            muted: g.muted.contains(&i),
            segments: s
                .segments
                .iter()
                .map(|seg| {
                    (
                        seg.from,
                        ammo_color(&seg.name, &mut palette),
                        seg.name.clone(),
                    )
                })
                .collect(),
        })
        .collect();
    let y_axis = g.req.y_axis();
    let (lo, hi) = (
        g.req.range.0.min(g.req.range.1),
        g.req.range.0.max(g.req.range.1),
    );
    let (y_lo, y_hi) = y_bounds(&drawn);
    let program = PlotProgram {
        drawn,
        names: g.series.iter().map(|s| s.name.clone()).collect(),
        x: (lo, hi),
        y: (y_lo, y_hi),
        x_unit: g.req.x_axis().unit,
        y_title: y_axis.title(&g.req.options),
        x_title: g.req.x_axis().title(),
        integer_y: matches!(
            y_axis.q,
            Quantity::Dps | Quantity::Volley | Quantity::Damage
        ),
        pinned: g.pinned.clone(),
        cache: &g.cache,
    };
    let canvas = canvas_widget(program)
        .width(Length::Fill)
        .height(Length::Fill);
    let legend = if application {
        let mut col = ui::col().spacing(2).align_x(iced::Alignment::End);
        for (name, color) in palette {
            col = col.add(legend_chip(name, color, false, None));
        }
        col
    } else {
        let mut col = ui::col().spacing(2).align_x(iced::Alignment::End);
        for (i, info) in g.series.iter().enumerate() {
            col = col.add(legend_chip(
                info.name.clone(),
                series_color(info),
                g.muted.contains(&i),
                Some(gm(GraphMsg::Mute(i))),
            ));
        }
        col
    };
    stack![
        canvas,
        container(legend)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .padding([ui::PAD, ui::PAD_WIDE]),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .boxed()
}

fn legend_chip<'a>(name: String, color: Color, muted: bool, press: Option<Message>) -> El<'a> {
    let swatch = container(space().width(8).height(8)).style(theme::rounded(
        if muted { theme::DISABLED } else { color },
        2.0,
    ));
    let label = text(name).size(ui::MICRO).color(if muted {
        theme::FAINT
    } else {
        theme::TEXT_SECONDARY
    });
    let inner = row![swatch, label]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
    let b = button(inner)
        .padding(ui::pill_tight(ui::GAP_ROW))
        .style(theme::legend);
    match press {
        Some(m) => b.on_press(m).boxed(),
        None => b.boxed(),
    }
}

fn y_bounds(drawn: &[Drawn]) -> (f64, f64) {
    let mut lo: f64 = 0.0;
    let mut hi: f64 = 0.0;
    for d in drawn.iter().filter(|d| !d.muted) {
        for p in &d.points {
            lo = lo.min(p.1);
            hi = hi.max(p.1);
        }
    }
    if hi <= lo {
        hi = lo + 1.0;
    }
    let hi = nice_ceiling(hi * 1.03);
    let lo = if lo < 0.0 { lo - (hi - lo) * 0.05 } else { 0.0 };
    (lo, hi)
}

/// The first of 1, 2, 5 times a power of ten at or above the value.
fn nice_ceiling(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let mag = 10f64.powf(v.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if m * mag >= v {
            return m * mag;
        }
    }
    v
}

/// Tick spacing that gives about `want` divisions.
fn tick_step(span: f64, want: f64) -> f64 {
    if span <= 0.0 {
        return 1.0;
    }
    let raw = span / want;
    let mag = 10f64.powf(raw.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if m * mag >= raw {
            return m * mag;
        }
    }
    raw
}

fn sig4(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let digits = (3 - v.abs().log10().floor() as i32).clamp(0, 6) as usize;
    trim_zeros(fmt::num(v, digits))
}

fn trim_zeros(s: String) -> String {
    if !s.contains('.') {
        return s;
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A tick label: whole numbers as such, the rest with the digits the step needs.
fn tick_label(v: f64, step: f64) -> String {
    if step >= 1.0 {
        return fmt::num(v, 0);
    }
    let digits = (-step.log10().floor() as i32).clamp(1, 6) as usize;
    trim_zeros(fmt::num(v, digits))
}

fn value_at(points: &[(f64, f64)], step: bool, x: f64) -> Option<f64> {
    let first = points.first()?;
    let last = points.last()?;
    if x < first.0 || x > last.0 {
        return None;
    }
    let i = points.partition_point(|p| p.0 <= x);
    if i == 0 {
        return Some(first.1);
    }
    let a = points[i - 1];
    if i >= points.len() {
        return Some(a.1);
    }
    let b = points[i];
    if step || b.0 <= a.0 {
        return Some(a.1);
    }
    let t = (x - a.0) / (b.0 - a.0);
    Some(a.1 + (b.1 - a.1) * t)
}

struct PlotProgram<'a> {
    drawn: Vec<Drawn>,
    names: Vec<String>,
    x: (f64, f64),
    y: (f64, f64),
    x_unit: Unit,
    y_title: String,
    x_title: String,
    integer_y: bool,
    pinned: Option<Pinned>,
    cache: &'a canvas::Cache,
}

#[derive(Default)]
struct PlotState {
    cursor: Option<Point>,
    panning: Option<f32>,
}

const MARGIN_LEFT: f32 = 52.0;
const MARGIN_BOTTOM: f32 = 30.0;
const MARGIN_TOP: f32 = 20.0;
const MARGIN_RIGHT: f32 = 14.0;

impl PlotProgram<'_> {
    fn area(&self, bounds: Size) -> Rectangle {
        Rectangle {
            x: MARGIN_LEFT,
            y: MARGIN_TOP,
            width: (bounds.width - MARGIN_LEFT - MARGIN_RIGHT).max(1.0),
            height: (bounds.height - MARGIN_TOP - MARGIN_BOTTOM).max(1.0),
        }
    }

    fn to_px(&self, area: Rectangle, x: f64, y: f64) -> Point {
        let fx = ((x - self.x.0) / (self.x.1 - self.x.0).max(1e-12)) as f32;
        let fy = ((y - self.y.0) / (self.y.1 - self.y.0).max(1e-12)) as f32;
        Point::new(
            area.x + fx * area.width,
            area.y + area.height - fy * area.height,
        )
    }

    fn x_of(&self, area: Rectangle, px: f32) -> f64 {
        let f = ((px - area.x) / area.width).clamp(0.0, 1.0) as f64;
        self.x.0 + f * (self.x.1 - self.x.0)
    }

    fn format_y(&self, v: f64) -> String {
        if self.integer_y {
            fmt::num(v, 0)
        } else {
            sig4(v)
        }
    }

    fn draw_static(&self, frame: &mut Frame) {
        let area = self.area(frame.size());
        let grid = Stroke::default()
            .with_color(theme::HAIRLINE)
            .with_width(1.0);
        let y_step = tick_step(self.y.1 - self.y.0, 4.0);
        let mut v = (self.y.0 / y_step).ceil() * y_step;
        while v <= self.y.1 + 1e-9 {
            let p = self.to_px(area, self.x.0, v);
            frame.stroke(
                &Path::line(
                    Point::new(area.x, p.y),
                    Point::new(area.x + area.width, p.y),
                ),
                grid,
            );
            frame.fill_text(Text {
                content: if self.integer_y {
                    fmt::num(v, 0)
                } else {
                    tick_label(v, y_step)
                },
                position: Point::new(area.x - 5.0, p.y),
                color: theme::MUTED,
                size: 10.0.into(),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
            v += y_step;
        }
        let x_step = tick_step(self.x.1 - self.x.0, 5.0);
        let mut v = (self.x.0 / x_step).ceil() * x_step;
        while v <= self.x.1 + 1e-9 {
            let p = self.to_px(area, v, self.y.0);
            frame.stroke(
                &Path::line(
                    Point::new(p.x, area.y),
                    Point::new(p.x, area.y + area.height),
                ),
                grid,
            );
            frame.fill_text(Text {
                content: tick_label(v, x_step),
                position: Point::new(p.x, area.y + area.height + 4.0),
                color: theme::MUTED,
                size: 10.0.into(),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Top,
                ..Text::default()
            });
            v += x_step;
        }
        frame.fill_text(Text {
            content: self.x_title.clone(),
            position: Point::new(area.x + area.width, area.y + area.height + 16.0),
            color: theme::FAINT,
            size: 10.0.into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Top,
            ..Text::default()
        });
        frame.fill_text(Text {
            content: self.y_title.clone(),
            position: Point::new(area.x, area.y - 6.0),
            color: theme::FAINT,
            size: 10.0.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Bottom,
            ..Text::default()
        });
        let visible = self.drawn.iter().filter(|d| !d.muted).count();
        for d in &self.drawn {
            if d.muted || d.points.is_empty() {
                continue;
            }
            let path = |points: &[(f64, f64)]| {
                Path::new(|b| {
                    let mut prev: Option<Point> = None;
                    for (x, y) in points {
                        let p = self.to_px(area, *x, y.clamp(self.y.0, self.y.1));
                        match prev {
                            None => b.move_to(p),
                            Some(q) if d.step => {
                                b.line_to(Point::new(p.x, q.y));
                                b.line_to(p);
                            }
                            Some(_) => b.line_to(p),
                        }
                        prev = Some(p);
                    }
                })
            };
            if visible == 1 {
                let fill = Path::new(|b| {
                    let base = area.y + area.height;
                    let first = self.to_px(area, d.points[0].0, self.y.0);
                    b.move_to(Point::new(first.x, base));
                    let mut prev: Option<Point> = None;
                    for (x, y) in &d.points {
                        let p = self.to_px(area, *x, y.clamp(self.y.0, self.y.1));
                        if let Some(q) = prev
                            && d.step
                        {
                            b.line_to(Point::new(p.x, q.y));
                        }
                        b.line_to(p);
                        prev = Some(p);
                    }
                    let last = self.to_px(area, d.points[d.points.len() - 1].0, self.y.0);
                    b.line_to(Point::new(last.x, base));
                    b.close();
                });
                frame.fill(&fill, Color { a: 0.12, ..d.color });
            }
            let stroke = |color: Color, dash: &'static [f32]| Stroke {
                line_dash: stroke::LineDash {
                    segments: dash,
                    offset: 0,
                },
                ..Stroke::default().with_color(color).with_width(1.5)
            };
            if d.segments.is_empty() {
                frame.stroke(&path(&d.points), stroke(d.color, d.dash));
            } else {
                for (n, (from, color, _)) in d.segments.iter().enumerate() {
                    let to = d
                        .segments
                        .get(n + 1)
                        .map(|s| s.0 + 1)
                        .unwrap_or(d.points.len())
                        .min(d.points.len());
                    if *from < to {
                        frame.stroke(&path(&d.points[*from..to]), stroke(*color, d.dash));
                    }
                }
            }
        }
    }

    fn readout(&self, frame: &mut Frame, area: Rectangle, x: f64, pinned: bool) {
        let px = self.to_px(area, x, self.y.0).x;
        let rule = Stroke {
            line_dash: stroke::LineDash {
                segments: if pinned { &[] } else { &[2.0, 3.0] },
                offset: 0,
            },
            ..Stroke::default()
                .with_color(if pinned {
                    theme::TEXT_SECONDARY
                } else {
                    theme::MUTED
                })
                .with_width(1.0)
        };
        frame.stroke(
            &Path::line(Point::new(px, area.y), Point::new(px, area.y + area.height)),
            rule,
        );
        let unit = self.x_unit.label();
        let mut lines: Vec<(String, Color)> = vec![(
            if unit.is_empty() {
                sig4(x)
            } else {
                format!("{} {unit}", sig4(x))
            },
            theme::TEXT,
        )];
        let values: Vec<(usize, f64, Option<String>)> = match &self.pinned {
            Some(p) if pinned => p.values.clone(),
            _ => self
                .drawn
                .iter()
                .enumerate()
                .filter(|(_, d)| !d.muted)
                .filter_map(|(i, d)| value_at(&d.points, d.step, x).map(|v| (i, v, None)))
                .collect(),
        };
        for (i, v, name) in values {
            let Some(d) = self.drawn.get(i) else {
                continue;
            };
            if d.muted {
                continue;
            }
            let label = match (&name, self.names.get(i)) {
                (Some(n), _) => format!("{}  {n}", self.format_y(v)),
                (None, Some(series)) => format!("{}  {series}", self.format_y(v)),
                _ => self.format_y(v),
            };
            lines.push((label, d.color));
        }
        let widest = lines
            .iter()
            .map(|(s, _)| ui::width(s, 11.0))
            .fold(0.0f32, f32::max)
            + 16.0;
        let height = lines.len() as f32 * 15.0 + 8.0;
        let left = px > area.x + area.width * 0.7;
        let bx = if left { px - 8.0 - widest } else { px + 8.0 };
        let by = (area.y + 8.0).min(area.y + area.height - height);
        frame.fill(
            &Path::rounded_rectangle(Point::new(bx, by), Size::new(widest, height), 3.0.into()),
            Color {
                a: 0.92,
                ..theme::OVERLAY
            },
        );
        for (n, (label, color)) in lines.iter().enumerate() {
            frame.fill_text(Text {
                content: label.clone(),
                position: Point::new(bx + 8.0, by + 5.0 + n as f32 * 15.0),
                color: *color,
                size: 11.0.into(),
                align_x: iced::alignment::Horizontal::Left.into(),
                align_y: iced::alignment::Vertical::Top,
                ..Text::default()
            });
        }
    }
}

impl canvas::Program<Message> for PlotProgram<'_> {
    type State = PlotState;

    fn update(
        &self,
        state: &mut PlotState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let area = self.area(bounds.size());
        let iced::Event::Mouse(event) = event else {
            return None;
        };
        let inside = cursor.position_in(bounds).filter(|p| area.contains(*p));
        match event {
            mouse::Event::CursorMoved { .. } => {
                let before = state.cursor;
                state.cursor = inside;
                if let (Some(start), Some(p)) = (state.panning, inside) {
                    let share = -((p.x - start) / area.width) as f64;
                    state.panning = Some(p.x);
                    if share != 0.0 {
                        return Some(
                            canvas::Action::publish(gm(GraphMsg::Pan(share))).and_capture(),
                        );
                    }
                }
                (before != inside).then(canvas::Action::request_redraw)
            }
            mouse::Event::CursorLeft => {
                state.cursor = None;
                state.panning = None;
                Some(canvas::Action::request_redraw())
            }
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let p = inside?;
                Some(
                    canvas::Action::publish(gm(GraphMsg::Pin(Some(self.x_of(area, p.x)))))
                        .and_capture(),
                )
            }
            mouse::Event::ButtonPressed(mouse::Button::Right) => {
                inside?;
                Some(canvas::Action::publish(gm(GraphMsg::Pin(None))).and_capture())
            }
            mouse::Event::ButtonPressed(mouse::Button::Middle) => {
                let p = inside?;
                state.panning = Some(p.x);
                Some(canvas::Action::capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Middle) => {
                state.panning = None;
                None
            }
            mouse::Event::WheelScrolled { delta } => {
                let p = inside?;
                let notches = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                if notches == 0.0 {
                    return None;
                }
                let factor = if notches > 0.0 { 0.85 } else { 1.0 / 0.85 };
                let at = self.x_of(area, p.x);
                Some(canvas::Action::publish(gm(GraphMsg::Zoom(factor, at))).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &PlotState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let plot = self
            .cache
            .draw(renderer, bounds.size(), |frame| self.draw_static(frame));
        let mut top = Frame::new(renderer, bounds.size());
        let area = self.area(bounds.size());
        if let Some(p) = &self.pinned {
            self.readout(&mut top, area, p.x, true);
        }
        if let Some(c) = state.cursor
            && self.pinned.is_none()
        {
            self.readout(&mut top, area, self.x_of(area, c.x), false);
        }
        vec![plot, top.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &PlotState,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.cursor.is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FitChoice {
    doc: usize,
    name: String,
}

impl std::fmt::Display for FitChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModeChoice(ResistMode);

impl std::fmt::Display for ModeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.label())
    }
}

fn target_strip(app: &App) -> El<'_> {
    let g = &app.graph;
    let mut line = row![ui::label("TARGET")]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
    for (i, p) in app.targets.iter().enumerate() {
        line = line.add(
            button(text(&p.name).size(ui::MICRO))
                .padding(ui::pill_tight(6.0))
                .style(theme::chip(g.targets_on.contains(&i)))
                .on_press(gm(GraphMsg::Target(i))),
        );
    }
    let open: Vec<FitChoice> = app
        .docs
        .iter()
        .enumerate()
        .filter(|(i, _)| !g.fit_targets.iter().any(|t| t.doc == *i))
        .map(|(i, d)| FitChoice {
            doc: i,
            name: app.fit_title(d),
        })
        .collect();
    if !open.is_empty() {
        line = line.add(
            pick_list(None::<FitChoice>, open, ToString::to_string)
                .on_select(|c: FitChoice| gm(GraphMsg::FitTarget(c.doc)))
                .placeholder("+ fit")
                .text_size(ui::MICRO)
                .padding(ui::field(2.0, 6.0))
                .style(theme::picker),
        );
    }
    for (i, t) in g.fit_targets.iter().enumerate() {
        let Some(doc) = app.docs.get(t.doc) else {
            continue;
        };
        let modes: Vec<ModeChoice> = ResistMode::ALL.iter().map(|m| ModeChoice(*m)).collect();
        line = line.add(
            row![
                button(text(app.fit_title(doc)).size(ui::MICRO))
                    .padding(ui::pill_tight(6.0))
                    .style(theme::chip(true))
                    .on_press(gm(GraphMsg::RemoveFitTarget(i))),
                pick_list(Some(ModeChoice(t.mode)), modes, ToString::to_string)
                    .on_select(move |m: ModeChoice| gm(GraphMsg::ResistMode(i, m.0)))
                    .text_size(ui::MICRO)
                    .padding(ui::field(2.0, 6.0))
                    .style(theme::picker),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center),
        );
    }
    line = line.add(
        button(
            text(if g.editor_open {
                "Done"
            } else {
                "Edit profiles"
            })
            .size(ui::MICRO),
        )
        .padding(ui::pill_tight(6.0))
        .style(theme::ghost)
        .on_press(gm(GraphMsg::Editor(!g.editor_open))),
    );
    line.wrap().boxed()
}

fn profile_editor(app: &App) -> El<'_> {
    let field = |placeholder: &'static str, value: &str, width: f32, on: fn(String) -> Message| {
        text_input(placeholder, value.to_string())
            .on_input(on)
            .size(ui::SMALL)
            .padding(ui::field(3.0, 6.0))
            .width(width)
            .style(theme::input)
    };
    let mut line = row![
        ui::label("PROFILE"),
        field("name", &app.target_name, 120.0, Message::TargetName),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    for (i, label) in ["EM", "TH", "KI", "EX"].iter().enumerate() {
        line = line.add(ui::label(*label));
        line = line.add(
            text_input("0%", app.target_resists[i].clone())
                .on_input(move |s| Message::TargetResist(i, s))
                .size(ui::SMALL)
                .padding(ui::field(3.0, 6.0))
                .width(46)
                .style(theme::input),
        );
    }
    for (i, (label, placeholder)) in [
        ("SIG", "m"),
        ("SPEED", "m/s"),
        ("RADIUS", "m"),
        ("HP", "any"),
    ]
    .iter()
    .enumerate()
    {
        line = line.add(ui::label(*label));
        line = line.add(
            text_input(*placeholder, app.target_numbers[i].clone())
                .on_input(move |s| Message::TargetNumber(i, s))
                .size(ui::SMALL)
                .padding(ui::field(3.0, 6.0))
                .width(60)
                .style(theme::input),
        );
    }
    line = line.add(
        button(text("Save").size(ui::SMALL))
            .padding(ui::pill(ui::PAD_WIDE))
            .style(theme::secondary)
            .on_press(Message::SaveTargetProfile),
    );
    let builtin = rufa_store::targets::builtin();
    for (i, p) in app.targets.iter().enumerate() {
        if builtin.iter().any(|b| b.name == p.name) {
            continue;
        }
        line = line.add(
            button(text(format!("\u{00d7} {}", p.name)).size(ui::MICRO))
                .padding(ui::pill_tight(ui::GAP))
                .style(theme::ghost)
                .on_press(Message::DeleteTargetProfile(i)),
        );
    }
    line.wrap().boxed()
}

fn input_strip<'a>(app: &'a App, misc: &[&'static rufa_sim::graph::InputDef]) -> El<'a> {
    let g = &app.graph;
    let mut line = row![ui::label("INPUTS")]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
    for def in misc {
        let key = def.key;
        line = line.add(ui::small(def.label));
        line = line.add(
            text_input(def.blank.unwrap_or(""), g.input_text(key).to_string())
                .on_input(move |s| gm(GraphMsg::Input(key, s)))
                .size(ui::SMALL)
                .padding(ui::field(3.0, 6.0))
                .width(64)
                .style(theme::input),
        );
        if def.unit != Unit::None {
            line = line.add(ui::label(def.unit.label()));
        }
    }
    if g.req.kind.has_vectors() {
        let x = g.req.x_axis();
        let direction_only = x.q == Quantity::TgtSpeed;
        for (attacker, label) in [(true, "ATTACKER"), (false, "TARGET")] {
            let v = if attacker {
                g.req.vectors.attacker
            } else {
                g.req.vectors.target
            };
            let dial = canvas_widget(Dial {
                attacker,
                vector: v,
                direction_only: !attacker && direction_only,
                snap: app.modifiers.shift(),
            })
            .width(44)
            .height(44);
            let caption = if !attacker && direction_only {
                format!("{:.0}\u{00b0}", v.angle)
            } else {
                format!("{:.0}%  {:.0}\u{00b0}", v.length * 100.0, v.angle)
            };
            line = line.add(
                row![ui::label(label), dial, ui::small(caption)]
                    .spacing(ui::GAP)
                    .align_y(iced::Alignment::Center),
            );
        }
    }
    line.wrap().boxed()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DroneChoice(DroneMode);

impl std::fmt::Display for DroneChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.label())
    }
}

fn option_strip<'a>(app: &'a App, toggles: &[Toggle]) -> El<'a> {
    let o = &app.graph.req.options;
    let mut line = row![ui::label("OPTIONS")]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
    let chip = |label: &str, on: bool, toggle: Toggle| {
        button(
            text(format!(
                "{} {label}",
                if on { "\u{25c9}" } else { "\u{25cb}" }
            ))
            .size(ui::MICRO),
        )
        .padding(ui::pill_tight(6.0))
        .style(theme::chip(on))
        .on_press(gm(GraphMsg::Toggle(toggle)))
    };
    for t in toggles {
        match t {
            Toggle::Resists => line = line.add(chip("Resists", o.resists, *t)),
            Toggle::Projected => line = line.add(chip("Projected", o.projected, *t)),
            Toggle::LockRange => line = line.add(chip("Lock range", o.lock_range, *t)),
            Toggle::DroneRange => line = line.add(chip("Drone range", o.drone_range, *t)),
            Toggle::Capsim => line = line.add(chip("Capsim", o.capsim, *t)),
            Toggle::AncillaryReload => {
                line = line.add(chip("Reload ancillary", o.ancillary_reload, *t))
            }
            Toggle::Ehp => line = line.add(chip("EHP", o.ehp, *t)),
            Toggle::DroneMode => {
                let modes: Vec<DroneChoice> =
                    DroneMode::ALL.iter().map(|m| DroneChoice(*m)).collect();
                line = line.add(ui::label("DRONES"));
                line = line.add(
                    pick_list(Some(DroneChoice(o.drone_mode)), modes, ToString::to_string)
                        .on_select(|m: DroneChoice| gm(GraphMsg::DroneMode(m.0)))
                        .text_size(ui::MICRO)
                        .padding(ui::field(2.0, 6.0))
                        .style(theme::picker),
                );
            }
            Toggle::AmmoTier => {
                line = line.add(ui::label("AMMO"));
                line = line.add(ui::segmented(
                    AmmoTier::ALL
                        .iter()
                        .map(|t| {
                            (
                                t.label().to_string(),
                                o.ammo_tier == *t,
                                gm(GraphMsg::AmmoTier(*t)),
                            )
                        })
                        .collect(),
                ));
            }
        }
    }
    line.wrap().boxed()
}

struct Dial {
    attacker: bool,
    vector: Vector,
    direction_only: bool,
    snap: bool,
}

#[derive(Default)]
struct DialState {
    dragging: bool,
}

impl Dial {
    fn vector_at(&self, bounds: Rectangle, p: Point) -> Vector {
        let c = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let r = bounds.width.min(bounds.height) / 2.0 - 2.0;
        let dx = (p.x - c.x) as f64;
        let dy = (c.y - p.y) as f64;
        let mut angle = dy.atan2(dx).to_degrees();
        let mut length = ((dx * dx + dy * dy).sqrt() / r as f64).min(1.0);
        if self.snap {
            angle = (angle / 15.0).round() * 15.0;
            length = (length / 0.05).floor() * 0.05;
        }
        if length < 0.01 {
            length = 0.0;
            angle = self.vector.angle;
        }
        Vector {
            length: if self.direction_only { 1.0 } else { length },
            angle,
        }
    }
}

impl canvas::Program<Message> for Dial {
    type State = DialState;

    fn update(
        &self,
        state: &mut DialState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let iced::Event::Mouse(event) = event else {
            return None;
        };
        let publish = |v: Vector| {
            Some(canvas::Action::publish(gm(GraphMsg::Vector(self.attacker, v))).and_capture())
        };
        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let p = cursor.position_in(bounds)?;
                state.dragging = true;
                publish(self.vector_at(bounds, p))
            }
            mouse::Event::CursorMoved { .. } if state.dragging => {
                let p = cursor.position_in(bounds).or_else(|| {
                    cursor
                        .position()
                        .map(|q| Point::new(q.x - bounds.x, q.y - bounds.y))
                })?;
                publish(self.vector_at(bounds, p))
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                state.dragging = false;
                None
            }
            mouse::Event::WheelScrolled { delta } if !self.direction_only => {
                cursor.position_in(bounds)?;
                let notches = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                let length = (self.vector.length + 0.1 * notches.signum() as f64).clamp(0.0, 1.0);
                publish(Vector {
                    length,
                    angle: self.vector.angle,
                })
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &DialState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let r = bounds.width.min(bounds.height) / 2.0 - 2.0;
        frame.fill(&Path::circle(c, r), theme::SUNKEN);
        frame.stroke(
            &Path::circle(c, r),
            Stroke::default().with_color(theme::EDGE).with_width(1.0),
        );
        let a = self.vector.angle.to_radians();
        let len = if self.direction_only {
            1.0
        } else {
            self.vector.length
        } as f32;
        let tip = Point::new(
            c.x + a.cos() as f32 * len * r,
            c.y - a.sin() as f32 * len * r,
        );
        frame.stroke(
            &Path::line(c, tip),
            Stroke::default().with_color(theme::ACCENT).with_width(1.5),
        );
        frame.fill(&Path::circle(tip, 2.5), theme::ACCENT);
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &DialState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
