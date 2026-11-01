use iced::widget::{button, column, container, mouse_area, pick_list, row, scrollable, text};
use rufa_sim::projection::Projected;

use crate::EnvChoice;
use iced::{Color, Element, Length};
use rufa_sim::place::Rack;
use rufa_sim::{Slot, State};

use crate::fmt;
use crate::{App, Doc, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let mut tabs = row![].spacing(2);
    for (i, doc) in app.docs.iter().enumerate() {
        let style = if i == app.active {
            button::primary
        } else {
            button::secondary
        };
        let mark = if doc.dirty { "*" } else { "" };
        tabs = tabs.push(
            button(text(format!("{}{mark}", app.fit_title(doc))).size(12))
                .padding([3, 8])
                .style(style)
                .on_press(Message::SelectTab(i)),
        );
        tabs = tabs.push(
            button(text("x").size(11))
                .padding([3, 5])
                .style(button::text)
                .on_press(Message::CloseTab(i)),
        );
    }
    let body: Element<'_, Message> = match app.docs.get(app.active) {
        Some(doc) => scrollable(slots(app, doc)).height(Length::Fill).into(),
        None => container(
            text("Pick a hull on the Ships tab, or paste an EFT fit with Ctrl+V")
                .style(text::secondary),
        )
        .padding(20)
        .into(),
    };
    column![
        scrollable(tabs).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
        )),
        body
    ]
    .spacing(6)
    .height(Length::Fill)
    .into()
}

type SlotFilter = fn(Slot) -> bool;

fn state_color(state: State) -> Color {
    match state {
        State::Offline => Color::from_rgb(0.45, 0.45, 0.45),
        State::Online => Color::from_rgb(0.55, 0.65, 0.85),
        State::Active => Color::from_rgb(0.35, 0.85, 0.45),
        State::Overload => Color::from_rgb(0.95, 0.45, 0.25),
    }
}

fn slots<'a>(app: &'a App, doc: &'a Doc) -> Element<'a, Message> {
    let fit = &doc.doc.fit;
    let calc = &doc.doc.calc;
    let layout = &doc.doc.layout;
    let mut col = column![].spacing(2).padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 0.0,
    });

    if let Ok(game) = &app.game
        && let Some(ty) = game.sde.get_type(fit.ship.type_id)
        && let Some(modes) = ty.mode_type_ids()
        && !modes.is_empty()
    {
        let mut line = row![text("Mode").size(12).style(text::secondary)]
            .spacing(4)
            .align_y(iced::Alignment::Center);
        for mode in modes {
            let label = app
                .type_name(mode)
                .rsplit(' ')
                .nth(1)
                .unwrap_or("")
                .to_string();
            let style = if fit.ship.mode == Some(mode) {
                button::primary
            } else {
                button::secondary
            };
            line = line.push(
                button(text(label).size(11))
                    .padding([1, 6])
                    .style(style)
                    .on_press(Message::SetMode(Some(mode))),
            );
        }
        col = col.push(container(line).padding([4, 4]));
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
        let header = format!("{} {used}/{count}", rack.label());
        let header_style = if used > count as usize {
            text::danger
        } else {
            text::secondary
        };
        col = col.push(
            container(text(header).size(12).style(header_style)).padding(iced::Padding {
                top: 6.0,
                right: 4.0,
                bottom: 2.0,
                left: 4.0,
            }),
        );
        for index in 0..count.max(used as u8) {
            let slot = rack.slot(index);
            match fit.items.iter().position(|it| it.slot == slot) {
                Some(i) => col = col.push(item_row(app, doc, i)),
                None => {
                    col = col.push(
                        container(
                            text(format!("[Empty {} slot]", rack.label()))
                                .size(12)
                                .style(text::secondary),
                        )
                        .padding([2, 8]),
                    );
                }
            }
        }
    }

    let sections: [(&str, SlotFilter); 5] = [
        ("Drones", |s| s == Slot::DroneBay),
        ("Fighters", |s| {
            matches!(s, Slot::FighterTube(_) | Slot::FighterBay)
        }),
        ("Implants", |s| matches!(s, Slot::Implant(_))),
        ("Boosters", |s| matches!(s, Slot::Booster(_))),
        ("Cargo", |s| s == Slot::Cargo),
    ];
    for (label, matches) in sections {
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
        col = col.push(
            container(text(label).size(12).style(text::secondary)).padding(iced::Padding {
                top: 6.0,
                right: 4.0,
                bottom: 2.0,
                left: 4.0,
            }),
        );
        for i in indexes {
            col = col.push(item_row(app, doc, i));
        }
    }

    col = col.push(implant_sets(app));
    col = col.push(projected(app, doc));

    if let Some(violations) = &calc.violations
        && !violations.is_empty()
    {
        col = col.push(
            container(text("Violations").size(12).style(text::danger)).padding(iced::Padding {
                top: 8.0,
                right: 4.0,
                bottom: 2.0,
                left: 4.0,
            }),
        );
        for v in violations {
            col = col.push(
                container(text(app.describe_violation(v)).size(11).style(text::danger))
                    .padding([1, 8]),
            );
        }
    }
    col.into()
}

fn item_row<'a>(app: &'a App, doc: &'a Doc, i: usize) -> Element<'a, Message> {
    let item = &doc.doc.fit.items[i];
    let result = doc.doc.calc.items.get(i);
    let state = result.map(|r| r.state).unwrap_or(item.state);
    let mut name = app.type_name(item.type_id).to_string();
    if item.quantity > 1 {
        name = format!("{name} x{}", item.quantity);
    }
    let charge = item
        .charge
        .as_ref()
        .map(|c| app.type_name(c.type_id).to_string())
        .unwrap_or_default();
    let selected = app.selected_item == Some(i);

    let state_dot = mouse_area(
        container(text(state_label(state)).size(11).color(state_color(state)))
            .width(48)
            .padding([2, 4]),
    )
    .on_press(Message::CycleState(i))
    .on_right_press(Message::ToggleOverload(i))
    .on_middle_press(Message::ToggleOffline(i));

    let info = item_info(app, doc, i);
    let mut line = row![
        state_dot,
        text(name)
            .size(12)
            .width(Length::FillPortion(3))
            .wrapping(text::Wrapping::None),
        charge_cell(i, charge),
        text(info)
            .size(11)
            .style(text::secondary)
            .width(Length::FillPortion(2))
            .wrapping(text::Wrapping::None),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    if item.slot == Slot::DroneBay || item.slot == Slot::Cargo {
        line = line.push(
            button(text("-").size(11))
                .padding([1, 5])
                .style(button::text)
                .on_press(Message::Quantity(i, item.quantity.saturating_sub(1))),
        );
        line = line.push(
            button(text("+").size(11))
                .padding([1, 5])
                .style(button::text)
                .on_press(Message::Quantity(i, item.quantity + 1)),
        );
    }
    line = line.push(
        button(text("x").size(11))
            .padding([1, 5])
            .style(button::text)
            .on_press(Message::RemoveItem(i)),
    );

    let style = move |theme: &iced::Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: selected.then(|| palette.primary.weak.color.into()),
            ..container::Style::default()
        }
    };
    mouse_area(
        container(line)
            .padding([1, 4])
            .style(style)
            .width(Length::Fill),
    )
    .on_press(Message::SelectItem(Some(i)))
    .into()
}

fn implant_sets(app: &App) -> Element<'_, Message> {
    let mut line = row![
        text("Implant sets").size(12).style(text::secondary),
        button(text("save current").size(10))
            .padding([1, 5])
            .style(button::secondary)
            .on_press(Message::SaveImplantSet),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);
    for (i, set) in app.implant_sets.iter().enumerate() {
        line = line.push(
            row![
                button(text(&set.name).size(10))
                    .padding([1, 5])
                    .style(button::primary)
                    .on_press(Message::ApplyImplantSet(i)),
                button(text("x").size(10))
                    .padding([1, 3])
                    .style(button::text)
                    .on_press(Message::DeleteImplantSet(i)),
            ]
            .align_y(iced::Alignment::Center),
        );
    }
    container(line.wrap()).padding([6, 4]).into()
}

fn projected<'a>(app: &'a App, doc: &'a Doc) -> Element<'a, Message> {
    let mut col =
        column![container(text("Projected").size(12).style(text::secondary)).padding([6, 4])]
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
                    .text_size(11)
                    .padding([2, 6]),
            )
            .padding([1, 4]),
        );
    }
    let mut tabs = row![text("Project fit").size(11).style(text::secondary)]
        .spacing(3)
        .align_y(iced::Alignment::Center);
    let mut any_tab = false;
    for (i, other) in app.docs.iter().enumerate() {
        if i == app.active {
            continue;
        }
        any_tab = true;
        tabs = tabs.push(
            button(text(app.fit_title(other)).size(11))
                .padding([1, 6])
                .style(button::secondary)
                .on_press(Message::ProjectFit(i)),
        );
    }
    if any_tab {
        col = col.push(container(tabs.wrap()).padding([1, 4]));
    }
    for (i, p) in doc.doc.projections.iter().enumerate() {
        if matches!(p, Projected::Beacon(_)) {
            continue;
        }
        col = col.push(
            row![
                text(app.projection_label(p)).size(12).width(Length::Fill),
                button(text("x").size(11))
                    .padding([1, 5])
                    .style(button::text)
                    .on_press(Message::RemoveProjection(i)),
            ]
            .padding([1, 8])
            .align_y(iced::Alignment::Center),
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
    parts.join("  ")
}

fn charge_cell<'a>(i: usize, charge: String) -> Element<'a, Message> {
    if charge.is_empty() {
        return text("").width(Length::FillPortion(2)).into();
    }
    mouse_area(
        text(charge)
            .size(11)
            .style(text::secondary)
            .width(Length::FillPortion(2))
            .wrapping(text::Wrapping::None),
    )
    .on_right_press(Message::LoadCharge(i, None))
    .into()
}

fn state_label(state: State) -> &'static str {
    match state {
        State::Offline => "offline",
        State::Online => "online",
        State::Active => "active",
        State::Overload => "heat",
    }
}
