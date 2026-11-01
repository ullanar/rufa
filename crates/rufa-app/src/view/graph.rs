use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::{button, canvas as canvas_widget, column, row, text, text_input};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};
use rufa_sim::graph::{self, Curve};

use crate::fmt;
use crate::theme;
use crate::ui;
use crate::{App, GraphKind, Message};

const COLORS: [Color; 6] = theme::SERIES;

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(ids) = app.ids else {
        return text("No game data").into();
    };
    let kinds = ui::segmented(
        [
            ("DPS over distance", GraphKind::DpsOverDistance),
            ("Capacitor over time", GraphKind::CapOverTime),
        ]
        .into_iter()
        .map(|(label, which)| {
            (
                label.to_string(),
                app.graph_kind == which,
                Message::GraphKind(which),
            )
        })
        .collect(),
    );
    if app.graph_kind == GraphKind::CapOverTime {
        return cap_view(app, kinds);
    }
    let target = app.graph_target();
    let mut reach: f64 = 0.0;
    for doc in &app.docs {
        reach = reach.max(graph::reach(&ids, &doc.doc.fit, &doc.doc.calc));
    }
    let max_distance = if reach > 0.0 { reach * 1.05 } else { 50_000.0 };
    let series: Vec<(String, Curve)> = app
        .docs
        .iter()
        .map(|doc| {
            (
                app.fit_title(doc),
                graph::dps_curve(
                    &ids,
                    &doc.doc.fit,
                    &doc.doc.calc,
                    &target,
                    max_distance,
                    240,
                ),
            )
        })
        .collect();

    let mut legend = row![].spacing(12);
    for (i, (name, _)) in series.iter().enumerate() {
        legend = legend.push(
            text(name.clone())
                .size(ui::BODY)
                .font(theme::UI_MEDIUM)
                .color(COLORS[i % COLORS.len()]),
        );
    }
    let builtin = rufa_store::targets::builtin();
    let mut profiles = row![ui::label("TARGET")]
        .spacing(3)
        .align_y(iced::Alignment::Center);
    for (i, p) in app.targets.iter().enumerate() {
        profiles = profiles.push(
            button(text(&p.name).size(ui::MICRO))
                .padding(ui::pill_tight(6.0))
                .style(theme::chip(app.target == i))
                .on_press(Message::TargetProfile(i)),
        );
        if !builtin.iter().any(|b| b.name == p.name) {
            profiles = profiles.push(
                button(text("\u{00d7}").size(ui::MICRO))
                    .padding(ui::pill_tight(ui::GAP))
                    .style(theme::ghost)
                    .on_press(Message::DeleteTargetProfile(i)),
            );
        }
    }
    let mut editor = row![
        ui::label("NEW PROFILE"),
        text_input("name", &app.target_name)
            .on_input(Message::TargetName)
            .size(ui::SMALL)
            .padding(ui::field(3.0, 6.0))
            .width(120)
            .style(theme::input),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    for (i, label) in ["EM", "Th", "Kin", "Exp"].iter().enumerate() {
        editor = editor.push(
            row![
                ui::label(*label),
                text_input("0%", &app.target_resists[i])
                    .on_input(move |s| Message::TargetResist(i, s))
                    .size(ui::SMALL)
                    .padding(ui::field(3.0, 6.0))
                    .width(46)
                    .style(theme::input),
            ]
            .spacing(ui::GAP_ROW)
            .align_y(iced::Alignment::Center),
        );
    }
    editor = editor.push(
        button(text("Save").size(ui::SMALL))
            .padding(ui::pill(ui::PAD_WIDE))
            .style(theme::secondary)
            .on_press(Message::SaveTargetProfile),
    );
    let controls = row![
        row![
            ui::label("SIGNATURE, M"),
            text_input("inf", &app.graph_signature)
                .on_input(Message::GraphSignature)
                .size(ui::SMALL)
                .padding(ui::field(3.0, 6.0))
                .width(70)
                .style(theme::input),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
        row![
            ui::label("VELOCITY, M/S"),
            text_input("0", &app.graph_velocity)
                .on_input(Message::GraphVelocity)
                .size(ui::SMALL)
                .padding(ui::field(3.0, 6.0))
                .width(70)
                .style(theme::input),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);

    let max_y = series
        .iter()
        .flat_map(|(_, c)| c.points.iter().map(|p| p.1))
        .fold(0.0f64, f64::max)
        .max(1.0);
    let plot = canvas_widget(Plot {
        series,
        max_x: max_distance,
        max_y,
        x_label: fmt::distance,
        y_label: |v| fmt::num(v, 0),
    })
    .width(Length::Fill)
    .height(Length::Fill);
    column![
        kinds,
        profiles.wrap(),
        controls.wrap(),
        editor.wrap(),
        legend,
        plot
    ]
    .spacing(ui::GAP_ROW)
    .height(Length::Fill)
    .into()
}

fn cap_view<'a>(app: &'a App, kinds: Element<'a, Message>) -> Element<'a, Message> {
    let horizon = rufa_sim::document::CAP_HORIZON;
    let series: Vec<(String, Curve)> = app
        .docs
        .iter()
        .map(|doc| {
            let capacity = doc.doc.stats.capacitor.capacity.max(1.0);
            let points = doc
                .doc
                .cap
                .series
                .iter()
                .map(|(t, cap)| (*t, cap / capacity * 100.0))
                .collect();
            (app.fit_title(doc), Curve { points })
        })
        .collect();
    let mut legend = row![].spacing(12);
    for (i, (name, _)) in series.iter().enumerate() {
        legend = legend.push(
            text(name.clone())
                .size(ui::BODY)
                .font(theme::UI_MEDIUM)
                .color(COLORS[i % COLORS.len()]),
        );
    }
    let plot = canvas_widget(Plot {
        series,
        max_x: horizon,
        max_y: 100.0,
        x_label: fmt::seconds,
        y_label: |v| format!("{v:.0}%"),
    })
    .width(Length::Fill)
    .height(Length::Fill);
    column![kinds, legend, plot]
        .spacing(6)
        .height(Length::Fill)
        .into()
}

struct Plot {
    series: Vec<(String, Curve)>,
    max_x: f64,
    max_y: f64,
    x_label: fn(f64) -> String,
    y_label: fn(f64) -> String,
}

const MARGIN_LEFT: f32 = 48.0;
const MARGIN_BOTTOM: f32 = 24.0;
const MARGIN_TOP: f32 = 10.0;
const MARGIN_RIGHT: f32 = 12.0;

impl canvas::Program<Message> for Plot {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let _ = theme;
        let axis_color = crate::theme::HAIRLINE;
        let text_color = crate::theme::MUTED;

        let max_dps = self.max_y;
        let plot = Rectangle {
            x: MARGIN_LEFT,
            y: MARGIN_TOP,
            width: (bounds.width - MARGIN_LEFT - MARGIN_RIGHT).max(1.0),
            height: (bounds.height - MARGIN_TOP - MARGIN_BOTTOM).max(1.0),
        };
        let to_point = |d: f64, dps: f64| {
            Point::new(
                plot.x + (d / self.max_x) as f32 * plot.width,
                plot.y + plot.height - (dps / max_dps) as f32 * plot.height,
            )
        };

        let grid = Stroke::default().with_color(axis_color).with_width(1.0);
        for i in 0..=4 {
            let dps = max_dps * i as f64 / 4.0;
            let y = to_point(0.0, dps).y;
            frame.stroke(
                &Path::line(Point::new(plot.x, y), Point::new(plot.x + plot.width, y)),
                grid,
            );
            frame.fill_text(Text {
                content: (self.y_label)(dps),
                position: Point::new(plot.x - 4.0, y),
                color: text_color,
                size: 10.0.into(),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }
        for i in 0..=5 {
            let d = self.max_x * i as f64 / 5.0;
            let x = to_point(d, 0.0).x;
            frame.stroke(
                &Path::line(Point::new(x, plot.y), Point::new(x, plot.y + plot.height)),
                grid,
            );
            let align = match i {
                0 => iced::alignment::Horizontal::Left,
                5 => iced::alignment::Horizontal::Right,
                _ => iced::alignment::Horizontal::Center,
            };
            frame.fill_text(Text {
                content: (self.x_label)(d),
                position: Point::new(x, plot.y + plot.height + 4.0),
                color: text_color,
                size: 10.0.into(),
                align_x: align.into(),
                align_y: iced::alignment::Vertical::Top,
                ..Text::default()
            });
        }

        let baseline = plot.y + plot.height;
        for (i, (_, curve)) in self.series.iter().enumerate() {
            let color = COLORS[i % COLORS.len()];
            let line = Path::new(|b| {
                for (n, (d, dps)) in curve.points.iter().enumerate() {
                    let p = to_point(*d, *dps);
                    if n == 0 {
                        b.move_to(p);
                    } else {
                        b.line_to(p);
                    }
                }
            });
            if let (Some(first), Some(last)) = (curve.points.first(), curve.points.last()) {
                let area = Path::new(|b| {
                    b.move_to(Point::new(to_point(first.0, 0.0).x, baseline));
                    for (d, dps) in &curve.points {
                        b.line_to(to_point(*d, *dps));
                    }
                    b.line_to(Point::new(to_point(last.0, 0.0).x, baseline));
                    b.close();
                });
                frame.fill(&area, Color { a: 0.12, ..color });
            }
            frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
        }
        vec![frame.into_geometry()]
    }
}
