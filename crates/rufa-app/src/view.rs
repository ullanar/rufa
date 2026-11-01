mod browser;
mod fit;
mod graph;
mod stats;

use iced::widget::{button, column, container, pane_grid, row, space, text, text_input};
use iced::{Element, Length};

use crate::theme;
use crate::ui::{self, El};
use crate::{App, CenterTab, CharChoice, Message, Pane};

pub fn root(app: &App) -> Element<'_, Message> {
    if app.downloading || app.download_error.is_some() {
        return first_run(app);
    }
    let grid = pane_grid(&app.panes, |_, pane, _| {
        let body: El<'_> = match pane {
            Pane::Browser => browser::view(app),
            Pane::Fit => center(app),
            Pane::Stats => stats::view(app),
        };
        pane_grid::Content::new(
            container(body)
                .padding(ui::PAD)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::pane),
        )
    })
    .spacing(ui::GAP)
    .min_size(220)
    .on_resize(8, Message::PaneResized);

    column![
        toolbar(app),
        container(grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0.0, ui::GAP]),
        container(fit::violations(app)).padding([0.0, ui::GAP]),
        status_bar(app),
    ]
    .spacing(ui::GAP)
    .into()
}

fn first_run(app: &App) -> El<'_> {
    use crate::fmt;
    let total: u64 = app.download.iter().map(|d| d.total.unwrap_or(0)).sum();
    let received: u64 = app.download.iter().map(|d| d.received).sum();
    let mut files = column![].spacing(ui::GAP_ROW);
    for name in rufa_data::update::FILES {
        let (done, fraction, size) = match app.download.iter().find(|d| d.file == name) {
            Some(d) => {
                let fraction = d
                    .total
                    .map(|t| d.received as f32 / t.max(1) as f32)
                    .unwrap_or(0.0);
                let size = match d.total {
                    Some(t) => format!(
                        "{} / {} MB",
                        fmt::num(d.received as f64 / 1e6, 1),
                        fmt::num(t as f64 / 1e6, 1)
                    ),
                    None => format!("{} MB", fmt::num(d.received as f64 / 1e6, 1)),
                };
                (fraction >= 1.0, fraction, size)
            }
            None => (false, 0.0, "waiting".to_string()),
        };
        let color = if done {
            theme::SUCCESS_TEXT
        } else {
            theme::ACCENT
        };
        files = files.push(
            column![
                row![
                    ui::body(describe_file(name)).width(Length::Fill),
                    ui::small(size)
                ]
                .align_y(iced::Alignment::Center),
                ui::bar(fraction, color, 6.0),
            ]
            .spacing(4),
        );
    }
    let headline = match &app.download_error {
        Some(_) => "The download did not finish".to_string(),
        None if total > 0 => format!(
            "Getting the game data  ·  {:.0}%",
            received as f64 / total as f64 * 100.0
        ),
        None => "Getting the game data".to_string(),
    };
    let mut card = column![
        row![
            iced::widget::canvas(Spinner { angle: app.spin, failed: app.download_error.is_some() })
                .width(36)
                .height(36),
            column![
                text("RUFA").font(theme::LABEL).size(ui::TITLE).color(theme::ACCENT),
                ui::title(headline),
            ]
            .spacing(2),
        ]
        .spacing(ui::PAD)
        .align_y(iced::Alignment::Center),
        ui::muted("Ship, module and attribute tables from EVEShipFit's sde-patched. This happens once; later starts are instant."),
        files,
    ]
    .spacing(16)
    .max_width(520);
    if let Some(error) = &app.download_error {
        card = card.push(
            container(
                column![
                    ui::body(error.clone()),
                    ui::muted(format!("Files go to {}", rufa_data::data_dir().display()))
                ]
                .spacing(4),
            )
            .padding([8, 12])
            .width(Length::Fill)
            .style(theme::banner(theme::DANGER_FILL, theme::DANGER_LINE)),
        );
        card = card.push(
            button(text("Try again").size(ui::BODY))
                .padding([6, 14])
                .style(theme::primary)
                .on_press(Message::RetryDownload),
        );
    } else {
        card = card.push(ui::small(format!(
            "Saved to {}",
            rufa_data::data_dir().display()
        )));
    }
    container(container(card).padding(28).style(theme::card))
        .center(Length::Fill)
        .into()
}

fn describe_file(name: &str) -> &'static str {
    match name {
        rufa_data::SDE_FILE => "Game data: types, attributes, effects",
        rufa_data::NAMES_FILE => "Type names in every language",
        _ => "Attribute tooltips and hull bonuses",
    }
}

struct Spinner {
    angle: f32,
    failed: bool,
}

impl iced::widget::canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        _: &iced::Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::{Frame, Path, Stroke};
        let mut frame = Frame::new(renderer, bounds.size());
        let center = iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 3.0;
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default().with_color(theme::TRACK).with_width(3.0),
        );
        if self.failed {
            frame.stroke(
                &Path::circle(center, radius),
                Stroke::default()
                    .with_color(theme::DANGER_TEXT)
                    .with_width(3.0),
            );
            return vec![frame.into_geometry()];
        }
        let arc = Path::new(|b| {
            b.arc(iced::widget::canvas::path::Arc {
                center,
                radius,
                start_angle: iced::Radians(self.angle),
                end_angle: iced::Radians(self.angle + 1.6),
            });
        });
        frame.stroke(
            &arc,
            Stroke::default().with_color(theme::ACCENT).with_width(3.0),
        );
        vec![frame.into_geometry()]
    }
}

fn center(app: &App) -> El<'_> {
    let tab = |label: &'static str, which: CenterTab| {
        button(text(label).size(ui::BODY).font(theme::UI_MEDIUM))
            .padding([3, 10])
            .style(theme::chip(app.center == which))
            .on_press(Message::CenterTab(which))
    };
    let tabs = row![tab("Fit", CenterTab::Fit), tab("Graph", CenterTab::Graph)].spacing(ui::GAP);
    let body = match app.center {
        CenterTab::Fit => fit::view(app),
        CenterTab::Graph => graph::view(app),
    };
    column![tabs, body]
        .spacing(ui::GAP_ROW)
        .height(Length::Fill)
        .into()
}

fn toolbar(app: &App) -> El<'_> {
    let has_fit = !app.docs.is_empty();
    let action = |label: &'static str, msg: Message| {
        let b = button(text(label).size(ui::BODY))
            .padding([4, 10])
            .style(theme::ghost);
        if has_fit { b.on_press(msg) } else { b }
    };
    let name = app
        .docs
        .get(app.active)
        .and_then(|d| d.doc.fit.name.clone())
        .unwrap_or_default();
    let mut bar = row![
        container(
            text("RUFA")
                .font(theme::LABEL)
                .size(ui::VALUE)
                .color(theme::ACCENT)
        )
        .padding([0, 6]),
        text_input("Fit name", &name)
            .on_input(Message::RenameFit)
            .width(260)
            .size(ui::BODY)
            .padding([5, 8])
            .style(theme::input),
        action("Save", Message::Save),
        action("Undo", Message::Undo),
        action("Redo", Message::Redo),
        action("Copy EFT", Message::CopyEft),
        button(text("Paste EFT").size(ui::BODY))
            .padding([4, 10])
            .style(theme::ghost)
            .on_press(Message::PasteEft),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);

    bar = bar.push(space().width(Length::Fill));
    bar = bar.push(ui::label("SKILLS"));
    for level in [0u8, 5] {
        bar = bar.push(
            button(text(format!("All {level}")).size(ui::SMALL))
                .padding([3, 9])
                .style(theme::chip(app.current == CharChoice::All(level)))
                .on_press(Message::SkillLevel(level)),
        );
    }
    for (i, c) in app.characters.iter().enumerate() {
        bar = bar.push(
            button(text(&c.name).size(ui::SMALL))
                .padding([3, 9])
                .style(theme::chip(app.current == CharChoice::Saved(i)))
                .on_press(Message::ChooseCharacter(CharChoice::Saved(i))),
        );
    }
    container(bar)
        .padding([0.0, ui::PAD])
        .height(ui::TOOLBAR)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center)
        .style(theme::toolbar)
        .into()
}

fn status_bar(app: &App) -> El<'_> {
    let mut bar = row![ui::small(&app.status).width(Length::Fill)]
        .spacing(ui::PAD)
        .align_y(iced::Alignment::Center);
    if app.prices_pending {
        bar = bar.push(ui::label("prices"));
    }
    let scale = app.settings.ui_scale;
    bar = bar.push(ui::label("ZOOM"));
    bar = bar.push(
        button(text("−").size(ui::SMALL))
            .padding([1, 7])
            .style(theme::ghost)
            .on_press(Message::UiScale(scale - 0.1)),
    );
    bar = bar.push(ui::small(format!("{:.0}%", scale * 100.0)));
    bar = bar.push(
        button(text("+").size(ui::SMALL))
            .padding([1, 7])
            .style(theme::ghost)
            .on_press(Message::UiScale(scale + 0.1)),
    );
    if let Some(tag) = &app.sde_update {
        bar = bar.push(
            button(text(format!("Update game data to {tag}")).size(ui::SMALL))
                .padding([2, 8])
                .style(theme::secondary)
                .on_press(Message::UpdateSde),
        );
    }
    container(bar)
        .padding([0.0, ui::PAD])
        .height(ui::STATUS)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center)
        .style(theme::toolbar)
        .into()
}
