use iced::Length;
use iced::widget::{button, column, container, row, text, text_input};
use rufa_data::catalog::MARKET_ROOTS;

use crate::theme;
use crate::ui::{self, El};
use crate::{App, CharChoice, LeftTab, Message};

pub fn view(app: &App) -> El<'_> {
    let tab = |label: &'static str, which: LeftTab| {
        button(text(label).size(ui::BODY).font(theme::UI_MEDIUM))
            .padding([3, 10])
            .style(theme::chip(app.left == which))
            .on_press(Message::LeftTab(which))
    };
    let tabs = row![
        tab("Ships", LeftTab::Ships),
        tab("Market", LeftTab::Market),
        tab("Skills", LeftTab::Character)
    ]
    .spacing(ui::GAP);
    let body = match app.left {
        LeftTab::Ships => ships(app),
        LeftTab::Market => market(app),
        LeftTab::Character => character(app),
    };
    column![tabs, body]
        .spacing(ui::GAP_ROW)
        .height(Length::Fill)
        .into()
}

fn search<'a>(placeholder: &'a str, value: &'a str, on_input: fn(String) -> Message) -> El<'a> {
    text_input(placeholder, value)
        .on_input(on_input)
        .size(ui::BODY)
        .padding([5, 8])
        .style(theme::input)
        .into()
}

fn list_button<'a>(label: String, msg: Message) -> El<'a> {
    button(text(label).size(ui::BODY).wrapping(text::Wrapping::None))
        .padding([3, 8])
        .width(Length::Fill)
        .style(theme::list_item(false))
        .on_press(msg)
        .into()
}

fn group_button<'a>(label: String, count: usize, msg: Message) -> El<'a> {
    button(
        row![
            text(label)
                .size(ui::BODY)
                .wrapping(text::Wrapping::None)
                .width(Length::Fill),
            text(count.to_string()).size(ui::MICRO).color(theme::MUTED),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([3, 8])
    .width(Length::Fill)
    .style(theme::list_item(false))
    .on_press(msg)
    .into()
}

fn back_button<'a>(label: String, msg: Message) -> El<'a> {
    button(
        text(format!("‹ {label}"))
            .size(ui::BODY)
            .color(theme::ACCENT),
    )
    .padding([3, 8])
    .width(Length::Fill)
    .style(theme::ghost)
    .on_press(msg)
    .into()
}

fn ships(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").into();
    };
    let query = app.ship_search.trim().to_lowercase();
    let mut list = column![].spacing(1);
    if !query.is_empty() {
        for group in game.catalog.hull_groups() {
            for hull in &group.hulls {
                if hull.name.to_lowercase().contains(&query) {
                    list = list.push(list_button(hull.name.clone(), Message::NewFit(hull.id)));
                }
            }
        }
    } else if let Some(group_id) = app.ship_group {
        if let Some(group) = game.catalog.hull_groups().iter().find(|g| g.id == group_id) {
            list = list.push(back_button(group.name.clone(), Message::ShipGroup(None)));
            for hull in &group.hulls {
                list = list.push(list_button(hull.name.clone(), Message::NewFit(hull.id)));
            }
        }
    } else {
        for group in game.catalog.hull_groups() {
            list = list.push(group_button(
                group.name.clone(),
                group.hulls.len(),
                Message::ShipGroup(Some(group.id)),
            ));
        }
    }

    let mut saved = column![
        row![
            ui::eyebrow("Saved fits").width(Length::Fill),
            button(text("Import from Pyfa").size(ui::SMALL))
                .padding([2, 8])
                .style(theme::secondary)
                .on_press(Message::ImportPyfa),
        ]
        .align_y(iced::Alignment::Center)
    ]
    .spacing(1);
    if app.saved.is_empty() {
        saved = saved.push(
            text("None yet. Ctrl+S saves the open fit.")
                .size(ui::SMALL)
                .color(theme::FAINT),
        );
    }
    for fit in &app.saved {
        let hull = fit.hull.map(|h| app.type_name(h)).unwrap_or("");
        saved = saved.push(
            row![
                button(
                    row![
                        text(&fit.name)
                            .size(ui::BODY)
                            .wrapping(text::Wrapping::None)
                            .width(Length::Fill),
                        text(hull).size(ui::MICRO).color(theme::MUTED),
                    ]
                    .spacing(ui::GAP)
                    .align_y(iced::Alignment::Center),
                )
                .padding([3, 8])
                .width(Length::Fill)
                .style(theme::list_item(false))
                .on_press(Message::OpenSaved(fit.path.clone())),
                button(text("×").size(ui::SMALL))
                    .padding([3, 5])
                    .style(theme::ghost)
                    .on_press(Message::DeleteSaved(fit.path.clone())),
            ]
            .align_y(iced::Alignment::Center),
        );
    }

    column![
        search("Search hulls", &app.ship_search, Message::ShipSearch),
        ui::scroll(list).height(Length::FillPortion(3)),
        ui::rule(),
        container(ui::scroll(saved)).height(Length::FillPortion(2)),
    ]
    .spacing(ui::GAP_ROW)
    .into()
}

fn market(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").into();
    };
    let mode = |label: &'static str, on: bool| {
        button(text(label).size(ui::MICRO))
            .padding([2, 7])
            .style(theme::chip(app.project_mode == on))
            .on_press(Message::ProjectMode(on))
    };
    let modes = row![
        ui::label("CLICK ADDS TO"),
        mode("fit", false),
        mode("projected", true)
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    let mut list = column![].spacing(1);

    if app.market_search.trim().len() >= 2 {
        for id in &app.market_results {
            list = list.push(list_button(
                game.type_name(*id).to_string(),
                Message::AddItem(*id),
            ));
        }
        if app.market_results.is_empty() {
            list = list.push(text("No matches").size(ui::SMALL).color(theme::FAINT));
        }
    } else {
        match app.market_group.and_then(|id| game.catalog.market_node(id)) {
            None => {
                for root in MARKET_ROOTS {
                    if let Some(node) = game.catalog.market_node(root) {
                        list = list.push(group_button(
                            node.name.clone(),
                            node.children.len(),
                            Message::MarketGroup(Some(root)),
                        ));
                    }
                }
            }
            Some(node) => {
                let parent = game.catalog.market_node(node.parent).map(|_| node.parent);
                let up = if MARKET_ROOTS.contains(&node.id) {
                    None
                } else {
                    parent
                };
                list = list.push(back_button(node.name.clone(), Message::MarketGroup(up)));
                for child in &node.children {
                    if let Some(c) = game.catalog.market_node(*child) {
                        list = list.push(group_button(
                            c.name.clone(),
                            c.children.len() + c.types.len(),
                            Message::MarketGroup(Some(*child)),
                        ));
                    }
                }
                let mut types: Vec<(&str, i32)> = node
                    .types
                    .iter()
                    .map(|t| (game.type_name(*t), *t))
                    .collect();
                types.sort();
                for (name, id) in types {
                    list = list.push(list_button(name.to_string(), Message::AddItem(id)));
                }
            }
        }
    }

    column![
        search("Search items", &app.market_search, Message::MarketSearch),
        modes,
        ui::scroll(container(list).padding(iced::Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 0.0
        }))
        .height(Length::Fill),
    ]
    .spacing(ui::GAP_ROW)
    .into()
}

fn character(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").into();
    };
    let choice = |label: String, which: CharChoice| {
        button(text(label).size(ui::SMALL))
            .padding([2, 8])
            .style(theme::chip(app.current == which))
            .on_press(Message::ChooseCharacter(which))
    };
    let mut picker = row![
        choice("All 0".to_string(), CharChoice::All(0)),
        choice("All 5".to_string(), CharChoice::All(5))
    ]
    .spacing(ui::GAP);
    for (i, c) in app.characters.iter().enumerate() {
        picker = picker.push(choice(c.name.clone(), CharChoice::Saved(i)));
    }
    picker = picker.push(
        button(text("+ New").size(ui::SMALL))
            .padding([2, 8])
            .style(theme::secondary)
            .on_press(Message::NewCharacter),
    );
    let picker = picker.wrap();

    let login_label = if app.esi_pending {
        "Waiting for the browser"
    } else {
        "Log in with EVE"
    };
    let mut login = button(text(login_label).size(ui::SMALL))
        .padding([4, 10])
        .style(theme::primary);
    if !app.esi_pending {
        login = login.on_press(Message::EsiLogin);
    }
    let esi = row![
        text_input("ESI client id", &app.settings.esi_client_id)
            .on_input(Message::EsiClientId)
            .size(ui::SMALL)
            .padding([4, 8])
            .style(theme::input),
        login,
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    let esi_hint = text("Register an application at developers.eveonline.com with the scope esi-skills.read_skills.v1 and the callback http://localhost:PORT/callback, any port.")
        .size(ui::MICRO)
        .color(theme::MUTED);

    let CharChoice::Saved(i) = app.current else {
        return column![
            picker,
            ui::muted(
                "All N sets every skill to that level. Add a character to edit skills, or log in."
            ),
            esi,
            esi_hint,
        ]
        .spacing(ui::GAP_ROW)
        .into();
    };
    let Some(current) = app.characters.get(i) else {
        return picker.into();
    };

    let header = row![
        text_input("Character name", &current.name)
            .on_input(Message::RenameCharacter)
            .size(ui::BODY)
            .padding([4, 8])
            .style(theme::input),
        button(text("Delete").size(ui::SMALL))
            .padding([3, 8])
            .style(theme::danger)
            .on_press(Message::DeleteCharacter),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    let bulk = row![
        ui::label("SET ALL TO"),
        button(text("0").size(ui::SMALL))
            .padding([1, 7])
            .style(theme::secondary)
            .on_press(Message::AllSkillsTo(0)),
        button(text("5").size(ui::SMALL))
            .padding([1, 7])
            .style(theme::secondary)
            .on_press(Message::AllSkillsTo(5)),
        text_input("Search skills", &app.skill_search)
            .on_input(Message::SkillSearch)
            .size(ui::SMALL)
            .padding([4, 8])
            .style(theme::input),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);

    let query = app.skill_search.trim().to_lowercase();
    let mut list = column![].spacing(1);
    for group in game.catalog.skill_groups() {
        let skills: Vec<_> = group
            .hulls
            .iter()
            .filter(|s| query.is_empty() || s.name.to_lowercase().contains(&query))
            .collect();
        if skills.is_empty() {
            continue;
        }
        list = list.push(container(ui::eyebrow(&group.name)).padding([6, 2]));
        for skill in skills {
            let level = current.skills.get(&skill.id).copied().unwrap_or(0);
            let mut levels = row![].spacing(1);
            for l in 0..=5u8 {
                levels = levels.push(
                    button(text(l.to_string()).size(ui::MICRO))
                        .padding([0, 5])
                        .style(theme::chip(l == level))
                        .on_press(Message::SetSkill(skill.id, l)),
                );
            }
            list = list.push(
                container(
                    row![
                        text(&skill.name)
                            .size(ui::SMALL)
                            .color(theme::TEXT)
                            .width(Length::Fill)
                            .wrapping(text::Wrapping::None),
                        levels
                    ]
                    .spacing(ui::GAP)
                    .align_y(iced::Alignment::Center),
                )
                .padding([0, 2])
                .height(ui::LIST_ROW),
            );
        }
    }

    column![
        picker,
        esi,
        header,
        bulk,
        ui::scroll(list).height(Length::Fill)
    ]
    .spacing(ui::GAP_ROW)
    .into()
}
