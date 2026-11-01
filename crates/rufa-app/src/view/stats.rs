use iced::widget::{button, column, container, row, scrollable, text};

use crate::DamagePreset;
use iced::{Element, Length};
use rufa_data::Info;
use rufa_sim::stats::{Layer, Stats};

use crate::fmt;
use crate::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(doc) = app.docs.get(app.active) else {
        return text("No fit open").style(text::secondary).into();
    };
    let s = &doc.doc.stats;
    let mut col = column![
        section("Resources", resources(s)),
        section("Defense", defense(app, s)),
        section("Offense", offense(app, doc, s)),
        section("Capacitor", capacitor(doc, s)),
        section("Navigation", navigation(s)),
        section("Targeting", targeting(s)),
        section("Price", price(app, doc)),
    ]
    .spacing(8);
    if let Some(item) = app.selected_item.filter(|i| *i < doc.doc.fit.items.len()) {
        col = col.push(section("Selected item", item_attributes(app, doc, item)));
    }
    scrollable(container(col).padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 0.0,
    }))
    .height(Length::Fill)
    .into()
}

fn section<'a>(title: &'static str, body: Element<'a, Message>) -> Element<'a, Message> {
    container(column![text(title).size(13), body].spacing(4))
        .padding(6)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn line<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .style(text::secondary)
            .width(Length::Fill),
        text(value).size(12),
    ]
    .into()
}

fn line_owned<'a>(label: String, value: String) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .style(text::secondary)
            .width(Length::Fill),
        text(value).size(12),
    ]
    .into()
}

fn price<'a>(app: &'a App, doc: &'a crate::Doc) -> Element<'a, Message> {
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
        line("Hull", isk(hull)),
        line("Modules and charges", isk(Some(modules))),
        line("Drones and fighters", isk(Some(drones))),
        line("Implants and cargo", isk(Some(other))),
        line("Total", isk(app.fit_price(doc))),
        row![
            text(status)
                .size(10)
                .style(text::secondary)
                .width(Length::Fill),
            button(text("refresh").size(10))
                .padding([0, 4])
                .style(button::text)
                .on_press(Message::RefreshPrices),
        ],
    ]
    .spacing(2)
    .into()
}

fn gauge<'a>(
    label: &'static str,
    used: f64,
    total: f64,
    unit: &'static str,
) -> Element<'a, Message> {
    let over = used > total + 1e-6;
    let value = format!("{} / {} {unit}", fmt::num(used, 1), fmt::num(total, 1));
    let style = if over { text::danger } else { text::base };
    row![
        text(label)
            .size(12)
            .style(text::secondary)
            .width(Length::Fill),
        text(value).size(12).style(style),
    ]
    .into()
}

fn resources(s: &Stats) -> Element<'_, Message> {
    let r = &s.resources;
    column![
        gauge("CPU", r.cpu_used, r.cpu_total, "tf"),
        gauge("Powergrid", r.power_used, r.power_total, "MW"),
        gauge("Calibration", r.calibration_used, r.calibration_total, ""),
        gauge(
            "Drone bandwidth",
            r.drone_bandwidth_used,
            r.drone_bandwidth_total,
            "Mbit/s"
        ),
        gauge("Drone bay", r.drone_bay_used, r.drone_bay_total, "m3"),
        line(
            "Drones active",
            format!("{} / {}", r.drones_active, r.drones_max)
        ),
        line("Turrets free", format!("{}", r.turrets_free)),
        line("Launchers free", format!("{}", r.launchers_free)),
        gauge("Cargo", r.cargo_used, r.cargo_total, "m3"),
    ]
    .spacing(2)
    .into()
}

fn layer_row<'a>(label: &'static str, l: &Layer) -> Element<'a, Message> {
    row![
        text(label).size(11).style(text::secondary).width(48),
        text(fmt::short(l.hp)).size(11).width(Length::Fill),
        text(fmt::resist(l.em)).size(11).width(Length::Fill),
        text(fmt::resist(l.thermal)).size(11).width(Length::Fill),
        text(fmt::resist(l.kinetic)).size(11).width(Length::Fill),
        text(fmt::resist(l.explosive)).size(11).width(Length::Fill),
        text(fmt::short(l.ehp)).size(11).width(Length::Fill),
    ]
    .into()
}

fn defense<'a>(app: &'a App, s: &'a Stats) -> Element<'a, Message> {
    let d = &s.defense;
    let current = app
        .docs
        .get(app.active)
        .and_then(|doc| DamagePreset::of(&doc.doc.fit.environment.damage_profile));
    let mut presets = row![text("Damage").size(11).style(text::secondary)]
        .spacing(3)
        .align_y(iced::Alignment::Center);
    for preset in DamagePreset::ALL {
        let style = if current == Some(preset) {
            button::primary
        } else {
            button::secondary
        };
        presets = presets.push(
            button(text(preset.label()).size(11))
                .padding([1, 5])
                .style(style)
                .on_press(Message::DamageProfile(preset)),
        );
    }
    let header = row![
        text("").width(48),
        text("HP")
            .size(11)
            .style(text::secondary)
            .width(Length::Fill),
        text("EM")
            .size(11)
            .style(text::secondary)
            .width(Length::Fill),
        text("Th")
            .size(11)
            .style(text::secondary)
            .width(Length::Fill),
        text("Kin")
            .size(11)
            .style(text::secondary)
            .width(Length::Fill),
        text("Exp")
            .size(11)
            .style(text::secondary)
            .width(Length::Fill),
        text("EHP")
            .size(11)
            .style(text::secondary)
            .width(Length::Fill),
    ];
    column![
        presets,
        line("Effective HP", fmt::num(d.ehp, 0)),
        header,
        layer_row("Shield", &d.shield),
        layer_row("Armor", &d.armor),
        layer_row("Hull", &d.hull),
        line("Shield recharge", fmt::seconds(d.shield_recharge_time)),
        line(
            "Passive shield",
            format!(
                "{} HP/s ({} EHP/s)",
                fmt::num(d.passive_shield_rate, 1),
                fmt::num(d.passive_shield_effective_rate, 1)
            )
        ),
        line(
            "Shield boost",
            format!(
                "{} HP/s ({} EHP/s)",
                fmt::num(d.shield_boost_rate, 1),
                fmt::num(d.shield_effective_boost_rate, 1)
            )
        ),
        line(
            "Armor repair",
            format!(
                "{} HP/s ({} EHP/s)",
                fmt::num(d.armor_repair_rate, 1),
                fmt::num(d.armor_effective_repair_rate, 1)
            )
        ),
        line(
            "Hull repair",
            format!(
                "{} HP/s ({} EHP/s)",
                fmt::num(d.hull_repair_rate, 1),
                fmt::num(d.hull_effective_repair_rate, 1)
            )
        ),
    ]
    .spacing(2)
    .into()
}

fn offense<'a>(app: &'a App, doc: &'a crate::Doc, s: &'a Stats) -> Element<'a, Message> {
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
        line("DPS", fmt::num(o.dps, 1)),
        line_owned(format!("DPS vs {target_name}"), fmt::num(applied, 1)),
        line("DPS with reload", fmt::num(o.dps_with_reload, 1)),
        line("Volley", fmt::num(o.volley, 1)),
        line("Drone DPS", fmt::num(o.drone_dps, 1)),
        line("Fighter DPS", fmt::num(o.fighter_dps, 1)),
    ]
    .spacing(2)
    .into()
}

fn capacitor<'a>(doc: &'a crate::Doc, s: &'a Stats) -> Element<'a, Message> {
    let c = &s.capacitor;
    let stability = if c.stable_pct > 0.0 {
        format!("Stable at {:.1}%", c.stable_pct)
    } else {
        format!("Lasts {}", fmt::seconds(c.depletes_in))
    };
    let sim = &doc.doc.cap;
    let simulated = match sim.depleted_at {
        Some(t) => format!("lasts {}", fmt::seconds(t)),
        None if c.capacity > 0.0 => format!("stable, low {:.0}%", sim.low / c.capacity * 100.0),
        None => "-".to_string(),
    };
    column![
        line("Capacity", format!("{} GJ", fmt::num(c.capacity, 1))),
        line("Recharge time", fmt::seconds(c.recharge_time)),
        line(
            "Peak recharge",
            format!("{} GJ/s", fmt::num(c.peak_recharge, 2))
        ),
        line("Peak load", format!("{} GJ/s", fmt::num(c.peak_load, 2))),
        line("Delta", format!("{} GJ/s", fmt::num(c.peak_delta, 2))),
        line("Stability", stability),
        line("Simulated", simulated),
    ]
    .spacing(2)
    .into()
}

fn navigation(s: &Stats) -> Element<'_, Message> {
    let n = &s.navigation;
    column![
        line(
            "Max velocity",
            format!("{} m/s", fmt::num(n.max_velocity, 1))
        ),
        line("Align time", fmt::seconds(n.align_time)),
        line(
            "Signature radius",
            format!("{} m", fmt::num(n.signature_radius, 1))
        ),
        line("Warp speed", format!("{} AU/s", fmt::num(n.warp_speed, 2))),
        line("Mass", format!("{} kg", fmt::short(n.mass))),
        line("Inertia modifier", fmt::num(n.agility, 3)),
    ]
    .spacing(2)
    .into()
}

fn targeting(s: &Stats) -> Element<'_, Message> {
    let t = &s.targeting;
    column![
        line("Max targets", format!("{}", t.max_targets)),
        line("Targeting range", fmt::distance(t.max_range)),
        line(
            "Scan resolution",
            format!("{} mm", fmt::num(t.scan_resolution, 0))
        ),
        line(
            "Sensor strength",
            format!("{} {}", fmt::num(t.sensor_strength, 1), t.sensor_type)
        ),
    ]
    .spacing(2)
    .into()
}

fn item_attributes<'a>(app: &'a App, doc: &'a crate::Doc, i: usize) -> Element<'a, Message> {
    let Ok(game) = &app.game else {
        return text("").into();
    };
    let item = &doc.doc.fit.items[i];
    let mut col = column![text(app.type_name(item.type_id)).size(12)].spacing(2);
    if let Some(ty) = game.sde.get_type(item.type_id) {
        let variants = game.catalog.variations(ty.market_group_id(), ty.group_id());
        if variants.len() > 1 {
            let mut line = row![text("Variants").size(11).style(text::secondary)].spacing(3);
            for v in variants {
                let style = if v.id == item.type_id {
                    button::primary
                } else {
                    button::secondary
                };
                line = line.push(
                    button(text(&v.name).size(10))
                        .padding([1, 4])
                        .style(style)
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
        let mut charges = row![text("Charges").size(11).style(text::secondary)].spacing(3);
        let mut any = false;
        for group in groups {
            for charge in game.catalog.charges_in_group(group) {
                if !rufa_sim::place::accepts_charge(&info, &ids, item.type_id, charge.id) {
                    continue;
                }
                any = true;
                let loaded = item.charge.as_ref().is_some_and(|c| c.type_id == charge.id);
                let style = if loaded {
                    button::primary
                } else {
                    button::secondary
                };
                charges = charges.push(
                    button(text(&charge.name).size(10))
                        .padding([1, 4])
                        .style(style)
                        .on_press(Message::LoadCharge(i, Some(charge.id))),
                );
            }
        }
        if any {
            charges = charges.push(
                button(text("none").size(10))
                    .padding([1, 4])
                    .style(button::text)
                    .on_press(Message::LoadCharge(i, None)),
            );
            col = col.push(charges.wrap());
        }
    }
    if app.ids.is_some() {
        col = col.push(mutation_controls(app, game, doc, i));
        col = col.push(toggle_controls(app, game, doc, i));
        col = col.push(compare_variants(game, item.type_id));
    }
    let Some(result) = doc.doc.calc.items.get(i) else {
        return col.into();
    };
    let mut rows: Vec<(String, String)> = Vec::new();
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
        let mark = if (value.value - value.base).abs() > 1e-9 {
            "*"
        } else {
            ""
        };
        rows.push((format!("{}{mark}", label.display_name), shown));
    }
    rows.sort();
    for (label, value) in rows {
        col = col.push(row![
            text(label)
                .size(11)
                .style(text::secondary)
                .width(Length::Fill),
            text(value).size(11),
        ]);
    }
    col.into()
}

fn mutation_controls<'a>(
    app: &'a App,
    game: &'static rufa_data::Game,
    doc: &'a crate::Doc,
    i: usize,
) -> Element<'a, Message> {
    use iced::widget::slider;
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
    let mut line = row![text("Mutaplasmid").size(11).style(text::secondary)].spacing(3);
    for p in &plasmids {
        let name = app
            .type_name(*p)
            .split(' ')
            .next()
            .unwrap_or("")
            .to_string();
        line = line.push(
            button(text(name).size(10))
                .padding([1, 4])
                .style(button::secondary)
                .on_press(Message::Mutate(i, *p)),
        );
    }
    let mut col = column![line.wrap()].spacing(2);
    let Some(mutation) = &item.mutation else {
        return col.into();
    };
    col = col.push(
        button(text("Revert mutation").size(10))
            .padding([1, 4])
            .style(button::danger)
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
        let style = if better { text::success } else { text::danger };
        let attribute = range.attribute_id;
        col = col.push(
            row![
                text(label)
                    .size(10)
                    .style(text::secondary)
                    .width(Length::FillPortion(2)),
                slider(range.min..=range.max, value, move |v| {
                    Message::MutationValue(i, attribute, v)
                })
                .step((range.max - range.min) / 200.0)
                .width(Length::FillPortion(3)),
                text(fmt::num(value, 3)).size(10).style(style).width(60),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        );
    }
    col.into()
}

fn toggle_controls<'a>(
    app: &'a App,
    game: &'static rufa_data::Game,
    doc: &'a crate::Doc,
    i: usize,
) -> Element<'a, Message> {
    use rufa_sim::edit;
    let _ = app;
    let item = &doc.doc.fit.items[i];
    let info = game.info();
    let mut col = column![].spacing(2);
    let side = edit::side_effects(&info, item.type_id);
    if !side.is_empty() {
        let mut line = row![text("Side effects").size(11).style(text::secondary)].spacing(3);
        for (effect, name) in side {
            let on = item.booster_side_effects.contains(&effect);
            let style = if on {
                button::danger
            } else {
                button::secondary
            };
            line = line.push(
                button(text(name).size(10))
                    .padding([1, 4])
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
        let mut line = row![text("Abilities").size(11).style(text::secondary)].spacing(3);
        for (effect, name, _) in edit::fighter_abilities(&info, item.type_id) {
            let on = edit::ability_on(&info, &doc.doc.fit, i, effect);
            let style = if on {
                button::primary
            } else {
                button::secondary
            };
            line = line.push(
                button(text(name).size(10))
                    .padding([1, 4])
                    .style(style)
                    .on_press(Message::ToggleAbility(i, effect)),
            );
        }
        col = col.push(line.wrap());
    }
    col.into()
}

fn compare_variants<'a>(game: &'static rufa_data::Game, type_id: i32) -> Element<'a, Message> {
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
    let mut header = row![
        text("Compare")
            .size(10)
            .style(text::secondary)
            .width(Length::FillPortion(2))
    ]
    .spacing(2);
    for v in &variants {
        let short: String = v
            .name
            .split(' ')
            .map(|w| w.chars().take(4).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        header = header.push(
            text(short)
                .size(9)
                .style(text::secondary)
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
                .size(10)
                .style(text::secondary)
                .width(Length::FillPortion(2))
        ]
        .spacing(2);
        for (n, map) in values.iter().enumerate() {
            let shown = map
                .get(&id)
                .map(|v| fmt::num(*v, 2))
                .unwrap_or_else(|| "-".to_string());
            let style = if variants[n].id == type_id {
                text::primary
            } else {
                text::base
            };
            line = line.push(
                text(shown)
                    .size(10)
                    .style(style)
                    .width(Length::FillPortion(1)),
            );
        }
        table = table.push(line);
    }
    table.into()
}
