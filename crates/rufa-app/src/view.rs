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
    if app.downloading {
        return container(
            column![
                text("RUFA")
                    .font(theme::UI_SEMI)
                    .size(ui::HERO)
                    .color(theme::TEXT),
                ui::body(&app.status),
                ui::muted("Three files from github.com/EVEShipFit/sde-patched, about 25 MB."),
            ]
            .spacing(10)
            .align_x(iced::Alignment::Center),
        )
        .center(Length::Fill)
        .into();
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
        status_bar(app),
    ]
    .spacing(ui::GAP)
    .into()
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
