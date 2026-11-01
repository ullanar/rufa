use iced::widget::{
    button, column, container, image, mouse_area, pick_list, pin, row, scrollable, space, stack,
    text, tooltip,
};
use iced::{Color, Length};
use rufa_data::icons::Kind;
use rufa_sim::place::Rack;
use rufa_sim::projection::Projected;
use rufa_sim::{Slot, State};

use crate::fmt;
use crate::theme;
use crate::ui::{self, El};
use crate::{App, Doc, EnvChoice, Message};

pub fn view(app: &App) -> El<'_> {
    let mut tabs = row![].spacing(2).align_y(iced::Alignment::Center);
    for (i, doc) in app.docs.iter().enumerate() {
        let mark = if doc.dirty { " •" } else { "" };
        tabs = tabs.push(
            button(
                text(format!("{}{mark}", app.fit_title(doc)))
                    .size(ui::BODY)
                    .font(theme::UI_MEDIUM),
            )
            .padding([4, 10])
            .style(theme::tab(i == app.active))
            .on_press(Message::SelectTab(i)),
        );
        tabs = tabs.push(
            button(text("×").size(ui::SMALL))
                .padding([3, 5])
                .style(theme::ghost)
                .on_press(Message::CloseTab(i)),
        );
    }
    let body: El<'_> = match app.docs.get(app.active) {
        Some(doc) => {
            let list = mouse_area(
                scrollable(slots(app, doc))
                    .style(theme::scroll)
                    .height(Length::Fill),
            )
            .on_move(Message::CursorMoved);
            let mut layers: Vec<El<'_>> = vec![list.into()];
            if let Some((i, at)) = app.menu
                && i < doc.doc.fit.items.len()
            {
                layers.push(
                    mouse_area(space().width(Length::Fill).height(Length::Fill))
                        .on_press(Message::CloseMenu)
                        .on_right_press(Message::CloseMenu)
                        .into(),
                );
                layers.push(
                    pin(context_menu(app, doc, i))
                        .x(at.x.min(400.0))
                        .y(at.y)
                        .into(),
                );
            }
            column![
                stack(layers).width(Length::Fill).height(Length::Fill),
                violations(app, doc),
            ]
            .spacing(ui::GAP_ROW)
            .height(Length::Fill)
            .into()
        }
        None => container(
            column![
                text("No fit open")
                    .font(theme::UI_SEMI)
                    .size(ui::TITLE)
                    .color(theme::TEXT_SECONDARY),
                ui::muted("Pick a hull on the Ships tab, or paste an EFT or DNA fit with Ctrl+V."),
            ]
            .spacing(ui::GAP_ROW)
            .align_x(iced::Alignment::Center),
        )
        .center(Length::Fill)
        .into(),
    };
    let strip = container(
        scrollable(tabs).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        )),
    )
    .padding(2)
    .width(Length::Fill)
    .style(theme::rounded(theme::SUNKEN, 6.0));
    column![strip, body]
        .spacing(ui::GAP_ROW)
        .height(Length::Fill)
        .into()
}

fn hull_card<'a>(app: &'a App, doc: &'a Doc) -> El<'a> {
    let fit = &doc.doc.fit;
    let group = app
        .game
        .as_ref()
        .ok()
        .and_then(|g| {
            g.sde
                .get_type(fit.ship.type_id)
                .map(|t| g.catalog.group_name(t.group_id()).to_string())
        })
        .unwrap_or_default();
    let picture: El<'a> = match app.icon(Kind::Render, fit.ship.type_id) {
        Some(handle) => image(handle).width(64).height(64).into(),
        None => container(space().width(64).height(64))
            .style(theme::rounded(theme::SUNKEN, 6.0))
            .into(),
    };
    let r = &doc.doc.stats.resources;
    let mini = |name: &'static str, used: f64, total: f64| {
        let fraction = if total > 0.0 {
            (used / total) as f32
        } else {
            0.0
        };
        column![
            row![
                ui::label(name).width(Length::Fill),
                text(format!("{:.0}%", fraction * 100.0))
                    .size(ui::MICRO)
                    .font(theme::UI_MEDIUM)
                    .color(if fraction > 1.0 {
                        theme::DANGER_TEXT
                    } else {
                        theme::TEXT_SECONDARY
                    })
            ],
            ui::bar(fraction, ui::gauge_color(fraction), 3.0),
        ]
        .spacing(2)
        .width(90)
    };
    container(
        row![
            picture,
            column![ui::title(app.type_name(fit.ship.type_id)), ui::muted(group),]
                .spacing(2)
                .width(Length::Fill),
            row![
                mini("CPU", r.cpu_used, r.cpu_total),
                mini("PG", r.power_used, r.power_total),
                mini("CAL", r.calibration_used, r.calibration_total),
            ]
            .spacing(ui::PAD),
        ]
        .spacing(ui::PAD)
        .align_y(iced::Alignment::Center),
    )
    .padding(ui::PAD)
    .width(Length::Fill)
    .style(theme::card)
    .into()
}

fn rail_color(rack: Rack) -> Color {
    match rack {
        Rack::High => theme::RAIL_HIGH,
        Rack::Medium => theme::RAIL_MEDIUM,
        Rack::Low => theme::RAIL_LOW,
        Rack::Rig => theme::RAIL_RIG,
        Rack::Subsystem => theme::RAIL_SUBSYSTEM,
        Rack::Service => theme::RAIL_SERVICE,
    }
}

fn rack_header<'a>(name: String, used: usize, count: u8, over: bool) -> El<'a> {
    let counter_color = if over {
        theme::DANGER_TEXT
    } else {
        theme::MUTED
    };
    container(
        row![
            ui::eyebrow(&name).width(Length::Fill),
            text(format!("{used}/{count}"))
                .font(theme::LABEL)
                .size(ui::LABEL)
                .color(counter_color),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding {
        top: 8.0,
        right: 6.0,
        bottom: 2.0,
        left: 2.0,
    })
    .width(Length::Fill)
    .into()
}

fn empty_row<'a>(label: String, rail: Color) -> El<'a> {
    container(
        row![
            container(space().width(3).height(14)).style(theme::rounded(rail, 1.5)),
            text(label).size(ui::SMALL).color(theme::FAINT),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    )
    .height(ui::ROW)
    .padding([0, 4])
    .width(Length::Fill)
    .into()
}

type SlotFilter = fn(Slot) -> bool;

fn slots<'a>(app: &'a App, doc: &'a Doc) -> El<'a> {
    let fit = &doc.doc.fit;
    let layout = &doc.doc.layout;
    let mut col = column![].spacing(1).padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 0.0,
    });
    col = col.push(hull_card(app, doc));

    if let Ok(game) = &app.game
        && let Some(ty) = game.sde.get_type(fit.ship.type_id)
        && let Some(modes) = ty.mode_type_ids()
        && !modes.is_empty()
    {
        let mut line = row![ui::label("MODE")]
            .spacing(ui::GAP)
            .align_y(iced::Alignment::Center);
        for mode in modes {
            let name = app
                .type_name(mode)
                .rsplit(' ')
                .nth(1)
                .unwrap_or("")
                .to_string();
            line = line.push(
                button(text(name).size(ui::SMALL))
                    .padding([2, 8])
                    .style(theme::chip(fit.ship.mode == Some(mode)))
                    .on_press(Message::SetMode(Some(mode))),
            );
        }
        col = col.push(container(line).padding([4, 2]));
    }

    for rack in Rack::ALL {
        let count = layout.count(rack);
        let used = fit
            .items
            .iter()
            .filter(|it| Rack::of_slot(it.slot).is_some_and(|(r, _)| r == rack))
            .count();
        if count == 0 && used == 0 {
            continue;
        }
        col = col.push(rack_header(
            rack.label().to_string(),
            used,
            count,
            used > count as usize,
        ));
        for index in 0..count.max(used as u8) {
            let slot = rack.slot(index);
            match fit.items.iter().position(|it| it.slot == slot) {
                Some(i) => col = col.push(item_row(app, doc, i, rail_color(rack))),
                None => {
                    col = col.push(empty_row(
                        format!("{} slot", rack.label()),
                        rail_color(rack),
                    ))
                }
            }
        }
    }

    let sections: [(&str, SlotFilter, Color); 5] = [
        ("Drones", |s| s == Slot::DroneBay, theme::RAIL_SERVICE),
        (
            "Fighters",
            |s| matches!(s, Slot::FighterTube(_) | Slot::FighterBay),
            theme::RAIL_SERVICE,
        ),
        (
            "Implants",
            |s| matches!(s, Slot::Implant(_)),
            theme::RAIL_SUBSYSTEM,
        ),
        (
            "Boosters",
            |s| matches!(s, Slot::Booster(_)),
            theme::RAIL_SUBSYSTEM,
        ),
        ("Cargo", |s| s == Slot::Cargo, theme::RAIL_SERVICE),
    ];
    for (label, matches, rail) in sections {
        let indexes: Vec<usize> = fit
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| matches(it.slot))
            .map(|(i, _)| i)
            .collect();
        if indexes.is_empty() {
            continue;
        }
        col = col.push(container(ui::eyebrow(label)).padding(iced::Padding {
            top: 8.0,
            right: 6.0,
            bottom: 2.0,
            left: 2.0,
        }));
        for i in indexes {
            col = col.push(item_row(app, doc, i, rail));
        }
    }

    col = col.push(implant_sets(app));
    col = col.push(projected(app, doc));
    col.into()
}

fn violations<'a>(app: &'a App, doc: &'a Doc) -> El<'a> {
    let list = doc.doc.calc.violations.as_deref().unwrap_or(&[]);
    if list.is_empty() {
        return space().height(0).into();
    }
    let mut lines = column![].spacing(2);
    let head = if list.len() == 1 {
        "1 problem".to_string()
    } else {
        format!("{} problems", list.len())
    };
    lines = lines.push(
        text(head)
            .font(theme::UI_MEDIUM)
            .size(ui::BODY)
            .color(theme::DANGER_TEXT),
    );
    for v in list.iter().take(4) {
        lines = lines.push(
            text(app.describe_violation(v))
                .size(ui::SMALL)
                .color(theme::DANGER_TEXT),
        );
    }
    if list.len() > 4 {
        lines = lines.push(
            text(format!("and {} more", list.len() - 4))
                .size(ui::SMALL)
                .color(theme::DANGER_TEXT),
        );
    }
    container(lines)
        .padding([6, 10])
        .width(Length::Fill)
        .style(theme::banner(theme::DANGER_FILL, theme::DANGER_LINE))
        .into()
}

fn implant_sets(app: &App) -> El<'_> {
    let mut line = row![
        ui::label("IMPLANT SETS"),
        button(text("save current").size(ui::SMALL))
            .padding([2, 8])
            .style(theme::ghost)
            .on_press(Message::SaveImplantSet),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    for (i, set) in app.implant_sets.iter().enumerate() {
        line = line.push(
            row![
                button(text(&set.name).size(ui::SMALL))
                    .padding([2, 8])
                    .style(theme::chip(false))
                    .on_press(Message::ApplyImplantSet(i)),
                button(text("×").size(ui::SMALL))
                    .padding([2, 4])
                    .style(theme::ghost)
                    .on_press(Message::DeleteImplantSet(i)),
            ]
            .align_y(iced::Alignment::Center),
        );
    }
    container(line.wrap()).padding([8, 2]).into()
}

fn projected<'a>(app: &'a App, doc: &'a Doc) -> El<'a> {
    let mut col = column![container(ui::eyebrow("Projected")).padding(iced::Padding {
        top: 8.0,
        right: 6.0,
        bottom: 2.0,
        left: 2.0
    })]
    .spacing(2);
    if let Ok(game) = &app.game {
        let mut options = vec![EnvChoice {
            id: 0,
            name: "No environment".to_string(),
        }];
        options.extend(game.catalog.environments().iter().map(|e| EnvChoice {
            id: e.id,
            name: e.name.clone(),
        }));
        let selected = doc
            .doc
            .projections
            .iter()
            .find_map(|p| match p {
                Projected::Beacon(id) => options.iter().find(|o| o.id == *id).cloned(),
                _ => None,
            })
            .unwrap_or_else(|| options[0].clone());
        col = col.push(
            container(
                pick_list(options, Some(selected), Message::Environment)
                    .text_size(ui::SMALL)
                    .padding([4, 8])
                    .style(theme::picker),
            )
            .padding([1, 2]),
        );
    }
    let mut tabs = row![ui::label("PROJECT FIT")]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
    let mut any_tab = false;
    for (i, other) in app.docs.iter().enumerate() {
        if i == app.active {
            continue;
        }
        any_tab = true;
        tabs = tabs.push(
            button(text(app.fit_title(other)).size(ui::SMALL))
                .padding([2, 8])
                .style(theme::chip(false))
                .on_press(Message::ProjectFit(i)),
        );
    }
    if any_tab {
        col = col.push(container(tabs.wrap()).padding([2, 2]));
    }
    for (i, p) in doc.doc.projections.iter().enumerate() {
        if matches!(p, Projected::Beacon(_)) {
            continue;
        }
        col = col.push(
            container(
                row![
                    ui::body(app.projection_label(p)).width(Length::Fill),
                    button(text("×").size(ui::SMALL))
                        .padding([2, 5])
                        .style(theme::ghost)
                        .on_press(Message::RemoveProjection(i)),
                ]
                .align_y(iced::Alignment::Center),
            )
            .height(ui::ROW)
            .padding([0, 6]),
        );
    }
    col.into()
}

fn item_info(app: &App, doc: &Doc, i: usize) -> String {
    let (Some(ids), Some(result)) = (app.ids, doc.doc.calc.items.get(i)) else {
        return String::new();
    };
    let get = |id: i32| {
        result
            .attributes
            .get(&id)
            .map(|a| a.value)
            .filter(|v| *v != 0.0)
    };
    let mut parts = Vec::new();
    if let Some(range) = get(ids.max_range) {
        match get(ids.falloff) {
            Some(falloff) => parts.push(format!(
                "{} + {}",
                fmt::distance(range),
                fmt::distance(falloff)
            )),
            None => parts.push(fmt::distance(range)),
        }
    }
    if let Some(dps) = get(ids.dps) {
        parts.push(format!("{} dps", fmt::num(dps, 1)));
    }
    if let Some(cycle) = get(ids.cycle_time) {
        parts.push(format!("{}s", fmt::num(cycle / 1000.0, 1)));
    }
    if let Some(cap) = get(ids.capacitor_need) {
        parts.push(format!("{} GJ", fmt::num(cap, 1)));
    }
    parts.join("  ·  ")
}

fn item_row<'a>(app: &'a App, doc: &'a Doc, i: usize, rail: Color) -> El<'a> {
    let item = &doc.doc.fit.items[i];
    let result = doc.doc.calc.items.get(i);
    let state = result.map(|r| r.state).unwrap_or(item.state);
    let mut name = app.type_name(item.type_id).to_string();
    if item.quantity > 1 {
        name = format!("{name}  ×{}", item.quantity);
    }
    let charge = item
        .charge
        .as_ref()
        .map(|c| app.type_name(c.type_id).to_string())
        .unwrap_or_default();
    let selected = app.selected_item == Some(i);
    let rail_shown = if state == State::Overload {
        theme::OVERLOAD
    } else {
        rail
    };

    let glyph = tooltip(
        mouse_area(container(ui::state_glyph(state)).padding([0, 2]))
            .on_press(Message::CycleState(i))
            .on_right_press(Message::ToggleOverload(i))
            .on_middle_press(Message::ToggleOffline(i)),
        container(
            text(format!(
                "{}  ·  click: online / active, right: overheat, middle: offline",
                state_name(state)
            ))
            .size(ui::SMALL),
        )
        .padding([4, 8])
        .style(theme::tooltip),
        tooltip::Position::Right,
    )
    .delay(std::time::Duration::from_millis(400));

    let icon: El<'a> = match app.icon(Kind::Icon, item.type_id) {
        Some(handle) => image(handle).width(18).height(18).into(),
        None => space().width(18).height(18).into(),
    };
    let mut line = row![
        container(space().width(3).height(14)).style(theme::rounded(rail_shown, 1.5)),
        glyph,
        icon,
        text(ui::clip(&name, 40))
            .size(ui::BODY)
            .font(theme::UI_MEDIUM)
            .color(theme::TEXT)
            .width(Length::Fill)
            .wrapping(text::Wrapping::None),
        charge_cell(i, charge),
        text(item_info(app, doc, i))
            .size(ui::SMALL)
            .color(theme::MUTED)
            .width(190)
            .wrapping(text::Wrapping::None)
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(ui::GAP_ROW)
    .align_y(iced::Alignment::Center);

    if item.slot == Slot::DroneBay || item.slot == Slot::Cargo {
        line = line.push(
            button(text("−").size(ui::SMALL))
                .padding([1, 5])
                .style(theme::ghost)
                .on_press(Message::Quantity(i, item.quantity.saturating_sub(1))),
        );
        line = line.push(
            button(text("+").size(ui::SMALL))
                .padding([1, 5])
                .style(theme::ghost)
                .on_press(Message::Quantity(i, item.quantity + 1)),
        );
    }
    line = line.push(
        button(text("×").size(ui::SMALL))
            .padding([1, 5])
            .style(theme::ghost)
            .on_press(Message::RemoveItem(i)),
    );

    mouse_area(
        container(line)
            .padding([0, 4])
            .height(ui::ROW)
            .align_y(iced::Alignment::Center)
            .style(theme::row(
                selected || app.selected.contains(&i),
                app.hovered_item == Some(i),
            ))
            .width(Length::Fill),
    )
    .on_press(Message::SelectItem(Some(i)))
    .on_right_press(Message::OpenMenu(i))
    .on_enter(Message::HoverItem(Some(i)))
    .on_exit(Message::HoverItem(None))
    .into()
}

fn context_menu<'a>(app: &'a App, doc: &'a Doc, i: usize) -> El<'a> {
    let item = &doc.doc.fit.items[i];
    let name = app.type_name(item.type_id).to_string();
    let count = app.selected.len().max(1);
    let title = if count > 1 {
        format!("{count} items selected")
    } else {
        name.clone()
    };
    let mut col = column![
        text(title)
            .size(ui::BODY)
            .font(theme::UI_MEDIUM)
            .color(theme::TEXT)
    ]
    .spacing(ui::GAP);
    let chip = |label: String, on: bool, msg: Message| {
        button(text(label).size(ui::SMALL))
            .padding([2, 7])
            .style(theme::chip(on))
            .on_press(msg)
    };
    let entry = |label: String, msg: Message| {
        button(text(label).size(ui::BODY).width(Length::Fill))
            .padding([3, 8])
            .width(Length::Fill)
            .style(theme::list_item(false))
            .on_press(msg)
    };
    let is_module = Rack::of_slot(item.slot).is_some();
    if let (Ok(game), Some(ids)) = (&app.game, app.ids)
        && is_module
    {
        let info = game.info();
        let mut charges = row![].spacing(3);
        let mut any = false;
        for group in rufa_sim::place::charge_groups(&info, &ids, item.type_id) {
            for charge in game.catalog.charges_in_group(group) {
                if !rufa_sim::place::accepts_charge(&info, &ids, item.type_id, charge.id) {
                    continue;
                }
                any = true;
                let loaded = item.charge.as_ref().is_some_and(|c| c.type_id == charge.id);
                charges = charges.push(chip(
                    charge.name.clone(),
                    loaded,
                    Message::MenuCharge(Some(charge.id)),
                ));
            }
        }
        if any {
            charges = charges.push(chip(
                "none".to_string(),
                item.charge.is_none(),
                Message::MenuCharge(None),
            ));
            col = col.push(ui::label("CHARGE"));
            col = col.push(charges.wrap());
        }
    }
    col = col.push(ui::rule());
    col = col.push(entry(
        format!("{} Stats", ui::clip(&name, 28)),
        Message::CloseMenu,
    ));
    col = col.push(entry("Market Group".to_string(), Message::MenuMarketJump));
    col = col.push(entry(
        if count > 1 {
            format!("Remove {count} items")
        } else {
            "Remove".to_string()
        },
        Message::MenuRemove,
    ));
    col = col.push(entry(
        "Remove All Similar".to_string(),
        Message::MenuRemoveSimilar,
    ));
    if let Ok(game) = &app.game
        && is_module
    {
        if let Some(ty) = game.sde.get_type(item.type_id) {
            let variants = game.catalog.variations(ty.market_group_id(), ty.group_id());
            if variants.len() > 1 {
                col = col.push(ui::rule());
                col = col.push(ui::label("VARIATIONS"));
                let mut line = row![].spacing(3);
                for v in variants {
                    line = line.push(chip(
                        v.name.clone(),
                        v.id == item.type_id,
                        Message::MenuVariant(v.id),
                    ));
                }
                col = col.push(line.wrap());
            }
        }
        let base = item
            .mutation
            .as_ref()
            .map(|m| m.base)
            .unwrap_or(item.type_id);
        let plasmids = rufa_sim::edit::mutaplasmids_for(&game.sde, base);
        if !plasmids.is_empty() {
            col = col.push(ui::label("APPLY MUTAPLASMID"));
            let mut line = row![].spacing(3);
            for p in &plasmids {
                let short = app
                    .type_name(*p)
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_string();
                line = line.push(chip(short, false, Message::MenuMutate(*p)));
            }
            if item.mutation.is_some() {
                line = line.push(chip(
                    "Revert".to_string(),
                    false,
                    Message::RevertMutation(i),
                ));
            }
            col = col.push(line.wrap());
        }
        col = col.push(ui::rule());
        col = col.push(entry(
            format!("Fill With {}", ui::clip(&name, 24)),
            Message::MenuFill,
        ));
    }
    let max = doc
        .doc
        .calc
        .items
        .get(i)
        .map(|r| r.max_state)
        .unwrap_or(State::Online);
    let mut states = row![].spacing(3);
    for s in [
        State::Offline,
        State::Online,
        State::Active,
        State::Overload,
    ] {
        if s > max {
            continue;
        }
        states = states.push(chip(
            state_name(s).to_string(),
            item.state == s,
            Message::MenuState(s),
        ));
    }
    col = col.push(ui::rule());
    col = col.push(ui::label("STATE"));
    col = col.push(states);
    container(col)
        .padding(ui::PAD)
        .width(360)
        .style(theme::tooltip)
        .into()
}

fn state_name(state: State) -> &'static str {
    match state {
        State::Offline => "Offline",
        State::Online => "Online",
        State::Active => "Active",
        State::Overload => "Overloaded",
    }
}

fn charge_cell<'a>(i: usize, charge: String) -> El<'a> {
    if charge.is_empty() {
        return space().width(150).into();
    }
    mouse_area(
        text(ui::clip(&charge, 26))
            .size(ui::SMALL)
            .color(theme::TEXT_SECONDARY)
            .width(150)
            .wrapping(text::Wrapping::None),
    )
    .on_right_press(Message::LoadCharge(i, None))
    .into()
}
