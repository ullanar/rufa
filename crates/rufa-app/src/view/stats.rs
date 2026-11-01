use iced::widget::{button, column, container, row, scrollable, slider, space, text};
use iced::{Color, Length};
use rufa_data::Info;
use rufa_sim::stats::{Layer, Stats};

use crate::fmt;
use crate::theme;
use crate::ui::{self, El};
use crate::{App, DamagePreset, Message};

pub fn view(app: &App) -> El<'_> {
    let Some(doc) = app.docs.get(app.active) else {
        return container(ui::muted("No fit open"))
            .center(Length::Fill)
            .into();
    };
    let s = &doc.doc.stats;
    let r = &s.resources;
    let worst = [
        r.cpu_used / r.cpu_total.max(1e-9),
        r.power_used / r.power_total.max(1e-9),
        r.calibration_used / r.calibration_total.max(1e-9),
    ]
    .into_iter()
    .fold(0.0f64, f64::max);
    let cap = &s.capacitor;
    let cap_head = if cap.stable_pct > 0.0 {
        format!("{:.0}%", cap.stable_pct)
    } else {
        fmt::seconds(cap.depletes_in)
    };
    let mut col = column![
        ui::section(
            "Resources",
            Some(format!("{:.0}%", worst * 100.0)),
            resources(s)
        ),
        ui::section("Defense", Some(fmt::num(s.defense.ehp, 0)), defense(app, s)),
        ui::section(
            "Offense",
            Some(fmt::num(
                s.offense.dps + s.offense.drone_dps + s.offense.fighter_dps,
                1
            )),
            offense(app, doc, s)
        ),
        ui::section("Capacitor", Some(cap_head), capacitor(doc, s)),
        ui::section(
            "Navigation",
            Some(format!("{} m/s", fmt::num(s.navigation.max_velocity, 0))),
            navigation(s)
        ),
        ui::section(
            "Targeting",
            Some(fmt::distance(s.targeting.max_range)),
            targeting(s)
        ),
        ui::section(
            "Price",
            app.fit_price(doc).map(|v| format!("{} ISK", fmt::short(v))),
            price(app, doc)
        ),
    ]
    .spacing(ui::SECTION_GAP);
    if let Some(item) = app.selected_item.filter(|i| *i < doc.doc.fit.items.len()) {
        col = col.push(ui::section(
            "Selected item",
            None,
            item_attributes(app, doc, item),
        ));
    }
    scrollable(container(col).padding(iced::Padding {
        top: 0.0,
        right: 10.0,
        bottom: 0.0,
        left: 0.0,
    }))
    .style(theme::scroll)
    .height(Length::Fill)
    .into()
}

fn resources(s: &Stats) -> El<'_> {
    let r = &s.resources;
    column![
        ui::gauge("CPU", r.cpu_used, r.cpu_total, "tf"),
        ui::gauge("Powergrid", r.power_used, r.power_total, "MW"),
        ui::gauge("Calibration", r.calibration_used, r.calibration_total, ""),
        ui::gauge(
            "Drone bandwidth",
            r.drone_bandwidth_used,
            r.drone_bandwidth_total,
            "Mbit/s"
        ),
        ui::gauge("Drone bay", r.drone_bay_used, r.drone_bay_total, "m3"),
        ui::stat_row(
            "Drones active",
            format!("{} / {}", r.drones_active, r.drones_max)
        ),
        ui::stat_row(
            "Hardpoints free",
            format!("{} turret, {} launcher", r.turrets_free, r.launchers_free)
        ),
        ui::gauge("Cargo", r.cargo_used, r.cargo_total, "m3"),
    ]
    .spacing(3)
    .into()
}

fn resist_cell<'a>(resonance: f64, color: Color) -> El<'a> {
    let resist = (1.0 - resonance).clamp(0.0, 1.0);
    column![
        ui::bar(resist as f32, color, 5.0),
        text(format!("{:.0}%", resist * 100.0))
            .size(ui::MICRO)
            .font(theme::UI_MEDIUM)
            .color(theme::TEXT_SECONDARY)
            .align_x(iced::alignment::Horizontal::Right)
            .width(Length::Fill),
    ]
    .spacing(2)
    .width(Length::Fill)
    .into()
}

fn layer_row<'a>(name: &'static str, l: &Layer) -> El<'a> {
    row![
        row![
            text(name)
                .size(ui::SMALL)
                .color(theme::TEXT_SECONDARY)
                .width(Length::Fill),
            text(fmt::short(l.hp)).size(ui::MICRO).color(theme::MUTED),
        ]
        .spacing(2)
        .width(72),
        resist_cell(l.em, theme::EM),
        resist_cell(l.thermal, theme::THERMAL),
        resist_cell(l.kinetic, theme::KINETIC),
        resist_cell(l.explosive, theme::EXPLOSIVE),
        text(fmt::short(l.ehp))
            .size(ui::SMALL)
            .font(theme::UI_MEDIUM)
            .color(theme::TEXT)
            .width(48)
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(ui::GAP)
    .align_y(iced::Alignment::Center)
    .into()
}

fn defense<'a>(app: &'a App, s: &'a Stats) -> El<'a> {
    let d = &s.defense;
    let current = app
        .docs
        .get(app.active)
        .and_then(|doc| DamagePreset::of(&doc.doc.fit.environment.damage_profile));
    let mut presets = row![ui::label("DAMAGE")]
        .spacing(3)
        .align_y(iced::Alignment::Center);
    for preset in DamagePreset::ALL {
        presets = presets.push(
            button(text(preset.label()).size(ui::MICRO))
                .padding([1, 6])
                .style(theme::chip(current == Some(preset)))
                .on_press(Message::DamageProfile(preset)),
        );
    }
    let header = row![
        space().width(72),
        ui::label("EM").width(Length::Fill),
        ui::label("TH").width(Length::Fill),
        ui::label("KIN").width(Length::Fill),
        ui::label("EXP").width(Length::Fill),
        ui::label("EHP")
            .width(48)
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(ui::GAP);
    let rep = |name: &'static str, raw: f64, effective: f64| {
        ui::stat_row(
            name,
            format!(
                "{} HP/s · {} EHP/s",
                fmt::num(raw, 1),
                fmt::num(effective, 1)
            ),
        )
    };
    column![
        presets.wrap(),
        header,
        column![
            layer_row("Shield", &d.shield),
            layer_row("Armor", &d.armor),
            layer_row("Hull", &d.hull),
        ]
        .spacing(6),
        ui::rule(),
        ui::stat_row("Shield recharge", fmt::seconds(d.shield_recharge_time)),
        rep(
            "Passive shield",
            d.passive_shield_rate,
            d.passive_shield_effective_rate
        ),
        rep(
            "Shield boost",
            d.shield_boost_rate,
            d.shield_effective_boost_rate
        ),
        rep(
            "Armor repair",
            d.armor_repair_rate,
            d.armor_effective_repair_rate
        ),
        rep(
            "Hull repair",
            d.hull_repair_rate,
            d.hull_effective_repair_rate
        ),
    ]
    .spacing(3)
    .into()
}

fn offense<'a>(app: &'a App, doc: &'a crate::Doc, s: &'a Stats) -> El<'a> {
    let o = &s.offense;
    let target = app.graph_target();
    let applied = app
        .ids
        .map(|ids| rufa_sim::graph::dps_at(&ids, &doc.doc.fit, &doc.doc.calc, 0.0, &target))
        .unwrap_or(0.0);
    let target_name = app
        .targets
        .get(app.target)
        .map(|p| p.name.as_str())
        .unwrap_or("target");
    column![
        ui::stat_row("Weapon DPS", fmt::num(o.dps, 1)),
        ui::stat_row("With reload", fmt::num(o.dps_with_reload, 1)),
        ui::stat_row("Volley", fmt::num(o.volley, 1)),
        ui::stat_row("Drone DPS", fmt::num(o.drone_dps, 1)),
        ui::stat_row("Fighter DPS", fmt::num(o.fighter_dps, 1)),
        ui::rule(),
        container(
            row![
                small_owned(format!("Applied vs {target_name}")).width(Length::Fill),
                ui::value(fmt::num(applied, 1))
            ]
            .align_y(iced::Alignment::Center)
        )
        .height(ui::STAT_ROW),
    ]
    .spacing(2)
    .into()
}

fn small_owned<'a>(s: String) -> text::Text<'a> {
    text(s).size(ui::SMALL).color(theme::TEXT_SECONDARY)
}

fn capacitor<'a>(doc: &'a crate::Doc, s: &'a Stats) -> El<'a> {
    let c = &s.capacitor;
    let sim = &doc.doc.cap;
    let (stability, color) = if c.stable_pct > 0.0 {
        (
            format!("Stable at {:.1}%", c.stable_pct),
            theme::SUCCESS_TEXT,
        )
    } else {
        (
            format!("Lasts {}", fmt::seconds(c.depletes_in)),
            theme::WARNING_TEXT,
        )
    };
    let simulated = match sim.depleted_at {
        Some(t) => format!("lasts {}", fmt::seconds(t)),
        None if c.capacity > 0.0 => format!("stable, low {:.0}%", sim.low / c.capacity * 100.0),
        None => "-".to_string(),
    };
    let level = if c.capacity > 0.0 {
        (sim.low / c.capacity) as f32
    } else {
        0.0
    };
    let bar_color = if sim.depleted_at.is_some() {
        theme::GAUGE_OVER
    } else {
        theme::ACCENT_LINE
    };
    column![
        ui::stat_row_colored("Stability", stability, color),
        ui::bar(
            level.max(if sim.depleted_at.is_some() { 0.0 } else { 0.02 }),
            bar_color,
            4.0
        ),
        ui::stat_row("Simulated", simulated),
        ui::stat_row("Capacity", format!("{} GJ", fmt::num(c.capacity, 1))),
        ui::stat_row("Recharge time", fmt::seconds(c.recharge_time)),
        ui::stat_row(
            "Peak recharge",
            format!("{} GJ/s", fmt::num(c.peak_recharge, 2))
        ),
        ui::stat_row("Peak load", format!("{} GJ/s", fmt::num(c.peak_load, 2))),
        ui::stat_row("Delta", format!("{} GJ/s", fmt::num(c.peak_delta, 2))),
    ]
    .spacing(2)
    .into()
}

fn navigation(s: &Stats) -> El<'_> {
    let n = &s.navigation;
    column![
        ui::stat_row(
            "Max velocity",
            format!("{} m/s", fmt::num(n.max_velocity, 1))
        ),
        ui::stat_row("Align time", fmt::seconds(n.align_time)),
        ui::stat_row(
            "Signature radius",
            format!("{} m", fmt::num(n.signature_radius, 1))
        ),
        ui::stat_row("Warp speed", format!("{} AU/s", fmt::num(n.warp_speed, 2))),
        ui::stat_row("Mass", format!("{} kg", fmt::short(n.mass))),
        ui::stat_row("Inertia modifier", fmt::num(n.agility, 3)),
    ]
    .spacing(2)
    .into()
}

fn targeting(s: &Stats) -> El<'_> {
    let t = &s.targeting;
    column![
        ui::stat_row("Max targets", format!("{}", t.max_targets)),
        ui::stat_row("Targeting range", fmt::distance(t.max_range)),
        ui::stat_row(
            "Scan resolution",
            format!("{} mm", fmt::num(t.scan_resolution, 0))
        ),
        ui::stat_row(
            "Sensor strength",
            format!("{} {}", fmt::num(t.sensor_strength, 1), t.sensor_type)
        ),
    ]
    .spacing(2)
    .into()
}

fn price<'a>(app: &'a App, doc: &'a crate::Doc) -> El<'a> {
    let fit = &doc.doc.fit;
    let isk = |v: Option<f64>| {
        v.map(|v| format!("{} ISK", fmt::short(v)))
            .unwrap_or_else(|| "-".to_string())
    };
    let hull = app.prices.get(fit.ship.type_id);
    let mut modules = 0.0;
    let mut drones = 0.0;
    let mut other = 0.0;
    for item in &fit.items {
        let price = app.prices.get(item.type_id).unwrap_or(0.0) * item.quantity as f64
            + item
                .charge
                .as_ref()
                .and_then(|c| app.prices.get(c.type_id))
                .unwrap_or(0.0);
        match item.slot {
            rufa_sim::Slot::DroneBay
            | rufa_sim::Slot::FighterTube(_)
            | rufa_sim::Slot::FighterBay => drones += price,
            slot if rufa_sim::place::Rack::of_slot(slot).is_some() => modules += price,
            _ => other += price,
        }
    }
    let status = if app.prices_pending {
        "fetching from Fuzzwork"
    } else {
        "Jita sell, cached a day"
    };
    column![
        ui::stat_row("Hull", isk(hull)),
        ui::stat_row("Modules and charges", isk(Some(modules))),
        ui::stat_row("Drones and fighters", isk(Some(drones))),
        ui::stat_row("Implants and cargo", isk(Some(other))),
        row![
            text(status)
                .size(ui::MICRO)
                .color(theme::MUTED)
                .width(Length::Fill),
            button(text("refresh").size(ui::MICRO))
                .padding([0, 5])
                .style(theme::ghost)
                .on_press(Message::RefreshPrices),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(2)
    .into()
}

fn item_attributes<'a>(app: &'a App, doc: &'a crate::Doc, i: usize) -> El<'a> {
    let Ok(game) = &app.game else {
        return space().into();
    };
    let item = &doc.doc.fit.items[i];
    let mut col = column![ui::title(app.type_name(item.type_id))].spacing(3);
    if let Some(ty) = game.sde.get_type(item.type_id) {
        let variants = game.catalog.variations(ty.market_group_id(), ty.group_id());
        if variants.len() > 1 {
            let mut line = row![ui::label("VARIANTS")]
                .spacing(3)
                .align_y(iced::Alignment::Center);
            for v in variants {
                line = line.push(
                    button(text(&v.name).size(ui::MICRO))
                        .padding([1, 5])
                        .style(theme::chip(v.id == item.type_id))
                        .on_press(Message::ReplaceItem(i, v.id)),
                );
            }
            col = col.push(line.wrap());
        }
    }
    if let Some(ids) = app.ids
        && rufa_sim::place::Rack::of_slot(item.slot).is_some()
    {
        let info = game.info();
        let groups = rufa_sim::place::charge_groups(&info, &ids, item.type_id);
        let mut charges = row![ui::label("CHARGES")]
            .spacing(3)
            .align_y(iced::Alignment::Center);
        let mut any = false;
        for group in groups {
            for charge in game.catalog.charges_in_group(group) {
                if !rufa_sim::place::accepts_charge(&info, &ids, item.type_id, charge.id) {
                    continue;
                }
                any = true;
                let loaded = item.charge.as_ref().is_some_and(|c| c.type_id == charge.id);
                charges = charges.push(
                    button(text(&charge.name).size(ui::MICRO))
                        .padding([1, 5])
                        .style(theme::chip(loaded))
                        .on_press(Message::LoadCharge(i, Some(charge.id))),
                );
            }
        }
        if any {
            charges = charges.push(
                button(text("none").size(ui::MICRO))
                    .padding([1, 5])
                    .style(theme::ghost)
                    .on_press(Message::LoadCharge(i, None)),
            );
            col = col.push(charges.wrap());
        }
    }
    if app.ids.is_some() {
        col = col.push(mutation_controls(app, game, doc, i));
        col = col.push(toggle_controls(game, doc, i));
        col = col.push(compare_variants(game, item.type_id));
    }
    let Some(result) = doc.doc.calc.items.get(i) else {
        return col.into();
    };
    let mut rows: Vec<(String, String, bool)> = Vec::new();
    for (id, value) in &result.attributes {
        let Some(label) = game.catalog.attr_label(*id) else {
            continue;
        };
        if !label.published || label.display_name.is_empty() {
            continue;
        }
        let shown = match label.unit.as_str() {
            "%" => format!("{}%", fmt::num(value.value, 2)),
            "ms" => fmt::seconds(value.value / 1000.0),
            "m" => fmt::distance(value.value),
            unit => format!("{} {unit}", fmt::num(value.value, 2))
                .trim_end()
                .to_string(),
        };
        let modified = (value.value - value.base).abs() > 1e-9;
        rows.push((label.display_name.clone(), shown, modified));
    }
    rows.sort();
    col = col.push(ui::rule());
    for (label, value, modified) in rows {
        let color = if modified { theme::ACCENT } else { theme::TEXT };
        col = col.push(
            row![
                text(label)
                    .size(ui::SMALL)
                    .color(theme::TEXT_SECONDARY)
                    .width(Length::Fill),
                text(value)
                    .size(ui::SMALL)
                    .font(theme::UI_MEDIUM)
                    .color(color),
            ]
            .align_y(iced::Alignment::Center),
        );
    }
    col.into()
}

fn mutation_controls<'a>(
    app: &'a App,
    game: &'static rufa_data::Game,
    doc: &'a crate::Doc,
    i: usize,
) -> El<'a> {
    use rufa_sim::edit;
    let item = &doc.doc.fit.items[i];
    let base = item
        .mutation
        .as_ref()
        .map(|m| m.base)
        .unwrap_or(item.type_id);
    let plasmids = edit::mutaplasmids_for(&game.sde, base);
    if plasmids.is_empty() {
        return column![].into();
    }
    let mut line = row![ui::label("MUTAPLASMID")]
        .spacing(3)
        .align_y(iced::Alignment::Center);
    for p in &plasmids {
        let name = app
            .type_name(*p)
            .split(' ')
            .next()
            .unwrap_or("")
            .to_string();
        line = line.push(
            button(text(name).size(ui::MICRO))
                .padding([1, 5])
                .style(theme::chip(false))
                .on_press(Message::Mutate(i, *p)),
        );
    }
    let mut col = column![line.wrap()].spacing(3);
    let Some(mutation) = &item.mutation else {
        return col.into();
    };
    col = col.push(
        button(text("Revert mutation").size(ui::MICRO))
            .padding([1, 6])
            .style(theme::danger)
            .on_press(Message::RevertMutation(i)),
    );
    let info = game.info();
    let plasmid = plasmids
        .iter()
        .copied()
        .find(|p| {
            let ranges = edit::mutation_ranges(&game.sde, &info, *p, base);
            mutation.attributes.iter().all(|(a, v)| {
                ranges
                    .iter()
                    .any(|r| r.attribute_id == *a && *v >= r.min - 1e-9 && *v <= r.max + 1e-9)
            })
        })
        .unwrap_or(plasmids[0]);
    for range in edit::mutation_ranges(&game.sde, &info, plasmid, base) {
        let label = game
            .catalog
            .attr_label(range.attribute_id)
            .map(|l| {
                if l.display_name.is_empty() {
                    l.name.clone()
                } else {
                    l.display_name.clone()
                }
            })
            .unwrap_or_else(|| range.attribute_id.to_string());
        let value = mutation
            .attributes
            .get(&range.attribute_id)
            .copied()
            .unwrap_or(range.base);
        let good = game
            .catalog
            .attr_label(range.attribute_id)
            .map(|l| l.high_is_good)
            .unwrap_or(true);
        let better = if good {
            value >= range.base
        } else {
            value <= range.base
        };
        let color = if better {
            theme::SUCCESS_TEXT
        } else {
            theme::DANGER_TEXT
        };
        let attribute = range.attribute_id;
        col = col.push(
            row![
                text(label)
                    .size(ui::MICRO)
                    .color(theme::TEXT_SECONDARY)
                    .width(Length::FillPortion(2)),
                slider(range.min..=range.max, value, move |v| {
                    Message::MutationValue(i, attribute, v)
                })
                .step((range.max - range.min) / 200.0)
                .style(theme::range)
                .width(Length::FillPortion(3)),
                text(fmt::num(value, 3))
                    .size(ui::MICRO)
                    .font(theme::UI_MEDIUM)
                    .color(color)
                    .width(60)
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(ui::GAP)
            .align_y(iced::Alignment::Center),
        );
    }
    col.into()
}

fn toggle_controls<'a>(game: &'static rufa_data::Game, doc: &'a crate::Doc, i: usize) -> El<'a> {
    use rufa_sim::edit;
    let item = &doc.doc.fit.items[i];
    let info = game.info();
    let mut col = column![].spacing(3);
    let side = edit::side_effects(&info, item.type_id);
    if !side.is_empty() {
        let mut line = row![ui::label("SIDE EFFECTS")]
            .spacing(3)
            .align_y(iced::Alignment::Center);
        for (effect, name) in side {
            let on = item.booster_side_effects.contains(&effect);
            let style: fn(&iced::Theme, button::Status) -> button::Style =
                if on { theme::danger } else { theme::secondary };
            line = line.push(
                button(text(name).size(ui::MICRO))
                    .padding([1, 5])
                    .style(style)
                    .on_press(Message::ToggleSideEffect(i, effect)),
            );
        }
        col = col.push(line.wrap());
    }
    if matches!(
        item.slot,
        rufa_sim::Slot::FighterTube(_) | rufa_sim::Slot::FighterBay
    ) {
        let mut line = row![ui::label("ABILITIES")]
            .spacing(3)
            .align_y(iced::Alignment::Center);
        for (effect, name, _) in edit::fighter_abilities(&info, item.type_id) {
            let on = edit::ability_on(&info, &doc.doc.fit, i, effect);
            line = line.push(
                button(text(name).size(ui::MICRO))
                    .padding([1, 5])
                    .style(theme::chip(on))
                    .on_press(Message::ToggleAbility(i, effect)),
            );
        }
        col = col.push(line.wrap());
    }
    col.into()
}

fn compare_variants<'a>(game: &'static rufa_data::Game, type_id: i32) -> El<'a> {
    let Some(ty) = game.sde.get_type(type_id) else {
        return column![].into();
    };
    let variants: Vec<_> = game
        .catalog
        .variations(ty.market_group_id(), ty.group_id())
        .iter()
        .take(6)
        .collect();
    if variants.len() < 2 {
        return column![].into();
    }
    let info = game.info();
    let values: Vec<std::collections::BTreeMap<i32, f64>> = variants
        .iter()
        .map(|v| {
            info.get_dogma_attributes(v.id)
                .into_iter()
                .flatten()
                .map(|a| (a.attribute_id(), a.value() as f64))
                .collect()
        })
        .collect();
    let mut differing: Vec<i32> = Vec::new();
    for map in &values {
        for id in map.keys() {
            let Some(label) = game.catalog.attr_label(*id) else {
                continue;
            };
            if !label.published || label.display_name.is_empty() || differing.contains(id) {
                continue;
            }
            let first = values[0].get(id).copied();
            if values.iter().any(|m| m.get(id).copied() != first) {
                differing.push(*id);
            }
        }
    }
    if differing.is_empty() {
        return column![].into();
    }
    let mut header = row![ui::label("COMPARE").width(Length::FillPortion(2))].spacing(2);
    for v in &variants {
        let short: String = v
            .name
            .split(' ')
            .map(|w| w.chars().take(4).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        header = header.push(
            text(short)
                .size(ui::MICRO)
                .color(theme::MUTED)
                .width(Length::FillPortion(1)),
        );
    }
    let mut table = column![header].spacing(1);
    for id in differing {
        let label = game
            .catalog
            .attr_label(id)
            .map(|l| l.display_name.clone())
            .unwrap_or_default();
        let mut line = row![
            text(label)
                .size(ui::MICRO)
                .color(theme::TEXT_SECONDARY)
                .width(Length::FillPortion(2))
        ]
        .spacing(2);
        for (n, map) in values.iter().enumerate() {
            let shown = map
                .get(&id)
                .map(|v| fmt::num(*v, 2))
                .unwrap_or_else(|| "-".to_string());
            let color = if variants[n].id == type_id {
                theme::ACCENT
            } else {
                theme::TEXT
            };
            line = line.push(
                text(shown)
                    .size(ui::MICRO)
                    .color(color)
                    .width(Length::FillPortion(1)),
            );
        }
        table = table.push(line);
    }
    table.into()
}
