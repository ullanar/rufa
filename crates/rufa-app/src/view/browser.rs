use iced::Length;
use iced::widget::Widget;
use iced::widget::{
    button, column, container, image, mouse_area, row, space, text, text_input, tooltip,
};
use rufa_data::catalog::MARKET_ROOTS;

use crate::fmt;
use crate::theme;
use crate::ui::Add;
use crate::ui::{self, El};
use rufa_data::icons::Kind;

use crate::{AddTo, App, CharChoice, LeftTab, Message};

pub fn view(app: &App) -> El<'_> {
    let tabs = ui::segmented_wide(
        [
            ("Ships", LeftTab::Ships),
            ("Fits", LeftTab::Fits),
            ("Market", LeftTab::Market),
            ("Skills", LeftTab::Character),
        ]
        .into_iter()
        .map(|(label, which)| {
            (
                label.to_string(),
                app.left == which,
                Message::LeftTab(which),
            )
        })
        .collect(),
    );
    let body = match app.left {
        LeftTab::Ships => ships(app),
        LeftTab::Fits => fits(app),
        LeftTab::Market => market(app),
        LeftTab::Character => character(app),
    };
    column![tabs, body]
        .spacing(ui::GAP_ROW)
        .height(Length::Fill)
        .boxed()
}

fn search<'a>(placeholder: &'a str, value: &'a str, on_input: fn(String) -> Message) -> El<'a> {
    text_input(placeholder, value)
        .on_input(on_input)
        .size(ui::BODY)
        .padding(ui::field(5.0, 8.0))
        .style(theme::input)
        .boxed()
}

/// The type's own picture, or the space it would take, so names stay aligned.
fn icon_cell<'a>(app: &App, type_id: i32, size: f32) -> El<'a> {
    let inner: El<'a> = match app.icon(Kind::Icon, type_id) {
        Some(handle) => image(handle)
            .width(size)
            .height(size)
            .border_radius(3.0)
            .boxed(),
        None => space().width(size).height(size).boxed(),
    };
    container(inner)
        .width(size)
        .height(size)
        .style(theme::rounded(theme::SUNKEN, 3.0))
        .boxed()
}

fn list_button<'a>(app: &App, type_id: i32, label: String, msg: Message) -> El<'a> {
    button(
        row![
            icon_cell(app, type_id, 18.0),
            container(text(label).size(ui::BODY).wrapping(text::Wrapping::None))
                .clip(true)
                .width(Length::Fill),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    )
    .padding(ui::pill(ui::PAD))
    .width(Length::Fill)
    .style(theme::list_item(false))
    .on_press(msg)
    .boxed()
}

fn group_button<'a>(label: String, count: usize, msg: Message) -> El<'a> {
    button(
        row![
            container(
                text(label)
                    .size(ui::BODY)
                    .wrapping(text::Wrapping::None)
                    .width(Length::Shrink),
            )
            .clip(true)
            .width(Length::Fill),
            text(count.to_string())
                .size(ui::MICRO)
                .color(theme::MUTED)
                .wrapping(text::Wrapping::None)
                .width(22)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    )
    .padding(ui::pill(ui::PAD))
    .width(Length::Fill)
    .style(theme::list_item(false))
    .on_press(msg)
    .boxed()
}

fn back_button<'a>(label: String, msg: Message) -> El<'a> {
    button(
        text(format!("‹ {label}"))
            .size(ui::BODY)
            .color(theme::ACCENT),
    )
    .padding(ui::pill(ui::PAD))
    .width(Length::Fill)
    .style(theme::ghost)
    .on_press(msg)
    .boxed()
}

fn ships(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").boxed();
    };
    let query = app.ship_search.trim().to_lowercase();
    let mut list = ui::col().spacing(1);
    if !query.is_empty() {
        let mut hits = 0;
        for group in game.catalog.hull_groups() {
            let group_matches = group.name.to_lowercase().contains(&query);
            for hull in &group.hulls {
                if !group_matches && !hull.name.to_lowercase().contains(&query) {
                    continue;
                }
                hits += 1;
                list = list.add(hull_button(app, &hull.name, &group.name, hull.id));
            }
        }
        if hits == 0 {
            list = list.add(text("No matches").size(ui::SMALL).color(theme::FAINT));
        }
    } else if let Some(group_id) = app.ship_group {
        if let Some(group) = game.catalog.hull_groups().iter().find(|g| g.id == group_id) {
            list = list.add(back_button(group.name.clone(), Message::ShipGroup(None)));
            for hull in &group.hulls {
                list = list.add(list_button(
                    app,
                    hull.id,
                    hull.name.clone(),
                    Message::NewFit(hull.id),
                ));
            }
        }
    } else {
        for group in game.catalog.hull_groups() {
            list = list.add(group_button(
                group.name.clone(),
                group.hulls.len(),
                Message::ShipGroup(Some(group.id)),
            ));
        }
    }

    column![
        search(
            "Search hulls and groups",
            &app.ship_search,
            Message::ShipSearch
        ),
        ui::scroll(list).height(Length::Fill),
    ]
    .spacing(ui::GAP_ROW)
    .boxed()
}

fn hull_button<'a>(app: &App, name: &str, group: &str, id: i32) -> El<'a> {
    button(
        row![
            icon_cell(app, id, 18.0),
            container(
                text(name.to_string())
                    .size(ui::BODY)
                    .wrapping(text::Wrapping::None)
            )
            .clip(true)
            .width(Length::Fill),
            text(group.to_string())
                .size(ui::MICRO)
                .color(theme::MUTED)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center),
    )
    .padding(ui::pill(ui::PAD))
    .width(Length::Fill)
    .style(theme::list_item(false))
    .on_press(Message::NewFit(id))
    .boxed()
}

fn fits(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").boxed();
    };
    let query = app.fit_search.trim().to_lowercase();
    let open = |fit: &rufa_store::SavedFit| {
        app.docs
            .iter()
            .any(|d| d.path.as_deref() == Some(fit.path.as_path()))
    };

    let mut list = ui::col().spacing(1);
    if app.saved.is_empty() {
        list = list.add(
            text("None yet. Ctrl+S saves the open fit.")
                .size(ui::SMALL)
                .color(theme::FAINT),
        );
    } else if query.is_empty() {
        for (group_id, group_name, hulls) in fit_tree(app, game) {
            let folded = app.fits_folded.contains(&group_id);
            let count = hulls.iter().map(|(_, f)| f.len()).sum::<usize>();
            list = list.add(
                container(fold_button(
                    &group_name,
                    count,
                    folded,
                    Message::ToggleFitGroup(group_id),
                ))
                .style(theme::group_band),
            );
            if folded {
                continue;
            }
            for (hull, fits) in hulls {
                let count = fits.len();
                let mut branch = column![hull_header(app, hull, count)].spacing(1);
                for fit in fits {
                    branch = branch.add(fit_row(app, fit, open(fit), None, 22.0));
                }
                list = list.add(
                    row![
                        container(space().width(1).height(Length::Fill))
                            .style(theme::fill(theme::EDGE)),
                        branch,
                    ]
                    .spacing(ui::GAP_ROW)
                    .padding(iced::Padding {
                        top: 1.0,
                        right: 0.0,
                        bottom: 3.0,
                        left: 16.0,
                    }),
                );
            }
        }
    } else {
        let mut hits: Vec<&rufa_store::SavedFit> = app
            .saved
            .iter()
            .filter(|f| {
                f.name.to_lowercase().contains(&query)
                    || f.hull
                        .is_some_and(|h| app.type_name(h).to_lowercase().contains(&query))
            })
            .collect();
        hits.sort_by_key(|f| f.name.to_lowercase());
        if hits.is_empty() {
            list = list.add(text("No matches").size(ui::SMALL).color(theme::FAINT));
        }
        for fit in hits {
            let hull = fit.hull.map(|h| app.type_name(h)).unwrap_or("");
            list = list.add(fit_row(app, fit, open(fit), Some(hull), 4.0));
        }
    }

    // A running import leaves the button unpressable, so a second press
    // cannot start a second one over the same fits.
    let import = if app.pyfa_pending {
        button(text("Importing").size(ui::SMALL))
            .padding(ui::pill(ui::PAD))
            .style(theme::secondary)
    } else {
        button(text("Import from Pyfa").size(ui::SMALL))
            .padding(ui::pill(ui::PAD))
            .style(theme::secondary)
            .on_press(Message::ImportPyfa)
    };
    let head = row![
        ui::eyebrow("Saved fits").width(Length::Fill),
        ui::small(fmt::num(app.saved.len() as f64, 0)),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    let mut actions = row![import].spacing(ui::GAP);
    if app.duplicates.is_empty() && app.saved.len() > 1 {
        let find: El<'_> = if app.duplicates_pending {
            button(text("Looking").size(ui::SMALL))
                .padding(ui::pill(ui::PAD))
                .style(theme::ghost)
                .boxed()
        } else {
            button(text("Find duplicates").size(ui::SMALL))
                .padding(ui::pill(ui::PAD))
                .style(theme::ghost)
                .on_press(Message::FindDuplicates)
                .boxed()
        };
        actions = actions.add(find);
    }

    column![
        search("Search saved fits", &app.fit_search, Message::FitSearch),
        head,
        actions.wrap(),
        duplicate_panel(app),
        ui::scroll(list).height(Length::Fill),
    ]
    .spacing(ui::GAP_ROW)
    .boxed()
}

/// The same shape as a stats section: a name, the number that matters, and
/// one fact per row. Seven thousand file names say less than three counts.
fn duplicate_panel(app: &App) -> El<'_> {
    if app.duplicates.is_empty() {
        return space().height(0).boxed();
    }
    let going: usize = app.duplicates.iter().map(|d| d.remove.len()).sum();
    let fits = app.duplicates.len();
    let count = |n: usize| fmt::num(n as f64, 0);
    let head = row![
        ui::eyebrow("duplicates").width(Length::Fill),
        ui::value_colored(count(going), theme::DANGER_TEXT),
    ]
    .spacing(ui::GAP_ROW)
    .align_y(iced::Alignment::Center);
    container(
        column![
            head,
            ui::rule(),
            ui::stat_row(f32::INFINITY, "Fits with copies", count(fits)),
            ui::stat_row(f32::INFINITY, "Each fit keeps", "1".to_string()),
            row![
                button(text(format!("Delete {} files", count(going))).size(ui::SMALL))
                    .padding(ui::pill(ui::PAD))
                    .style(theme::danger)
                    .on_press(Message::RemoveDuplicates),
                button(text("Cancel").size(ui::SMALL))
                    .padding(ui::pill(ui::PAD))
                    .style(theme::ghost)
                    .on_press(Message::ForgetDuplicates),
            ]
            .spacing(ui::GAP)
            .wrap(),
        ]
        .spacing(ui::GAP),
    )
    .padding([ui::GAP_ROW, ui::PAD])
    .width(Length::Fill)
    .style(theme::card)
    .boxed()
}

type FitTree<'a> = Vec<(i32, String, Vec<(i32, Vec<&'a rufa_store::SavedFit>)>)>;

fn fit_tree<'a>(app: &'a App, game: &'static rufa_data::Game) -> FitTree<'a> {
    let mut groups: FitTree<'a> = Vec::new();
    for fit in &app.saved {
        let hull = fit.hull.unwrap_or(0);
        let group_id = game.sde.get_type(hull).map(|t| t.group_id()).unwrap_or(0);
        let group_name = match group_id {
            0 => "Unknown hull".to_string(),
            id => game.catalog.group_name(id).to_string(),
        };
        let group = match groups.iter().position(|(id, _, _)| *id == group_id) {
            Some(i) => &mut groups[i],
            None => {
                groups.push((group_id, group_name, Vec::new()));
                groups.last_mut().unwrap()
            }
        };
        match group.2.iter().position(|(id, _)| *id == hull) {
            Some(i) => group.2[i].1.push(fit),
            None => group.2.push((hull, vec![fit])),
        }
    }
    for group in &mut groups {
        group.2.sort_by_key(|(hull, _)| app.type_name(*hull));
        for (_, fits) in &mut group.2 {
            fits.sort_by_key(|f| f.name.to_lowercase());
        }
    }
    groups.sort_by(|a, b| a.1.cmp(&b.1));
    groups
}

/// The hull a group of saved fits belongs to: its picture, its name, and how
/// many fits hang under it.
fn hull_header<'a>(app: &App, hull: i32, count: usize) -> El<'a> {
    row![
        icon_cell(app, hull, 16.0),
        container(
            text(app.type_name(hull).to_string())
                .size(ui::SMALL)
                .font(theme::UI_MEDIUM)
                .color(theme::MUTED)
                .wrapping(text::Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill),
        text(count.to_string()).size(ui::MICRO).color(theme::FAINT),
    ]
    .spacing(ui::GAP_ROW)
    .padding(iced::Padding {
        top: 5.0,
        right: 8.0,
        bottom: 1.0,
        left: 4.0,
    })
    .align_y(iced::Alignment::Center)
    .boxed()
}

fn fold_button<'a>(label: &str, count: usize, folded: bool, msg: Message) -> El<'a> {
    button(
        row![
            text(if folded { "▸" } else { "▾" })
                .size(ui::MICRO)
                .color(theme::MUTED),
            container(
                text(label.to_string())
                    .size(ui::BODY)
                    .font(theme::UI_MEDIUM)
                    .wrapping(text::Wrapping::None)
            )
            .clip(true)
            .width(Length::Fill),
            text(count.to_string()).size(ui::MICRO).color(theme::MUTED),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    )
    .padding(ui::pill(ui::PAD))
    .width(Length::Fill)
    .style(theme::list_item(false))
    .on_press(msg)
    .boxed()
}

fn fit_row<'a>(
    app: &App,
    fit: &'a rufa_store::SavedFit,
    open: bool,
    hull: Option<&'a str>,
    indent: f32,
) -> El<'a> {
    let color = if open { theme::ACCENT } else { theme::TEXT };
    let mut line = row![
        container(
            text(&fit.name)
                .size(ui::BODY)
                .color(color)
                .wrapping(text::Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill)
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    if let Some(hull) = hull {
        line = line.add(text(hull).size(ui::MICRO).color(theme::MUTED));
    }
    // Copying and deleting are not offered at rest, only under the cursor.
    // The whole row carries the hover, so moving onto a button keeps it.
    let hovered = app.hovered_saved.as_deref() == Some(fit.path.as_path());
    let action = |glyph: &'static str, hint: &'static str, msg: Message| {
        tooltip(
            button(text(glyph).size(ui::SMALL))
                .padding(ui::pill(ui::GAP_ROW))
                .style(theme::ghost)
                .on_press(msg),
            container(text(hint).size(ui::SMALL))
                .padding([ui::GAP, ui::PAD])
                .style(theme::tooltip),
            tooltip::Position::Left,
        )
        .delay(std::time::Duration::from_millis(400))
    };
    let actions: El<'a> = if hovered {
        row![
            action(
                "\u{29c9}",
                "Duplicate",
                Message::DuplicateSaved(fit.path.clone())
            ),
            action("\u{00d7}", "Delete", Message::DeleteSaved(fit.path.clone())),
        ]
        .spacing(ui::GAP_ROW)
        .boxed()
    } else {
        space().width(58).boxed()
    };
    mouse_area(
        row![
            button(line)
                .padding(iced::Padding {
                    top: 3.0,
                    right: 8.0,
                    bottom: 3.0,
                    left: indent,
                })
                .width(Length::Fill)
                .style(theme::list_item(open))
                .on_press(Message::OpenSaved(fit.path.clone())),
            actions,
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    )
    .on_enter(Message::HoverSaved(Some(fit.path.clone())))
    .on_exit(Message::HoverSaved(None))
    .boxed()
}

fn market(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").boxed();
    };
    let mut modes = row![
        ui::label("ADD TO"),
        ui::segmented(
            [
                ("fit", AddTo::Fit),
                ("cargo", AddTo::Cargo),
                ("projected", AddTo::Projected),
            ]
            .into_iter()
            .map(|(label, target)| (
                label.to_string(),
                app.add_to == target,
                Message::AddTo(target),
            ))
            .collect(),
        ),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    if let Some(rack) = app.market_rack {
        modes = modes.add(
            button(text(format!("{} only  ×", rack.label())).size(ui::MICRO))
                .padding(ui::pill_tight(7.0))
                .style(theme::chip(true))
                .on_press(Message::PickForRack(None)),
        );
    }
    let modes = modes.wrap();
    let fits_rack = |type_id: i32| -> bool {
        let Some(rack) = app.market_rack else {
            return true;
        };
        let Some(ids) = app.ids else { return true };
        rufa_sim::place::placement(&game.info(), &ids, type_id)
            == Some(rufa_sim::place::Placement::Module(rack))
    };
    let mut list = ui::col().spacing(1);

    if app.market_search.trim().len() >= 2 || app.market_rack.is_some() {
        let mut any = false;
        for id in &app.market_results {
            if !fits_rack(*id) {
                continue;
            }
            any = true;
            list = list.add(list_button(
                app,
                *id,
                game.type_name(*id).to_string(),
                Message::AddItem(*id),
            ));
        }
        if !any {
            let hint = if app.market_search.trim().len() < 2 {
                "Type two letters to search this rack"
            } else {
                "No matches"
            };
            list = list.add(text(hint).size(ui::SMALL).color(theme::FAINT));
        }
    } else {
        match app.market_group.and_then(|id| game.catalog.market_node(id)) {
            None => {
                for root in MARKET_ROOTS {
                    if let Some(node) = game.catalog.market_node(root) {
                        list = list.add(group_button(
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
                list = list.add(back_button(node.name.clone(), Message::MarketGroup(up)));
                for child in &node.children {
                    if let Some(c) = game.catalog.market_node(*child) {
                        list = list.add(group_button(
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
                    list = list.add(list_button(app, id, name.to_string(), Message::AddItem(id)));
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
    .boxed()
}

fn character(app: &App) -> El<'_> {
    let Ok(game) = &app.game else {
        return ui::muted("No game data loaded").boxed();
    };
    let mut options = vec![
        ("All 0".to_string(), CharChoice::All(0)),
        ("All 5".to_string(), CharChoice::All(5)),
    ];
    for (i, c) in app.characters.iter().enumerate() {
        options.push((c.name.clone(), CharChoice::Saved(i)));
    }
    let picker = row![
        ui::segmented(
            options
                .into_iter()
                .map(|(label, which)| (
                    label,
                    app.current == which,
                    Message::ChooseCharacter(which),
                ))
                .collect(),
        ),
        button(text("+ New").size(ui::SMALL))
            .padding(ui::pill(8.0))
            .style(theme::secondary)
            .on_press(Message::NewCharacter),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center)
    .wrap();

    let login_label = if app.esi_pending {
        "Waiting for the browser"
    } else {
        "Log in with EVE"
    };
    let mut login = button(text(login_label).size(ui::SMALL))
        .padding(ui::pill(ui::PAD_WIDE))
        .style(theme::primary);
    if !app.esi_pending {
        login = login.on_press(Message::EsiLogin);
    }
    let esi = row![
        text_input("ESI client id", &app.settings.esi_client_id)
            .on_input(Message::EsiClientId)
            .size(ui::SMALL)
            .padding(ui::field(4.0, 8.0))
            .style(theme::input),
        login,
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    let esi_hint = column![
        text("An import needs an application of your own.")
            .size(ui::MICRO)
            .color(theme::MUTED),
        text("Register it at developers.eveonline.com.")
            .size(ui::MICRO)
            .color(theme::MUTED),
        text("Scope: esi-skills.read_skills.v1")
            .size(ui::MICRO)
            .color(theme::TEXT_SECONDARY),
        text("Callback: http://localhost:PORT/callback, any port.")
            .size(ui::MICRO)
            .color(theme::TEXT_SECONDARY),
    ]
    .spacing(1);

    let CharChoice::Saved(i) = app.current else {
        return column![
            picker,
            ui::muted(
                "All 0 and All 5 set every skill to that level. Add a character to edit \
                 single skills, or import one from EVE."
            ),
            esi,
            esi_hint,
        ]
        .spacing(ui::GAP_ROW)
        .boxed();
    };
    let Some(current) = app.characters.get(i) else {
        return picker.boxed();
    };

    let header = row![
        text_input("Character name", &current.name)
            .on_input(Message::RenameCharacter)
            .size(ui::BODY)
            .padding(ui::field(4.0, 8.0))
            .style(theme::input),
        button(text("Delete").size(ui::SMALL))
            .padding(ui::pill(ui::PAD))
            .style(theme::danger)
            .on_press(Message::DeleteCharacter),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    let bulk = row![
        ui::label("SET ALL TO"),
        button(text("0").size(ui::SMALL))
            .padding(ui::pill_tight(ui::GAP_ROW))
            .style(theme::secondary)
            .on_press(Message::AllSkillsTo(0)),
        button(text("5").size(ui::SMALL))
            .padding(ui::pill_tight(ui::GAP_ROW))
            .style(theme::secondary)
            .on_press(Message::AllSkillsTo(5)),
        text_input("Search skills", &app.skill_search)
            .on_input(Message::SkillSearch)
            .size(ui::SMALL)
            .padding(ui::field(4.0, 8.0))
            .style(theme::input),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);

    let query = app.skill_search.trim().to_lowercase();
    let mut list = ui::col().spacing(1);
    for group in game.catalog.skill_groups() {
        let skills: Vec<_> = group
            .hulls
            .iter()
            .filter(|s| query.is_empty() || s.name.to_lowercase().contains(&query))
            .collect();
        if skills.is_empty() {
            continue;
        }
        list = list.add(container(ui::eyebrow(&group.name)).padding([ui::GAP_ROW, ui::TIGHT]));
        for skill in skills {
            let level = current.skills.get(&skill.id).copied().unwrap_or(0);
            let mut levels = ui::line().spacing(1);
            for l in 0..=5u8 {
                levels = levels.add(
                    button(text(l.to_string()).size(ui::MICRO))
                        .padding(ui::pill_tight(5.0))
                        .style(theme::chip(l == level))
                        .on_press(Message::SetSkill(skill.id, l)),
                );
            }
            list = list.add(
                container(
                    row![
                        container(
                            text(&skill.name)
                                .size(ui::SMALL)
                                .color(theme::TEXT)
                                .wrapping(text::Wrapping::None)
                        )
                        .clip(true)
                        .width(Length::Fill),
                        levels
                    ]
                    .spacing(ui::GAP)
                    .align_y(iced::Alignment::Center),
                )
                .padding([0.0, ui::TIGHT])
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
    .boxed()
}
