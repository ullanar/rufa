mod browser;
mod fit;
mod graph;
mod stats;

use iced::widget::Widget;
use iced::widget::{button, column, container, pane_grid, row, space, stack, text, text_input};
use iced::{Element, Length};

use crate::theme;
use crate::ui::Add;
use crate::ui::{self, El};
use crate::{App, CenterTab, CharChoice, Message, Pane};

pub fn root(app: &App) -> Element<'_, Message> {
    if app.setup_prompt {
        return install_screen(app);
    }
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

    let body = column![
        toolbar(app),
        container(grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0.0, ui::GAP]),
        container(fit::violations(app)).padding([0.0, ui::GAP]),
        status_bar(app),
    ]
    .spacing(ui::GAP);
    if app.update_prompt {
        return stack![body, update_window(app)].boxed();
    }
    body.boxed()
}

fn paragraph(s: String) -> El<'static> {
    text(s)
        .size(ui::SMALL)
        .color(theme::TEXT_SECONDARY)
        .width(Length::Fill)
        .boxed()
}

fn install_screen(app: &App) -> Element<'_, Message> {
    let mut card = ui::col().spacing(ui::GAP_ROW).width(Length::Fill);
    match &app.setup_done {
        Some(placed) => {
            card = card.add(ui::eyebrow("Installed"));
            card = card.add(paragraph(placed.to_string_lossy().to_string()));
            card = card.add(paragraph(
                "The menu entry and the icon are in place on Linux.".to_string(),
            ));
            card = card.add(ui::rule());
            card = card.add(
                row![
                    button(text("Start Rufa").size(ui::SMALL))
                        .padding(ui::pill_tall(ui::PAD_WIDE))
                        .style(theme::primary)
                        .on_press(Message::SetupLaunch),
                    button(text("Close").size(ui::SMALL))
                        .padding(ui::pill_tall(ui::PAD_WIDE))
                        .style(theme::ghost)
                        .on_press(Message::SetupSkip),
                ]
                .spacing(ui::GAP),
            );
        }
        None => {
            card = card.add(ui::eyebrow(if app.setup_replacing {
                "Replace the installed copy"
            } else {
                "Install Rufa"
            }));
            card = card.add(paragraph(
                if app.setup_replacing {
                    "Rufa is already installed. This build can take its place, with no download and no network."
                } else {
                    "This build runs from where it was downloaded. Installing copies it into your own directory, so no administrator rights are needed."
                }
                .to_string(),
            ));
            card = card.add(
                text_input("install directory", &app.setup_dir)
                    .on_input(Message::SetupDir)
                    .padding(ui::GAP_ROW)
                    .size(ui::SMALL),
            );
            if let Some(error) = &app.setup_error {
                card = card.add(
                    text(error.clone())
                        .size(ui::SMALL)
                        .color(theme::DANGER_TEXT),
                );
            }
            card = card.add(ui::rule());
            card = card.add(
                row![
                    button(
                        text(if app.setup_replacing {
                            "Replace"
                        } else {
                            "Install"
                        })
                        .size(ui::SMALL)
                    )
                    .padding(ui::pill_tall(ui::PAD_WIDE))
                    .style(theme::primary)
                    .on_press(Message::SetupInstall),
                    button(text("Run without installing").size(ui::SMALL))
                        .padding(ui::pill_tall(ui::PAD_WIDE))
                        .style(theme::ghost)
                        .on_press(Message::SetupSkip),
                ]
                .spacing(ui::GAP),
            );
        }
    }
    container(
        container(card)
            .padding(ui::PAD * 2.0)
            .style(theme::card)
            .width(460),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .boxed()
}

fn update_window(app: &App) -> El<'_> {
    let installed = env!("CARGO_PKG_VERSION");
    let mut card = ui::col()
        .spacing(ui::GAP_ROW)
        .width(Length::Shrink.max(380));
    match &app.app_update {
        Some(release) => {
            card = card.add(ui::eyebrow("Update available"));
            card = card.add(fact("Installed", format!("v{installed}")));
            card = card.add(fact("Available", release.tag.clone()));
            if !release.published.is_empty() {
                card = card.add(fact("Released", release.published.clone()));
            }
            card = card.add(notes_link("Read what changed"));
        }
        None => {
            card = card.add(ui::eyebrow("No update"));
            card = card.add(fact("Installed", format!("v{installed}")));
            card = card.add(ui::small("This is the latest release."));
            card = card.add(notes_link("Read what it carries"));
        }
    }
    if let Some(error) = &app.update_error {
        card = card.add(ui::rule());
        card = card.add(
            text(error.clone())
                .size(ui::SMALL)
                .color(theme::DANGER_TEXT),
        );
    }
    card = card.add(ui::rule());
    card = card.add(update_buttons(app));
    container(
        container(card)
            .padding(ui::PAD * 2.0)
            .style(theme::card)
            .width(Length::Shrink.max(420)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(theme::scrim)
    .boxed()
}

fn notes_link<'a>(label: &'a str) -> El<'a> {
    button(text(label).size(ui::SMALL))
        .padding(ui::pill_tight(0.0))
        .style(theme::link)
        .on_press(Message::ReleaseNotes)
        .boxed()
}

fn fact<'a>(name: &'a str, value: String) -> El<'a> {
    row![ui::small(name).width(Length::Fill), ui::value(value)]
        .align_y(iced::Alignment::Center)
        .boxed()
}

fn update_buttons(app: &App) -> El<'_> {
    let mut line = ui::line().spacing(ui::GAP);
    if let Some(tag) = &app.update_ready {
        line = line.add(
            button(text(format!("Restart into {tag}")).size(ui::SMALL))
                .padding(ui::pill_tall(ui::PAD_WIDE))
                .style(theme::primary)
                .on_press(Message::Restart),
        );
        line = line.add(
            button(text("Later").size(ui::SMALL))
                .padding(ui::pill_tall(ui::PAD_WIDE))
                .style(theme::ghost)
                .on_press(Message::UpdateLater),
        );
        return line.boxed();
    }
    if app.app_update.is_some() {
        let now = button(
            text(if app.update_busy {
                "Downloading"
            } else {
                "Update now"
            })
            .size(ui::SMALL),
        )
        .padding(ui::pill_tall(ui::PAD_WIDE))
        .style(theme::primary);
        line = line.add(if app.update_busy {
            now
        } else {
            now.on_press(Message::UpdateNow)
        });
        line = line.add(
            button(text("Next time").size(ui::SMALL))
                .padding(ui::pill_tall(ui::PAD_WIDE))
                .style(theme::secondary)
                .on_press(Message::UpdateLater),
        );
        line = line.add(
            button(text("Skip this version").size(ui::SMALL))
                .padding(ui::pill_tall(ui::PAD_WIDE))
                .style(theme::ghost)
                .on_press(Message::UpdateSkip),
        );
        return line.boxed();
    }
    line.add(
        button(text("Close").size(ui::SMALL))
            .padding(ui::pill_tall(ui::PAD_WIDE))
            .style(theme::secondary)
            .on_press(Message::UpdateLater),
    )
    .boxed()
}

fn first_run(app: &App) -> El<'_> {
    use crate::fmt;
    let total: u64 = app.download.iter().map(|d| d.total.unwrap_or(0)).sum();
    let received: u64 = app.download.iter().map(|d| d.received).sum();
    let mut files = ui::col().spacing(ui::GAP_ROW);
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
        files = files.add(
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
    .width(Length::Shrink.max(520));
    if let Some(error) = &app.download_error {
        card = card.add(
            container(
                column![
                    ui::body(error.clone()),
                    ui::muted(format!("Files go to {}", rufa_data::data_dir().display()))
                ]
                .spacing(4),
            )
            .padding([ui::PAD, ui::PAD_WIDE])
            .width(Length::Fill)
            .style(theme::banner(theme::DANGER_FILL, theme::DANGER_LINE)),
        );
        card = card.add(
            button(text("Try again").size(ui::BODY))
                .padding(ui::pill_tall(ui::PAD_WIDE))
                .style(theme::primary)
                .on_press(Message::RetryDownload),
        );
    } else {
        card = card.add(ui::small(format!(
            "Saved to {}",
            rufa_data::data_dir().display()
        )));
    }
    container(container(card).padding(ui::PAD_CARD).style(theme::card))
        .center(Length::Fill)
        .boxed()
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
    let tabs = ui::segmented_wide(
        [("Fit", CenterTab::Fit), ("Graph", CenterTab::Graph)]
            .into_iter()
            .map(|(label, which)| {
                (
                    label.to_string(),
                    app.center == which,
                    Message::CenterTab(which),
                )
            })
            .collect(),
    );
    let body = match app.center {
        CenterTab::Fit => fit::view(app),
        CenterTab::Graph => graph::view(app),
    };
    column![tabs, body]
        .spacing(ui::GAP_ROW)
        .height(Length::Fill)
        .boxed()
}

fn toolbar(app: &App) -> El<'_> {
    let has_fit = !app.docs.is_empty();
    let action = |label: &'static str, msg: Message| {
        let b = button(text(label).size(ui::BODY))
            .padding(ui::pill_tall(ui::PAD))
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
        .padding([0.0, ui::GAP_ROW]),
        text_input("Fit name", name)
            .on_input(Message::RenameFit)
            .width(260)
            .size(ui::BODY)
            .padding(ui::field(5.0, 8.0))
            .style(theme::input),
        action("Save", Message::Save),
        action("Undo", Message::Undo),
        action("Redo", Message::Redo),
        action("Copy EFT", Message::CopyEft),
        button(text("Paste EFT").size(ui::BODY))
            .padding(ui::pill_tall(ui::PAD))
            .style(theme::ghost)
            .on_press(Message::PasteEft),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);

    bar = bar.add(space().width(Length::Fill));
    bar = bar.add(ui::label("SKILLS"));
    let mut who: Vec<(String, bool, Message)> = [0u8, 5]
        .into_iter()
        .map(|level| {
            (
                format!("All {level}"),
                app.current == CharChoice::All(level),
                Message::SkillLevel(level),
            )
        })
        .collect();
    for (i, c) in app.characters.iter().enumerate() {
        who.push((
            c.name.clone(),
            app.current == CharChoice::Saved(i),
            Message::ChooseCharacter(CharChoice::Saved(i)),
        ));
    }
    bar = bar.add(ui::segmented(who));
    container(bar)
        .padding([0.0, ui::PAD])
        .height(ui::TOOLBAR)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center)
        .style(theme::toolbar)
        .boxed()
}

fn status_bar(app: &App) -> El<'_> {
    let mut bar = row![
        container(ui::small(&app.status))
            .clip(true)
            .width(Length::Fill)
    ]
    .spacing(ui::PAD)
    .align_y(iced::Alignment::Center);
    if app.prices_pending {
        bar = bar.add(ui::label("prices"));
    }
    let scale = app.settings.ui_scale;
    bar = bar.add(ui::label("ZOOM"));
    bar = bar.add(
        button(text("−").size(ui::SMALL))
            .padding(ui::pill_tight(ui::GAP_ROW))
            .style(theme::ghost)
            .on_press(Message::Zoom(crate::ZoomStep::Out)),
    );
    bar = bar.add(ui::small(format!("{:.0}%", scale * 100.0)));
    bar = bar.add(
        button(text("+").size(ui::SMALL))
            .padding(ui::pill_tight(ui::GAP_ROW))
            .style(theme::ghost)
            .on_press(Message::Zoom(crate::ZoomStep::In)),
    );
    bar = bar.add(
        button(text("Check for updates").size(ui::SMALL))
            .padding(ui::pill(ui::PAD))
            .style(theme::ghost)
            .on_press(Message::CheckUpdate),
    );
    if let Some(tag) = &app.sde_update {
        bar = bar.add(
            button(text(format!("Update game data to {tag}")).size(ui::SMALL))
                .padding(ui::pill(ui::PAD))
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
        .boxed()
}
