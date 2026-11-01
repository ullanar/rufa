use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use rufa_data::catalog::MARKET_ROOTS;

use crate::{App, CharChoice, LeftTab, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let tab = |label: &'static str, which: LeftTab| {
        let style = if app.left == which {
            button::primary
        } else {
            button::secondary
        };
        button(text(label).size(12))
            .padding([3, 10])
            .style(style)
            .on_press(Message::LeftTab(which))
    };
    let tabs = row![
        tab("Ships", LeftTab::Ships),
        tab("Market", LeftTab::Market),
        tab("Skills", LeftTab::Character),
    ]
    .spacing(4);
    let body = match app.left {
        LeftTab::Ships => ships(app),
        LeftTab::Market => market(app),
        LeftTab::Character => character(app),
    };
    column![tabs, body].spacing(6).height(Length::Fill).into()
}

fn list_button<'a>(label: String, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12))
        .padding([2, 6])
        .width(Length::Fill)
        .style(button::text)
        .on_press(msg)
        .into()
}

fn ships(app: &App) -> Element<'_, Message> {
    let Ok(game) = &app.game else {
        return text("No game data loaded").into();
    };
    let search = text_input("Search hulls", &app.ship_search)
        .on_input(Message::ShipSearch)
        .size(12)
        .padding(4);
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
        list = list.push(list_button(
            "< All groups".to_string(),
            Message::ShipGroup(None),
        ));
        if let Some(group) = game.catalog.hull_groups().iter().find(|g| g.id == group_id) {
            for hull in &group.hulls {
                list = list.push(list_button(hull.name.clone(), Message::NewFit(hull.id)));
            }
        }
    } else {
        for group in game.catalog.hull_groups() {
            let label = format!("{} ({})", group.name, group.hulls.len());
            list = list.push(list_button(label, Message::ShipGroup(Some(group.id))));
        }
    }

    let mut saved = column![text("Saved fits").size(12).style(text::secondary)].spacing(1);
    if app.saved.is_empty() {
        saved = saved.push(
            text("none yet, Ctrl+S saves the open fit")
                .size(11)
                .style(text::secondary),
        );
    }
    for fit in &app.saved {
        let hull = fit.hull.map(|h| app.type_name(h)).unwrap_or("");
        let label = format!("{} [{}]", fit.name, hull);
        saved = saved.push(
            row![
                list_button(label, Message::OpenSaved(fit.path.clone())),
                button(text("x").size(11))
                    .padding([2, 5])
                    .style(button::text)
                    .on_press(Message::DeleteSaved(fit.path.clone())),
            ]
            .align_y(iced::Alignment::Center),
        );
    }

    column![
        search,
        scrollable(list).height(Length::FillPortion(3)),
        container(scrollable(saved)).height(Length::FillPortion(2)),
    ]
    .spacing(6)
    .into()
}

fn market(app: &App) -> Element<'_, Message> {
    let Ok(game) = &app.game else {
        return text("No game data loaded").into();
    };
    let search = text_input("Search items", &app.market_search)
        .on_input(Message::MarketSearch)
        .size(12)
        .padding(4);
    let mode = |label: &'static str, on: bool| {
        let style = if app.project_mode == on {
            button::primary
        } else {
            button::secondary
        };
        button(text(label).size(11))
            .padding([2, 6])
            .style(style)
            .on_press(Message::ProjectMode(on))
    };
    let modes = row![
        text("Click adds to").size(11).style(text::secondary),
        mode("fit", false),
        mode("projected", true),
    ]
    .spacing(4)
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
            list = list.push(text("No matches").size(12).style(text::secondary));
        }
    } else {
        match app.market_group.and_then(|id| game.catalog.market_node(id)) {
            None => {
                for root in MARKET_ROOTS {
                    if let Some(node) = game.catalog.market_node(root) {
                        list = list.push(list_button(
                            node.name.clone(),
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
                list = list.push(list_button(
                    format!("< {}", node.name),
                    Message::MarketGroup(up),
                ));
                for child in &node.children {
                    if let Some(c) = game.catalog.market_node(*child) {
                        list = list.push(list_button(
                            c.name.clone(),
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

    column![search, modes, scrollable(list).height(Length::Fill)]
        .spacing(6)
        .into()
}

fn character(app: &App) -> Element<'_, Message> {
    let Ok(game) = &app.game else {
        return text("No game data loaded").into();
    };
    let choice = |label: String, which: CharChoice| {
        let style = if app.current == which {
            button::primary
        } else {
            button::secondary
        };
        button(text(label).size(12))
            .padding([2, 8])
            .style(style)
            .on_press(Message::ChooseCharacter(which))
    };
    let mut picker = row![
        choice("All 0".to_string(), CharChoice::All(0)),
        choice("All 5".to_string(), CharChoice::All(5))
    ]
    .spacing(4);
    for (i, c) in app.characters.iter().enumerate() {
        picker = picker.push(choice(c.name.clone(), CharChoice::Saved(i)));
    }
    picker = picker.push(
        button(text("+ New").size(12))
            .padding([2, 8])
            .style(button::success)
            .on_press(Message::NewCharacter),
    );

    let picker = picker.wrap();
    let login_label = if app.esi_pending {
        "Waiting for the browser"
    } else {
        "Log in with EVE"
    };
    let mut login = button(text(login_label).size(11))
        .padding([2, 8])
        .style(button::primary);
    if !app.esi_pending {
        login = login.on_press(Message::EsiLogin);
    }
    let esi = row![
        text_input("ESI client id", &app.settings.esi_client_id)
            .on_input(Message::EsiClientId)
            .size(11)
            .padding(3),
        login,
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);
    let esi_hint = text("Register an application at developers.eveonline.com with the scope esi-skills.read_skills.v1 and a callback of http://localhost:PORT/callback; any port.")
        .size(10)
        .style(text::secondary);
    let CharChoice::Saved(i) = app.current else {
        return column![
            picker,
            text(
                "All N sets every skill to that level. Add a character to edit skills, or log in."
            )
            .size(11)
            .style(text::secondary),
            esi,
            esi_hint,
        ]
        .spacing(6)
        .into();
    };
    let Some(current) = app.characters.get(i) else {
        return picker.into();
    };

    let header = row![
        text_input("Character name", &current.name)
            .on_input(Message::RenameCharacter)
            .size(12)
            .padding(4),
        button(text("Delete").size(11))
            .padding([2, 6])
            .style(button::danger)
            .on_press(Message::DeleteCharacter),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);
    let bulk = row![
        text("Set all to").size(11).style(text::secondary),
        button(text("0").size(11))
            .padding([1, 6])
            .style(button::secondary)
            .on_press(Message::AllSkillsTo(0)),
        button(text("5").size(11))
            .padding([1, 6])
            .style(button::secondary)
            .on_press(Message::AllSkillsTo(5)),
        text_input("Search skills", &app.skill_search)
            .on_input(Message::SkillSearch)
            .size(12)
            .padding(4),
    ]
    .spacing(4)
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
        list =
            list.push(container(text(&group.name).size(12).style(text::secondary)).padding([4, 2]));
        for skill in skills {
            let level = current.skills.get(&skill.id).copied().unwrap_or(0);
            let mut levels = row![].spacing(1);
            for l in 0..=5u8 {
                let style = if l == level {
                    button::primary
                } else {
                    button::text
                };
                levels = levels.push(
                    button(text(l.to_string()).size(10))
                        .padding([0, 4])
                        .style(style)
                        .on_press(Message::SetSkill(skill.id, l)),
                );
            }
            list = list.push(
                row![text(&skill.name).size(11).width(Length::Fill), levels]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
            );
        }
    }

    column![
        picker,
        esi,
        header,
        bulk,
        scrollable(list).height(Length::Fill)
    ]
    .spacing(6)
    .into()
}
