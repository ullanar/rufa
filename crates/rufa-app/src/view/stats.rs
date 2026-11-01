use iced::widget::{button, column, container, responsive, row, slider, space, text};
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
    responsive(move |size| stats_body(app, doc, size.width < 300.0)).into()
}

fn stats_body<'a>(app: &'a App, doc: &'a crate::Doc, narrow: bool) -> El<'a> {
    let s = &doc.doc.stats;
    let cap = &s.capacitor;
    let cap_head = if cap.stable_pct > 0.0 {
        format!("{:.0}%", cap.stable_pct)
    } else {
        fmt::seconds(cap.depletes_in)
    };
    let mut col = column![
        ui::section(
            "Resources",
            None,
            app.stats_folded.contains("Resources"),
            resources(s, narrow),
        ),
        ui::section(
            "Defense",
            Some(fmt::num(s.defense.ehp, 0)),
            app.stats_folded.contains("Defense"),
            defense(app, s, narrow),
        ),
    ]
    .spacing(ui::SECTION_GAP);
    let guns = s.offense.dps + s.offense.drone_dps + s.offense.fighter_dps;
    if guns > 0.0 {
        col = col.push(ui::section(
            "Offense",
            Some(fmt::num(guns, 1)),
            app.stats_folded.contains("Offense"),
            offense(app, doc, s, narrow),
        ));
    }
    if s.remote.any() {
        let head = if s.remote.hp() > 0.0 {
            format!("{} HP/s", fmt::num(s.remote.hp(), 1))
        } else {
            format!("{} GJ/s", fmt::num(s.remote.capacitor, 1))
        };
        col = col.push(ui::section_colored(
            "Remote reps",
            Some(head),
            theme::SUCCESS_TEXT,
            app.stats_folded.contains("Remote reps"),
            remote(s, narrow),
        ));
    }
    if s.mining.any() {
        col = col.push(ui::section(
            "Mining",
            Some(format!("{} m3/s", fmt::num(s.mining.total(), 2))),
            app.stats_folded.contains("Mining"),
            mining(s, narrow),
        ));
    }
    col = col.extend([
        ui::section(
            "Capacitor",
            Some(cap_head),
            app.stats_folded.contains("Capacitor"),
            capacitor(doc, s, narrow),
        ),
        ui::section(
            "Navigation",
            Some(format!("{} m/s", fmt::num(s.navigation.max_velocity, 0))),
            app.stats_folded.contains("Navigation"),
            navigation(s, narrow),
        ),
        ui::section(
            "Targeting",
            Some(fmt::distance(s.targeting.max_range)),
            app.stats_folded.contains("Targeting"),
            targeting(s, narrow),
        ),
        ui::section(
            "Price",
            app.fit_price(doc)
                .map(|price| format!("{} ISK", fmt::short(price.hull))),
            app.stats_folded.contains("Price"),
            price(app, doc, narrow),
        ),
    ]);
    if let Some(item) = app.selected_item.filter(|i| *i < doc.doc.fit.items.len()) {
        let count = app.selected.len();
        let (name, headline) = if count > 1 {
            ("Selection", Some(format!("{count} items")))
        } else {
            ("Selected item", None)
        };
        col = col.push(ui::section(
            name,
            headline,
            app.stats_folded.contains(name),
            item_attributes(app, doc, item),
        ));
    }
    ui::scroll(container(col).padding(iced::Padding {
        top: 0.0,
        right: 10.0,
        bottom: 0.0,
        left: 0.0,
    }))
    .height(Length::Fill)
    .into()
}

fn resources(s: &Stats, narrow: bool) -> El<'_> {
    let r = &s.resources;
    let mut col = column![
        ui::gauge(narrow, "CPU", r.cpu_used, r.cpu_total, "tf"),
        ui::gauge(narrow, "Powergrid", r.power_used, r.power_total, "MW"),
        ui::gauge(
            narrow,
            "Calibration",
            r.calibration_used,
            r.calibration_total,
            ""
        ),
    ]
    .spacing(3);
    if r.drone_bay_total > 0.0 || r.drone_bay_used > 0.0 {
        col = col.push(ui::gauge(
            narrow,
            "Drone bandwidth",
            r.drone_bandwidth_used,
            r.drone_bandwidth_total,
            "Mbit/s",
        ));
        col = col.push(ui::gauge(
            narrow,
            "Drone bay",
            r.drone_bay_used,
            r.drone_bay_total,
            "m3",
        ));
        col = col.push(ui::stat_row(
            narrow,
            "Drones in space",
            format!("{} / {}", r.drones_active, r.drones_max),
        ));
    }
    col = col.push(ui::stat_row(
        narrow,
        "Hardpoints free",
        format!("{} turret, {} launcher", r.turrets_free, r.launchers_free),
    ));
    col = col.push(ui::gauge(
        narrow,
        "Cargo",
        r.cargo_used,
        r.cargo_total,
        "m3",
    ));
    if r.fighter_bay_total > 0.0 {
        col = col.push(ui::gauge(
            narrow,
            "Fighter bay",
            r.fighter_bay_used,
            r.fighter_bay_total,
            "m3",
        ));
        col = col.push(ui::stat_row(
            narrow,
            "Fighter tubes",
            format!("{} / {}", r.fighter_tubes_used, r.fighter_tubes_total),
        ));
    }
    for ((name, _), size) in rufa_sim::stats::HOLDS.iter().zip(r.holds) {
        if size <= 0.0 {
            continue;
        }
        col = col.push(ui::stat_row(
            narrow,
            name,
            format!("{} m3", fmt::num(size, 0)),
        ));
    }
    col.into()
}

fn resist_cell<'a>(resonance: f64, color: Color, narrow: bool) -> El<'a> {
    let resist = (1.0 - resonance).clamp(0.0, 1.0);
    if narrow {
        return ui::bar(resist as f32, color, 5.0);
    }
    column![
        ui::bar(resist as f32, color, 5.0),
        container(
            text(format!("{:.0}%", resist * 100.0))
                .size(ui::MICRO)
                .font(theme::UI_MEDIUM)
                .color(color)
                .wrapping(text::Wrapping::None)
        )
        .clip(true)
        .align_x(iced::alignment::Horizontal::Right)
        .width(Length::Fill),
    ]
    .spacing(2)
    .width(Length::Fill)
    .into()
}

fn layer_row<'a>(name: &'static str, l: &Layer, narrow: bool) -> El<'a> {
    row![
        row![
            text(name)
                .size(ui::SMALL)
                .color(theme::TEXT_SECONDARY)
                .wrapping(text::Wrapping::None)
                .width(Length::Fill),
            text(fmt::short(l.hp))
                .size(ui::MICRO)
                .color(theme::MUTED)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(2)
        .width(72),
        resist_cell(l.em, theme::EM, narrow),
        resist_cell(l.thermal, theme::THERMAL, narrow),
        resist_cell(l.kinetic, theme::KINETIC, narrow),
        resist_cell(l.explosive, theme::EXPLOSIVE, narrow),
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

fn defense<'a>(app: &'a App, s: &'a Stats, narrow: bool) -> El<'a> {
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
                .padding(ui::pill_tight(6.0))
                .style(theme::chip(current == Some(preset)))
                .on_press(Message::DamageProfile(preset)),
        );
    }
    let head = |name: &'static str, color: Color| {
        ui::label(name)
            .color(color)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
    };
    let header = row![
        space().width(72),
        head("EM", theme::EM),
        head("TH", theme::THERMAL),
        head("KI", theme::KINETIC),
        head("EX", theme::EXPLOSIVE),
        ui::label("EHP")
            .width(48)
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(ui::GAP);
    let rep = |name: &'static str, raw: f64, effective: f64| -> El<'a> {
        if raw <= 0.0 && effective <= 0.0 {
            return space().height(0).into();
        }
        let shown = if narrow {
            format!("{} EHP/s", fmt::num(effective, 1))
        } else {
            format!(
                "{} HP/s · {} EHP/s",
                fmt::num(raw, 1),
                fmt::num(effective, 1)
            )
        };
        ui::stat_row(narrow, name, shown)
    };
    let mut col = column![
        presets.wrap(),
        header,
        column![
            layer_row("Shield", &d.shield, narrow),
            layer_row("Armor", &d.armor, narrow),
            layer_row("Hull", &d.hull, narrow),
        ]
        .spacing(6),
        ui::rule(),
        ui::stat_row(
            narrow,
            "Shield recharge",
            fmt::seconds(d.shield_recharge_time)
        ),
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
    .spacing(3);
    let incoming = [
        ("Incoming shield", s.incoming.shield),
        ("Incoming armor", s.incoming.armor),
        ("Incoming hull", s.incoming.hull),
    ];
    for (name, value) in incoming {
        if value <= 0.0 {
            continue;
        }
        col = col.push(ui::stat_row_colored(
            narrow,
            name,
            format!("{} HP/s", fmt::num(value, 1)),
            theme::SUCCESS_TEXT,
        ));
    }
    if d.sustained.capped {
        let sustained = [
            ("Shield sustained", d.sustained.shield, d.shield_boost_rate),
            ("Armor sustained", d.sustained.armor, d.armor_repair_rate),
            ("Hull sustained", d.sustained.hull, d.hull_repair_rate),
        ];
        for (name, held, burst) in sustained {
            if burst <= 0.0 {
                continue;
            }
            col = col.push(ui::stat_row_colored(
                narrow,
                name,
                format!("{} HP/s", fmt::num(held, 1)),
                theme::WARNING_TEXT,
            ));
        }
    }
    if d.bombs.iter().any(|count| *count > 0.0) {
        let mut line = row![].spacing(ui::GAP_ROW);
        let names = ["Electron", "Scorch", "Concussion", "Shrapnel"];
        for ((name, count), color) in names.iter().zip(d.bombs).zip([
            theme::EM,
            theme::THERMAL,
            theme::KINETIC,
            theme::EXPLOSIVE,
        ]) {
            line = line.push(
                row![
                    container(space().width(6).height(6)).style(theme::rounded(color, 1.0)),
                    text(format!("{name} {}", fmt::num(count, 1)))
                        .size(ui::MICRO)
                        .color(theme::TEXT_SECONDARY)
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(3)
                .align_y(iced::Alignment::Center),
            );
        }
        col = col.push(ui::rule());
        col = col.push(column![ui::label("BOMBS TO KILL, COVERT OPS V"), line.wrap()].spacing(2));
    }
    col.into()
}

fn offense<'a>(app: &'a App, doc: &'a crate::Doc, s: &'a Stats, narrow: bool) -> El<'a> {
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
    let colors = [theme::EM, theme::THERMAL, theme::KINETIC, theme::EXPLOSIVE];
    let total: f64 = o.damage.iter().fold(0.0, |sum, part| sum + part);
    let mut legend = row![].spacing(ui::GAP_ROW);
    for ((name, part), color) in rufa_sim::stats::DAMAGE_NAMES
        .iter()
        .zip(o.damage)
        .zip(colors)
    {
        if part <= 0.0 {
            continue;
        }
        legend = legend.push(
            row![
                container(space().width(6).height(6)).style(theme::rounded(color, 1.0)),
                text(format!("{name} {:.0}%", part / total * 100.0))
                    .size(ui::MICRO)
                    .color(theme::TEXT_SECONDARY)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(3)
            .align_y(iced::Alignment::Center),
        );
    }
    let split: El<'a> = if total > 0.0 {
        container(column![ui::split_bar(o.damage, colors, 4.0), legend.wrap()].spacing(3))
            .padding(iced::Padding {
                top: 5.0,
                right: 0.0,
                bottom: 2.0,
                left: 0.0,
            })
            .into()
    } else {
        space().height(0).into()
    };
    let mut col = column![].spacing(2);
    if o.dps > 0.0 {
        let shown = if o.dps_spooled > o.dps + 0.05 {
            format!("{} - {}", fmt::num(o.dps, 1), fmt::num(o.dps_spooled, 1))
        } else {
            fmt::num(o.dps, 1)
        };
        col = col.push(ui::stat_row(narrow, "Weapon DPS", shown));
    }
    for (name, value) in [
        ("With reload", o.dps_with_reload),
        ("Volley", o.volley),
        ("Drone DPS", o.drone_dps),
        ("Fighter DPS", o.fighter_dps),
    ] {
        if value <= 0.0 {
            continue;
        }
        col = col.push(ui::stat_row(narrow, name, fmt::num(value, 1)));
    }
    col = col.push(split);
    col = col.push(ui::rule());
    col = col.push(
        container(
            row![
                small_owned(format!("Applied vs {target_name}")).width(Length::Fill),
                ui::value(fmt::num(applied, 1))
            ]
            .align_y(iced::Alignment::Center),
        )
        .height(ui::STAT_ROW),
    );
    col.into()
}

fn remote<'a>(s: &'a Stats, narrow: bool) -> El<'a> {
    let r = &s.remote;
    let hp = |value: f64| format!("{} HP/s", fmt::num(value, 1));
    let mut col = column![].spacing(3);
    if r.shield > 0.0 {
        col = col.push(ui::stat_row_colored(
            narrow,
            "Shield",
            hp(r.shield),
            theme::SUCCESS_TEXT,
        ));
    }
    if r.armor > 0.0 {
        let shown = if r.spools() {
            format!(
                "{} - {} HP/s",
                fmt::num(r.armor, 1),
                fmt::num(r.armor_spooled, 1)
            )
        } else {
            hp(r.armor)
        };
        col = col.push(ui::stat_row_colored(
            narrow,
            "Armor",
            shown,
            theme::SUCCESS_TEXT,
        ));
    }
    if r.hull > 0.0 {
        col = col.push(ui::stat_row_colored(
            narrow,
            "Hull",
            hp(r.hull),
            theme::SUCCESS_TEXT,
        ));
    }
    if r.capacitor > 0.0 {
        col = col.push(ui::stat_row_colored(
            narrow,
            "Capacitor",
            format!("{} GJ/s", fmt::num(r.capacitor, 1)),
            theme::SUCCESS_TEXT,
        ));
    }
    if r.range > 0.0 {
        col = col.push(ui::stat_row(narrow, "Range", fmt::distance(r.range)));
    }
    if r.drain > 0.0 {
        col = col.push(ui::stat_row(
            narrow,
            "Capacitor used",
            format!("{} GJ/s", fmt::num(r.drain, 1)),
        ));
    }
    col.into()
}

fn mining<'a>(s: &'a Stats, narrow: bool) -> El<'a> {
    let m = &s.mining;
    let rate = |value: f64| format!("{} m3/s", fmt::num(value, 2));
    let mut col = column![].spacing(3);
    if m.miner > 0.0 {
        col = col.push(ui::stat_row(narrow, "Miners", rate(m.miner)));
    }
    if m.drone > 0.0 {
        col = col.push(ui::stat_row(narrow, "Drones", rate(m.drone)));
    }
    col = col.push(ui::stat_row(
        narrow,
        "Per hour",
        format!("{} m3", fmt::num(m.per_hour(), 0)),
    ));
    if m.drain > 0.0 {
        col = col.push(ui::stat_row(
            narrow,
            "Capacitor used",
            format!("{} GJ/s", fmt::num(m.drain, 1)),
        ));
    }
    col.into()
}

fn lock_times<'a>(s: &'a Stats) -> El<'a> {
    let mut line = row![].spacing(ui::GAP_ROW);
    for ((name, _), seconds) in rufa_sim::stats::TARGET_CLASSES
        .iter()
        .zip(s.targeting.lock_times)
    {
        if seconds <= 0.0 {
            continue;
        }
        line = line.push(
            row![
                text(*name)
                    .size(ui::MICRO)
                    .color(theme::MUTED)
                    .wrapping(text::Wrapping::None),
                text(format!("{:.1}s", seconds))
                    .size(ui::MICRO)
                    .font(theme::UI_MEDIUM)
                    .color(theme::TEXT_SECONDARY)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(3),
        );
    }
    column![ui::label("LOCK TIME"), line.wrap()]
        .spacing(2)
        .into()
}

fn small_owned<'a>(s: String) -> text::Text<'a> {
    text(s).size(ui::SMALL).color(theme::TEXT_SECONDARY)
}

fn capacitor<'a>(doc: &'a crate::Doc, s: &'a Stats, narrow: bool) -> El<'a> {
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
        ui::stat_row_colored(narrow, "Stability", stability, color),
        ui::bar(
            if sim.depleted_at.is_some() {
                0.0
            } else {
                level.max(0.02)
            },
            bar_color,
            4.0
        ),
        ui::stat_row_colored(narrow, "Simulated", simulated, bar_color),
        ui::stat_row(
            narrow,
            "Capacity",
            format!("{} GJ", fmt::num(c.capacity, 1))
        ),
        ui::stat_row(narrow, "Recharge time", fmt::seconds(c.recharge_time)),
        ui::stat_row(
            narrow,
            "Peak recharge",
            format!("{} GJ/s", fmt::num(c.peak_recharge, 2))
        ),
        ui::stat_row(
            narrow,
            "Peak load",
            format!("{} GJ/s", fmt::num(c.peak_load, 2))
        ),
        ui::stat_row(
            narrow,
            "Delta",
            format!("{} GJ/s", fmt::num(c.peak_delta, 2))
        ),
    ]
    .spacing(2)
    .into()
}

fn navigation(s: &Stats, narrow: bool) -> El<'_> {
    let n = &s.navigation;
    let mut col = column![
        ui::stat_row(
            narrow,
            "Max velocity",
            format!("{} m/s", fmt::num(n.max_velocity, 1))
        ),
        ui::stat_row(narrow, "Align time", fmt::seconds(n.align_time)),
        ui::stat_row(
            narrow,
            "Signature radius",
            format!("{} m", fmt::num(n.signature_radius, 1))
        ),
        ui::stat_row(
            narrow,
            "Warp speed",
            format!("{} AU/s", fmt::num(n.warp_speed, 2))
        ),
        ui::stat_row(narrow, "Mass", format!("{} kg", fmt::short(n.mass))),
        ui::stat_row(narrow, "Inertia modifier", fmt::num(n.agility, 3)),
    ]
    .spacing(2);
    if n.max_warp_distance > 0.0 {
        col = col.push(ui::stat_row(
            narrow,
            "Max warp distance",
            format!("{} AU", fmt::num(n.max_warp_distance, 1)),
        ));
    }
    col.into()
}

fn targeting(s: &Stats, narrow: bool) -> El<'_> {
    let t = &s.targeting;
    let mut col = column![
        ui::stat_row(narrow, "Max targets", format!("{}", t.max_targets)),
        ui::stat_row(narrow, "Targeting range", fmt::distance(t.max_range)),
        ui::stat_row(
            narrow,
            "Scan resolution",
            format!("{} mm", fmt::num(t.scan_resolution, 0))
        ),
        ui::stat_row(
            narrow,
            "Sensor strength",
            format!("{} {}", fmt::num(t.sensor_strength, 1), t.sensor_type)
        ),
    ]
    .spacing(2);
    if t.drone_control_range > 0.0 {
        col = col.push(ui::stat_row(
            narrow,
            "Drone control range",
            fmt::distance(t.drone_control_range),
        ));
    }
    if t.jam_chance > 0.0 {
        col = col.push(ui::stat_row_colored(
            narrow,
            "Chance to be jammed",
            format!("{} %", fmt::num(t.jam_chance, 1)),
            theme::DANGER_TEXT,
        ));
    }
    if t.scan_resolution > 0.0 {
        col = col.push(ui::rule());
        col = col.push(lock_times(s));
    }
    col.into()
}

fn price<'a>(app: &'a App, doc: &'a crate::Doc, narrow: bool) -> El<'a> {
    let fit = &doc.doc.fit;
    let isk = |v: Option<f64>| {
        v.map(|v| format!("{} ISK", fmt::short(v)))
            .unwrap_or_else(|| "-".to_string())
    };
    let hull = app.prices.get(fit.ship.type_id);
    let mut modules = 0.0;
    let mut drones = 0.0;
    let mut head = 0.0;
    let mut cargo = 0.0;
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
            rufa_sim::Slot::Implant(_) | rufa_sim::Slot::Booster(_) => head += price,
            slot if rufa_sim::place::Rack::of_slot(slot).is_some() => modules += price,
            _ => cargo += price,
        }
    }
    let status = if app.prices_pending {
        "fetching from Fuzzwork"
    } else {
        "Jita sell, cached a day"
    };
    column![
        ui::stat_row(narrow, "Hull", isk(hull)),
        ui::stat_row(narrow, "Modules and charges", isk(Some(modules))),
        ui::stat_row(narrow, "Drones and fighters", isk(Some(drones))),
        ui::stat_row(narrow, "Cargo", isk(Some(cargo))),
        ui::rule(),
        ui::stat_row(narrow, "In the head", isk(Some(head))),
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
                        .padding(ui::pill_tight(5.0))
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
                        .padding(ui::pill_tight(5.0))
                        .style(theme::chip(loaded))
                        .on_press(Message::LoadCharge(i, Some(charge.id))),
                );
            }
        }
        if any {
            charges = charges.push(
                button(text("none").size(ui::MICRO))
                    .padding(ui::pill_tight(5.0))
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
                container(
                    text(label)
                        .size(ui::SMALL)
                        .color(theme::TEXT_SECONDARY)
                        .wrapping(text::Wrapping::None)
                )
                .clip(true)
                .width(Length::Fill),
                text(value)
                    .size(ui::SMALL)
                    .font(theme::UI_MEDIUM)
                    .color(color)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(ui::GAP)
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
                .padding(ui::pill_tight(5.0))
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
            .padding(ui::pill_tight(6.0))
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
                    .padding(ui::pill_tight(5.0))
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
                    .padding(ui::pill_tight(5.0))
                    .style(theme::chip(on))
                    .on_press(Message::ToggleAbility(i, effect)),
            );
        }
        col = col.push(line.wrap());
    }
    col.into()
}

fn compare_variants<'a>(game: &'static rufa_data::Game, type_id: i32) -> El<'a> {
    responsive(move |size| compare_table(game, type_id, size.width)).into()
}

fn compare_table<'a>(game: &'static rufa_data::Game, type_id: i32, width: f32) -> El<'a> {
    let Some(ty) = game.sde.get_type(type_id) else {
        return column![].into();
    };
    let label_width = (width * 0.42).clamp(60.0, 150.0);
    let columns = (((width - label_width) / 46.0) as usize).clamp(1, 5);
    let all = game.catalog.variations(ty.market_group_id(), ty.group_id());
    if all.len() < 2 {
        return column![].into();
    }
    let mut variants: Vec<&rufa_data::TypeRef> = all.iter().filter(|v| v.id == type_id).collect();
    variants.extend(all.iter().filter(|v| v.id != type_id));
    variants.truncate(columns);
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
                .map(|a| (a.attribute_id(), f64::from(a.value())))
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

    let names = short_names(&variants);
    let cell_width = ((width - label_width) / variants.len() as f32).max(28.0);
    let cell = |shown: String, color: Color| {
        container(
            text(shown)
                .size(ui::MICRO)
                .color(color)
                .wrapping(text::Wrapping::None)
                .align_x(iced::alignment::Horizontal::Right)
                .width(Length::Fill),
        )
        .clip(true)
        .width(cell_width)
    };
    let mut header = row![
        container(ui::label("COMPARE"))
            .clip(true)
            .width(label_width)
    ];
    for (n, name) in names.iter().enumerate() {
        let color = if variants[n].id == type_id {
            theme::ACCENT
        } else {
            theme::MUTED
        };
        header = header.push(cell(name.clone(), color));
    }

    let mut table = column![header].spacing(1);
    for id in differing {
        let label = game
            .catalog
            .attr_label(id)
            .map(|l| l.display_name.clone())
            .unwrap_or_default();
        let mut line = row![
            container(
                text(label)
                    .size(ui::MICRO)
                    .color(theme::TEXT_SECONDARY)
                    .wrapping(text::Wrapping::None)
            )
            .clip(true)
            .width(label_width)
        ];
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
            line = line.push(cell(shown, color));
        }
        table = table.push(line);
    }
    table.into()
}

fn short_names(variants: &[&rufa_data::TypeRef]) -> Vec<String> {
    let words: Vec<Vec<&str>> = variants
        .iter()
        .map(|v| v.name.split_whitespace().collect())
        .collect();
    let shared: Vec<&str> = words[0]
        .iter()
        .copied()
        .filter(|w| words.iter().all(|other| other.contains(w)))
        .collect();
    words
        .iter()
        .enumerate()
        .map(|(n, parts)| {
            let kept: Vec<&str> = parts
                .iter()
                .copied()
                .filter(|w| !shared.contains(w))
                .collect();
            let name = if kept.is_empty() {
                variants[n].name.clone()
            } else {
                kept.join(" ")
            };
            ui::clip(&name, 9)
        })
        .collect()
}
