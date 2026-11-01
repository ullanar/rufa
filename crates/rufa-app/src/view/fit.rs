use iced::widget::responsive;
use iced::widget::{
    button, column, container, image, mouse_area, pick_list, pin, row, space, stack, text,
    text_input, tooltip,
};
use iced::{Color, Length};
use rufa_data::icons::Kind;
use rufa_sim::drones;
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
        let active = i == app.active;
        let lit = active || app.hovered_tab == Some(i);
        let icon: El<'_> = match app.icon(Kind::Icon, doc.doc.fit.ship.type_id) {
            Some(handle) => image(handle).width(14).height(14).into(),
            None => space().width(0).into(),
        };
        let mut label = row![
            icon,
            text(ui::clip(&app.fit_title(doc), 28))
                .size(ui::BODY)
                .font(theme::UI_MEDIUM)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(ui::GAP)
        .align_y(iced::Alignment::Center);
        if doc.dirty {
            label = label.push(text("•").size(ui::BODY).color(theme::ACCENT));
        }
        let close =
            button(
                text("×")
                    .size(ui::SMALL)
                    .color(if lit { theme::TEXT } else { theme::FAINT }),
            )
            .padding([3, 5])
            .style(theme::ghost)
            .on_press(Message::CloseTab(i));
        tabs = tabs.push(
            mouse_area(
                container(
                    row![
                        button(label)
                            .padding(ui::field(4.0, 8.0))
                            .style(theme::tab(active))
                            .on_press(Message::SelectTab(i)),
                        close,
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .style(theme::tab_shell(active, lit)),
            )
            .on_middle_press(Message::CloseTab(i))
            .on_enter(Message::HoverTab(Some(i)))
            .on_exit(Message::HoverTab(None)),
        );
    }
    let body: El<'_> = match app.docs.get(app.active) {
        Some(doc) => responsive(move |size| {
            let list = mouse_area(ui::scroll(slots(app, doc, size.width)).height(Length::Fill))
                .on_move(Message::CursorMoved);
            let mut layers: Vec<El<'_>> = vec![list.into()];
            if let Some((i, at)) = app.menu
                && i < doc.doc.fit.items.len()
            {
                let width = 360.0f32.min(size.width - 16.0);
                let x = at.x.min((size.width - width - 8.0).max(0.0));
                let y = at.y.min((size.height - 60.0).max(0.0));
                layers.push(
                    mouse_area(space().width(Length::Fill).height(Length::Fill))
                        .on_press(Message::CloseMenu)
                        .on_right_press(Message::CloseMenu)
                        .into(),
                );
                let max_height = (size.height - y - 8.0).max(120.0);
                layers.push(
                    pin(container(context_menu(app, doc, i, width)).max_height(max_height))
                        .x(x)
                        .y(y)
                        .into(),
                );
            }
            stack(layers)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
        .into(),
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
    if app.docs.is_empty() {
        return body;
    }
    let strip = container(ui::scroll_x(tabs))
        .padding(2)
        .width(Length::Fill)
        .style(theme::rounded(theme::SUNKEN, 6.0));
    column![strip, body]
        .spacing(ui::GAP_ROW)
        .height(Length::Fill)
        .into()
}

fn hull_card<'a>(app: &'a App, doc: &'a Doc, width: f32) -> El<'a> {
    let compact = width < 700.0;
    let inner = (width - 28.0).max(120.0);
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
        Some(handle) => image(handle).width(48).height(48).into(),
        None => container(space().width(48).height(48))
            .style(theme::rounded(theme::SUNKEN, 6.0))
            .into(),
    };
    let r = &doc.doc.stats.resources;
    let tiles: [(&str, f64, f64); 2] = [
        ("CPU", r.cpu_used, r.cpu_total),
        ("PG", r.power_used, r.power_total),
    ];
    let count = tiles.len() as f32;
    let gaps = ui::GAP_ROW * (count - 1.0);
    let block = if compact {
        inner
    } else {
        (count * 84.0 + gaps).min(inner * 0.5)
    };
    let tile_width = ((block - gaps) / count).max(44.0);
    let mut gauges = row![].spacing(ui::GAP_ROW);
    for (name, used, total) in tiles {
        gauges = gauges.push(gauge_tile(name, used, total, tile_width));
    }

    let name = container(
        column![
            ui::title(app.type_name(fit.ship.type_id)).wrapping(text::Wrapping::None),
            ui::muted(group).wrapping(text::Wrapping::None),
        ]
        .spacing(2),
    )
    .clip(true)
    .width(Length::Fill);
    let head: El<'a> = if compact {
        column![
            row![picture, name]
                .spacing(ui::PAD)
                .align_y(iced::Alignment::Center),
            gauges,
        ]
        .spacing(ui::GAP_ROW)
        .into()
    } else {
        row![picture, name, gauges]
            .spacing(ui::PAD)
            .align_y(iced::Alignment::Center)
            .into()
    };

    container(column![head, ui::rule(), summary(app, doc)].spacing(ui::GAP_ROW))
        .padding(ui::PAD)
        .width(Length::Fill)
        .style(theme::card)
        .into()
}

fn gauge_tile<'a>(name: &'a str, used: f64, total: f64, width: f32) -> El<'a> {
    let fraction = if total > 0.0 {
        (used / total) as f32
    } else {
        0.0
    };
    column![
        container(
            row![
                ui::label(name),
                text(format!("{}%", fmt::num(f64::from(fraction) * 100.0, 0)))
                    .size(ui::MICRO)
                    .font(theme::UI_MEDIUM)
                    .color(ui::gauge_text_color(fraction))
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(ui::GAP)
            .align_y(iced::Alignment::Center)
        )
        .clip(true)
        .width(Length::Fill),
        ui::bar(fraction, ui::gauge_color(fraction), 3.0),
    ]
    .spacing(2)
    .width(width)
    .into()
}

fn summary<'a>(app: &'a App, doc: &'a Doc) -> El<'a> {
    let s = &doc.doc.stats;
    let cap = &s.capacitor;
    let (cap_shown, cap_color) = if cap.stable_pct > 0.0 {
        (
            format!("stable {:.0}%", cap.stable_pct),
            theme::SUCCESS_TEXT,
        )
    } else {
        (fmt::seconds(cap.depletes_in), theme::WARNING_TEXT)
    };
    let dps = s.offense.dps + s.offense.drone_dps + s.offense.fighter_dps;
    let mut line = row![
        figure("DPS", fmt::num(dps, 1), theme::TEXT),
        figure("EHP", fmt::num(s.defense.ehp, 0), theme::TEXT),
        figure(
            "SPEED",
            format!("{} m/s", fmt::num(s.navigation.max_velocity, 0)),
            theme::TEXT
        ),
        figure("CAP", cap_shown, cap_color),
    ]
    .spacing(ui::PAD);
    if let Some(price) = app.fit_price(doc) {
        line = line.push(figure(
            "FIT",
            format!("{} ISK", fmt::short(price.hull)),
            theme::TEXT_SECONDARY,
        ));
        if price.head > 0.0 {
            line = line.push(figure(
                "HEAD",
                format!("{} ISK", fmt::short(price.head)),
                theme::TEXT_SECONDARY,
            ));
        }
    }
    line.wrap().into()
}

fn figure<'a>(name: &'a str, shown: String, color: Color) -> El<'a> {
    row![
        ui::label(name),
        text(shown)
            .size(ui::SMALL)
            .font(theme::UI_MEDIUM)
            .color(color)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center)
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

fn empty_row<'a>(label: String, rail: Color, rack: Option<Rack>) -> El<'a> {
    let line = container(
        row![
            container(space().width(3).height(Length::Fill)).style(theme::rounded(rail, 1.0)),
            text(label).size(ui::SMALL).color(theme::FAINT),
        ]
        .spacing(ui::GAP_ROW)
        .align_y(iced::Alignment::Center),
    )
    .height(ui::ROW)
    .padding([0, 4])
    .width(Length::Fill);
    // An empty slot is the shortest way to ask for a module that fits it.
    match rack {
        Some(rack) => mouse_area(line)
            .on_press(Message::PickForRack(Some(rack)))
            .into(),
        None => line.into(),
    }
}

type SlotFilter = fn(Slot) -> bool;

const CHAR: f32 = 6.6;

fn name_budget(width: f32, fixed: f32) -> usize {
    (((width - fixed) / CHAR) as usize).clamp(6, 60)
}

fn slots<'a>(app: &'a App, doc: &'a Doc, width: f32) -> El<'a> {
    let compact = width < 700.0;
    let fit = &doc.doc.fit;
    // The charge column takes what the longest loaded charge needs and no more,
    // so a short name like EMP S leaves the rest to the module names.
    let longest_charge = fit
        .items
        .iter()
        .filter(|item| Rack::of_slot(item.slot).is_some())
        .filter_map(|item| item.charge.as_ref())
        .map(|c| app.type_name(c.type_id).chars().count())
        .max()
        .unwrap_or(0);
    let charge_width = if longest_charge == 0 {
        0.0
    } else {
        let wanted = longest_charge as f32 * CHAR + 20.0;
        wanted.clamp(40.0, if compact { 90.0 } else { 140.0 })
    };
    let info_width = if compact { 0.0 } else { 200.0 };
    let module_names = name_budget(width, 117.0 + charge_width + info_width);
    let layout = &doc.doc.layout;
    let mut col = column![].spacing(1).padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 0.0,
    });
    col = col.push(hull_card(app, doc, width));

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
                    .padding(ui::pill(8.0))
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
                Some(i) => {
                    col = col.push(item_row(
                        app,
                        doc,
                        i,
                        rail_color(rack),
                        compact,
                        module_names,
                        charge_width,
                    ))
                }
                None => {
                    col = col.push(empty_row(
                        format!("{} slot", rack.label()),
                        rail_color(rack),
                        Some(rack),
                    ))
                }
            }
        }
    }

    if let Some(section) = drone_section(app, doc, compact, width) {
        col = col.push(section);
    }

    let sections: [(&str, SlotFilter, Color); 4] = [
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
            col = col.push(item_row(
                app,
                doc,
                i,
                rail,
                compact,
                module_names,
                charge_width,
            ));
        }
    }

    col = col.push(implant_sets(app));
    col = col.push(projected(app, doc));
    col.into()
}

fn drone_section<'a>(app: &'a App, doc: &'a Doc, compact: bool, width: f32) -> Option<El<'a>> {
    let Ok(game) = &app.game else { return None };
    let ids = app.ids?;
    let types = drones::types(&doc.doc.fit);
    let load = drones::load(&ids, &doc.doc.calc);
    if types.is_empty() && load.bay_total <= 0.0 {
        return None;
    }
    let mut col = column![drone_header(load, compact)].spacing(1);
    if types.is_empty() {
        col = col.push(empty_row(
            "Drone bay".to_string(),
            theme::RAIL_SERVICE,
            None,
        ));
    }
    for type_id in types {
        let room = drones::room(&game.info(), &ids, &doc.doc.calc, type_id);
        let pips = f32::from(load.max_launched.clamp(1, 32) as u16) * 12.0;
        let names = name_budget(width, 163.0 + pips + if compact { 0.0 } else { 160.0 });
        col = col.push(drone_row(app, doc, type_id, room, load, compact, names));
    }
    Some(col.into())
}

fn drone_header<'a>(load: drones::Load, compact: bool) -> El<'a> {
    let pair = |used: f64, total: f64, unit: &str| {
        let color = if used > total {
            theme::DANGER_TEXT
        } else {
            theme::MUTED
        };
        text(format!(
            "{} / {}{unit}",
            fmt::num(used, 1),
            fmt::num(total, 1)
        ))
        .size(ui::MICRO)
        .font(theme::UI_MEDIUM)
        .color(color)
    };
    let launched_color = if load.launched > load.max_launched {
        theme::DANGER_TEXT
    } else {
        theme::TEXT_SECONDARY
    };
    let mut line = row![
        ui::eyebrow("Drones").width(Length::Fill),
        ui::label("IN SPACE"),
        text(format!("{} / {}", load.launched, load.max_launched))
            .size(ui::MICRO)
            .font(theme::UI_MEDIUM)
            .color(launched_color),
    ]
    .spacing(ui::GAP_ROW)
    .align_y(iced::Alignment::Center);
    if !compact {
        line = line.push(ui::label("BANDWIDTH"));
        line = line.push(pair(load.bandwidth_used, load.bandwidth_total, " Mbit"));
        line = line.push(ui::label("BAY"));
        line = line.push(pair(load.bay_used, load.bay_total, " m3"));
    }
    container(line)
        .padding(iced::Padding {
            top: 8.0,
            right: 6.0,
            bottom: 2.0,
            left: 2.0,
        })
        .width(Length::Fill)
        .into()
}

fn drone_pips<'a>(type_id: i32, stack: drones::Stack, room: drones::Room, cap: u32) -> El<'a> {
    let boxes = cap.max(stack.launched).clamp(1, 32);
    let mut line = row![].spacing(2);
    for count in 1..=boxes {
        let state = if count <= stack.launched {
            theme::Pip::Launched
        } else if count > stack.total() {
            theme::Pip::Missing
        } else if count <= stack.launched + room.launch {
            theme::Pip::Open
        } else {
            theme::Pip::Blocked
        };
        let mut pip = button(space().width(8).height(8))
            .padding(2)
            .style(theme::pip(state));
        if matches!(state, theme::Pip::Launched | theme::Pip::Open) {
            let want = if count == stack.launched {
                count - 1
            } else {
                count
            };
            pip = pip.on_press(Message::DroneLaunched(type_id, want));
        }
        line = line.push(pip);
    }
    tooltip(
        line,
        container(
            text(format!(
                "{} of {} in space; click a box to launch that many",
                stack.launched,
                stack.total()
            ))
            .size(ui::SMALL),
        )
        .padding(ui::field(4.0, 8.0))
        .style(theme::tooltip),
        tooltip::Position::Left,
    )
    .delay(std::time::Duration::from_millis(400))
    .into()
}

fn drone_row<'a>(
    app: &'a App,
    doc: &'a Doc,
    type_id: i32,
    room: drones::Room,
    load: drones::Load,
    compact: bool,
    names: usize,
) -> El<'a> {
    let fit = &doc.doc.fit;
    let stack = drones::stack(fit, type_id);
    let indexes = drones::indexes(fit, type_id);
    let first = indexes.first().copied().unwrap_or(0);
    let selected = indexes.iter().any(|i| app.selected.contains(i));
    let icon: El<'a> = match app.icon(Kind::Icon, type_id) {
        Some(handle) => image(handle).width(18).height(18).into(),
        None => space().width(18).height(18).into(),
    };
    let rail = if stack.launched > 0 {
        theme::ACTIVE
    } else {
        theme::RAIL_SERVICE
    };
    let info = indexes
        .iter()
        .find(|i| fit.items[**i].state != rufa_sim::State::Offline)
        .map(|i| item_info(app, doc, *i))
        .unwrap_or_default();

    let mut line = row![
        container(space().width(3).height(Length::Fill)).style(theme::rounded(rail, 1.0)),
        icon,
        container(
            text(ui::clip(app.type_name(type_id), names))
                .size(ui::BODY)
                .font(theme::UI_MEDIUM)
                .color(theme::TEXT)
                .wrapping(text::Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill),
        drone_pips(type_id, stack, room, load.max_launched),
    ]
    .spacing(ui::GAP_ROW)
    .align_y(iced::Alignment::Center);

    if !compact {
        line = line.push(info_cell(info, 160.0));
    }

    line = line.push(
        text(format!("x{}", stack.total()))
            .size(ui::SMALL)
            .color(theme::TEXT_SECONDARY)
            .wrapping(text::Wrapping::None),
    );
    line = line.push(step(
        "−",
        (stack.total() > 0).then(|| Message::DroneCarried(type_id, stack.total() - 1)),
    ));
    line = line.push(step(
        "+",
        (room.bay > 0).then(|| Message::DroneCarried(type_id, stack.total() + 1)),
    ));
    line = line.push(
        button(text("×").size(ui::SMALL))
            .padding([1, 5])
            .style(theme::ghost)
            .on_press(Message::DroneCarried(type_id, 0)),
    );

    mouse_area(
        container(line)
            .padding([0, 4])
            .height(ui::ROW)
            .align_y(iced::Alignment::Center)
            .style(theme::row(selected, app.hovered_item == Some(first)))
            .width(Length::Fill),
    )
    .on_press(Message::SelectItem(Some(first)))
    .on_right_press(Message::OpenMenu(first))
    .on_enter(Message::HoverItem(Some(first)))
    .on_exit(Message::HoverItem(None))
    .into()
}

fn info_cell<'a>(parts: Vec<String>, width: f32) -> El<'a> {
    let budget = (width / 6.1) as usize;
    let mut shown = String::new();
    for part in parts {
        let next = if shown.is_empty() {
            part
        } else {
            format!("{shown}  ·  {part}")
        };
        if next.chars().count() > budget {
            break;
        }
        shown = next;
    }
    container(
        text(ui::clip(&shown, budget))
            .size(ui::SMALL)
            .color(theme::MUTED)
            .wrapping(text::Wrapping::None)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right),
    )
    .clip(true)
    .width(width)
    .into()
}

fn step<'a>(glyph: &'static str, msg: Option<Message>) -> El<'a> {
    let color = if msg.is_some() {
        theme::TEXT_SECONDARY
    } else {
        theme::DISABLED
    };
    let mut b = button(text(glyph).size(ui::SMALL).color(color))
        .padding([1, 5])
        .style(theme::ghost);
    if let Some(msg) = msg {
        b = b.on_press(msg);
    }
    b.into()
}

/// A strip above the status bar: the count and the first problem, with the
/// whole list one click away. Absent when the fit is legal.
pub fn violations<'a>(app: &'a App) -> El<'a> {
    let Some(doc) = app.docs.get(app.active) else {
        return space().height(0).into();
    };
    let list = doc.doc.calc.violations.as_deref().unwrap_or(&[]);
    let Some(first) = list.first() else {
        return space().height(0).into();
    };
    let count = if list.len() == 1 {
        "1 problem".to_string()
    } else {
        format!("{} problems", list.len())
    };
    let head = row![
        text("\u{25b2}").size(ui::SMALL).color(theme::DANGER_TEXT),
        text(count)
            .font(theme::UI_MEDIUM)
            .size(ui::SMALL)
            .color(theme::DANGER_TEXT)
            .wrapping(text::Wrapping::None),
        text(app.describe_violation(first))
            .size(ui::SMALL)
            .color(theme::DANGER_TEXT)
            .width(Length::Fill)
            .wrapping(text::Wrapping::None),
        text(if app.violations_open {
            "\u{25b4}"
        } else {
            "\u{25be}"
        })
        .size(ui::SMALL)
        .color(theme::DANGER_TEXT),
    ]
    .spacing(ui::GAP_ROW)
    .align_y(iced::Alignment::Center);
    let mut body = column![
        button(head)
            .padding(0)
            .style(theme::bare)
            .on_press(Message::ToggleViolations)
    ]
    .spacing(2);
    if app.violations_open {
        for v in list.iter().skip(1) {
            body = body.push(
                row![
                    space().width(ui::SMALL),
                    text(app.describe_violation(v))
                        .size(ui::SMALL)
                        .color(theme::DANGER_TEXT),
                ]
                .spacing(ui::GAP_ROW),
            );
        }
    }
    container(body)
        .padding([4, 10])
        .width(Length::Fill)
        .style(theme::banner(theme::DANGER_FILL, theme::DANGER_LINE))
        .into()
}

fn implant_sets(app: &App) -> El<'_> {
    let mut line = row![
        ui::label("IMPLANT SETS"),
        button(text("save current").size(ui::SMALL))
            .padding(ui::pill(8.0))
            .style(theme::ghost)
            .on_press(Message::SaveImplantSet),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center);
    for (i, set) in app.implant_sets.iter().enumerate() {
        line = line.push(
            row![
                button(text(&set.name).size(ui::SMALL))
                    .padding(ui::pill(8.0))
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
                    .padding(ui::field(4.0, 8.0))
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
                .padding(ui::pill(8.0))
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
                    container(ui::body(app.projection_label(p)).wrapping(text::Wrapping::None))
                        .clip(true)
                        .width(Length::Fill),
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

fn item_info(app: &App, doc: &Doc, i: usize) -> Vec<String> {
    let (Some(ids), Some(result)) = (app.ids, doc.doc.calc.items.get(i)) else {
        return Vec::new();
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
    parts
}

fn item_row<'a>(
    app: &'a App,
    doc: &'a Doc,
    i: usize,
    rail: Color,
    compact: bool,
    names: usize,
    charge_width: f32,
) -> El<'a> {
    let item = &doc.doc.fit.items[i];
    let result = doc.doc.calc.items.get(i);
    let state = result.map(|r| r.state).unwrap_or(item.state);
    // A drone or a cargo row carries its own count field, so the name says it
    // only where no field follows.
    let counted = item.slot == Slot::DroneBay || item.slot == Slot::Cargo;
    let mut name = app.type_name(item.type_id).to_string();
    if item.quantity > 1 && !counted {
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

    // Cargo carries no state. An implant or a booster works the moment it is
    // plugged in, and the engine applies it whatever the state says, so a
    // control there would lie about what the numbers do.
    let glyph: El<'a> = if item.slot == Slot::Cargo {
        space().width(18).into()
    } else if matches!(item.slot, Slot::Implant(_) | Slot::Booster(_)) {
        container(ui::state_glyph(State::Online))
            .padding([0, 2])
            .into()
    } else {
        tooltip(
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
            .padding(ui::field(4.0, 8.0))
            .style(theme::tooltip),
            tooltip::Position::Right,
        )
        .delay(std::time::Duration::from_millis(400))
        .into()
    };

    let icon: El<'a> = match app.icon(Kind::Icon, item.type_id) {
        Some(handle) => image(handle).width(18).height(18).border_radius(3.0).into(),
        None => space().width(18).height(18).into(),
    };
    let mut line = row![
        container(space().width(3).height(Length::Fill)).style(theme::rounded(rail_shown, 1.0)),
        glyph,
        icon,
        container(
            text(ui::clip(&name, names))
                .size(ui::BODY)
                .font(theme::UI_MEDIUM)
                .color(theme::TEXT)
                .wrapping(text::Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill),
        charge_cell(
            i,
            charge,
            charge_width,
            item.charge
                .as_ref()
                .and_then(|c| app.icon(Kind::Icon, c.type_id)),
        ),
        info_cell(
            if compact {
                Vec::new()
            } else {
                item_info(app, doc, i)
            },
            if compact { 0.0 } else { 200.0 },
        ),
    ]
    .spacing(ui::GAP_ROW)
    .align_y(iced::Alignment::Center);

    if counted {
        let shown = match &app.quantity_edit {
            Some((at, text)) if *at == i => text.clone(),
            _ => item.quantity.to_string(),
        };
        line = line.push(
            button(text("−").size(ui::SMALL))
                .padding([1, 5])
                .style(theme::ghost)
                .on_press(Message::Quantity(i, item.quantity.saturating_sub(1))),
        );
        line = line.push(
            text_input("", &shown)
                .on_input(move |s| Message::QuantityInput(i, s))
                .on_submit(Message::QuantityDone)
                .size(ui::SMALL)
                .padding(ui::field(1.0, 4.0))
                .width(48)
                .align_x(iced::alignment::Horizontal::Right)
                .style(theme::input),
        );
        line = line.push(
            button(text("+").size(ui::SMALL))
                .padding([1, 5])
                .style(theme::ghost)
                .on_press(Message::Quantity(i, item.quantity + 1)),
        );
    }
    // The remove button shows under the cursor. Its space stays reserved, so a
    // row never shifts as the pointer passes over it.
    let hovered = app.hovered_item == Some(i) || selected;
    let remove: El<'a> = if hovered {
        button(text("×").size(ui::SMALL))
            .padding([1, 5])
            .style(theme::ghost)
            .on_press(Message::RemoveItem(i))
            .into()
    } else {
        space().width(19).into()
    };
    line = line.push(remove);

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

fn context_menu<'a>(app: &'a App, doc: &'a Doc, i: usize, width: f32) -> El<'a> {
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
            .padding(ui::pill_tight(7.0))
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
    let is_drone = item.slot == Slot::DroneBay;
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
    if item.slot != Slot::Cargo {
        col = col.push(entry("Move To Cargo".to_string(), Message::MoveToCargo(i)));
    }
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
                for (label, ids) in meta_groups(game, variants) {
                    col = col.push(text(label).size(ui::MICRO).color(theme::MUTED));
                    for (id, vname) in ids {
                        let on = id == item.type_id;
                        col = col.push(
                            button(
                                container(
                                    text(vname)
                                        .size(ui::SMALL)
                                        .color(if on { theme::ACCENT } else { theme::TEXT })
                                        .wrapping(text::Wrapping::None),
                                )
                                .clip(true)
                                .width(Length::Fill),
                            )
                            .padding(ui::pill(8.0))
                            .width(Length::Fill)
                            .style(theme::list_item(on))
                            .on_press(Message::MenuVariant(id)),
                        );
                    }
                }
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
    if is_drone {
        let stack = drones::stack(&doc.doc.fit, item.type_id);
        col = col.push(ui::rule());
        col = col.push(entry("Fill The Drone Bay".to_string(), Message::MenuFill));
        col = col.push(ui::label("IN SPACE"));
        col = col.push(
            row![
                chip(
                    "Launch all".to_string(),
                    stack.launched == stack.total(),
                    Message::DroneLaunched(item.type_id, stack.total()),
                ),
                chip(
                    "Recall all".to_string(),
                    stack.launched == 0,
                    Message::DroneLaunched(item.type_id, 0),
                ),
            ]
            .spacing(3),
        );
        return container(ui::scroll(col))
            .padding(ui::PAD)
            .width(width)
            .style(theme::tooltip)
            .into();
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
    container(ui::scroll(col))
        .padding(ui::PAD)
        .width(width)
        .style(theme::tooltip)
        .into()
}

fn meta_groups<'a>(
    game: &'static rufa_data::Game,
    variants: &'a [rufa_data::TypeRef],
) -> Vec<(&'static str, Vec<(i32, &'a str)>)> {
    let order: [(i32, &'static str); 7] = [
        (1, "Tech I"),
        (2, "Tech II"),
        (3, "Storyline"),
        (4, "Faction"),
        (6, "Deadspace"),
        (5, "Officer"),
        (0, "Other"),
    ];
    let mut groups: Vec<(&'static str, Vec<(i32, &'a str)>)> =
        order.iter().map(|(_, l)| (*l, Vec::new())).collect();
    for v in variants {
        let meta = game
            .sde
            .get_type(v.id)
            .map(|t| t.meta_group_id())
            .unwrap_or(0);
        let slot = order
            .iter()
            .position(|(m, _)| *m == meta)
            .unwrap_or(order.len() - 1);
        groups[slot].1.push((v.id, v.name.as_str()));
    }
    groups.retain(|(_, list)| !list.is_empty());
    groups
}

fn state_name(state: State) -> &'static str {
    match state {
        State::Offline => "Offline",
        State::Online => "Online",
        State::Active => "Active",
        State::Overload => "Overloaded",
    }
}

fn charge_cell<'a>(i: usize, charge: String, width: f32, icon: Option<image::Handle>) -> El<'a> {
    if charge.is_empty() {
        return space().width(width).into();
    }
    let picture: El<'a> = match icon {
        Some(handle) => image(handle).width(14).height(14).border_radius(2.0).into(),
        None => space().width(14).height(14).into(),
    };
    mouse_area(
        container(
            row![
                picture,
                container(
                    text(ui::clip(&charge, ((width - 18.0) / 6.0) as usize))
                        .size(ui::SMALL)
                        .color(theme::TEXT_SECONDARY)
                        .wrapping(text::Wrapping::None),
                )
                .clip(true)
                .width(Length::Fill),
            ]
            .spacing(3)
            .align_y(iced::Alignment::Center),
        )
        .clip(true)
        .width(width),
    )
    .on_right_press(Message::LoadCharge(i, None))
    .into()
}
