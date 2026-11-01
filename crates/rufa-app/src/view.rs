mod browser;
mod fit;
mod graph;
mod stats;

use iced::widget::{button, column, container, pane_grid, row, text, text_input};
use iced::{Element, Length};

use crate::{App, CenterTab, CharChoice, Message, Pane};

pub fn root(app: &App) -> Element<'_, Message> {
    if app.downloading {
        return container(
            column![
                text("Rufa").size(28),
                text(&app.status).size(14),
                text("Three files from github.com/EVEShipFit/sde-patched, about 25 MB.")
                    .size(12)
                    .style(text::secondary),
            ]
            .spacing(10)
            .align_x(iced::Alignment::Center),
        )
        .center(Length::Fill)
        .into();
    }
    let grid = pane_grid(&app.panes, |_, pane, _| {
        let body: Element<'_, Message> = match pane {
            Pane::Browser => browser::view(app),
            Pane::Fit => center(app),
            Pane::Stats => stats::view(app),
        };
        pane_grid::Content::new(
            container(body)
                .padding(6)
                .width(Length::Fill)
                .height(Length::Fill),
        )
    })
    .spacing(4)
    .on_resize(8, Message::PaneResized);

    column![
        toolbar(app),
        container(grid).width(Length::Fill).height(Length::Fill),
        status_bar(app),
    ]
    .into()
}

fn center(app: &App) -> Element<'_, Message> {
    let tab = |label: &'static str, which: CenterTab| {
        let style = if app.center == which {
            button::primary
        } else {
            button::secondary
        };
        button(text(label).size(12))
            .padding([3, 10])
            .style(style)
            .on_press(Message::CenterTab(which))
    };
    let tabs = row![tab("Fit", CenterTab::Fit), tab("Graph", CenterTab::Graph)].spacing(4);
    let body = match app.center {
        CenterTab::Fit => fit::view(app),
        CenterTab::Graph => graph::view(app),
    };
    column![tabs, body].spacing(6).height(Length::Fill).into()
}

fn toolbar(app: &App) -> Element<'_, Message> {
    let has_fit = !app.docs.is_empty();
    let action = |label: &'static str, msg: Message| {
        let b = button(text(label).size(12))
            .padding([3, 8])
            .style(button::secondary);
        if has_fit { b.on_press(msg) } else { b }
    };
    let name = app
        .docs
        .get(app.active)
        .and_then(|d| d.doc.fit.name.clone())
        .unwrap_or_default();
    let mut bar = row![
        text("Rufa").size(18),
        text_input("Fit name", &name)
            .on_input(Message::RenameFit)
            .width(260)
            .size(12)
            .padding(4),
        action("Save", Message::Save),
        action("Undo", Message::Undo),
        action("Redo", Message::Redo),
        action("Copy EFT", Message::CopyEft),
        button(text("Paste EFT").size(12))
            .padding([3, 8])
            .style(button::secondary)
            .on_press(Message::PasteEft),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    bar = bar.push(iced::widget::space::horizontal());
    bar = bar.push(text("Skills").size(12));
    for level in [0u8, 5] {
        let label = format!("All {level}");
        let style = if app.current == CharChoice::All(level) {
            button::primary
        } else {
            button::secondary
        };
        bar = bar.push(
            button(text(label).size(12))
                .padding([3, 8])
                .style(style)
                .on_press(Message::SkillLevel(level)),
        );
    }
    for (i, c) in app.characters.iter().enumerate() {
        let style = if app.current == CharChoice::Saved(i) {
            button::primary
        } else {
            button::secondary
        };
        bar = bar.push(
            button(text(&c.name).size(12))
                .padding([3, 8])
                .style(style)
                .on_press(Message::ChooseCharacter(CharChoice::Saved(i))),
        );
    }
    container(bar).padding([4, 8]).width(Length::Fill).into()
}

fn status_bar(app: &App) -> Element<'_, Message> {
    let mut bar = row![
        text(&app.status)
            .size(12)
            .style(text::secondary)
            .width(Length::Fill)
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);
    if let Some(tag) = &app.sde_update {
        bar = bar.push(
            button(text(format!("Update game data to {tag}")).size(11))
                .padding([2, 8])
                .style(button::success)
                .on_press(Message::UpdateSde),
        );
    }
    container(bar).padding([2, 8]).width(Length::Fill).into()
}
