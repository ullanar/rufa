use std::sync::LazyLock;

use iced::border::Radius;
use iced::font::{Family, Stretch, Weight};
use iced::theme::Palette;
use iced::widget::{button, container, pick_list, scrollable, slider, text_input};
use iced::{Background, Border, Color, Font, Theme};

const fn rgb(hex: u32) -> Color {
    Color::from_rgb(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    )
}

pub const VOID: Color = rgb(0x0B0E13);
pub const SUNKEN: Color = rgb(0x10141B);
pub const BASE: Color = rgb(0x161B23);
pub const RAISED: Color = rgb(0x1E242E);
pub const OVERLAY: Color = rgb(0x262E3A);

pub const HAIRLINE: Color = rgb(0x232B36);
pub const EDGE: Color = rgb(0x323B49);
pub const STRONG: Color = rgb(0x44505F);

pub const TEXT: Color = rgb(0xE6EBF2);
pub const TEXT_SECONDARY: Color = rgb(0xA3AEBE);
pub const MUTED: Color = rgb(0x7E8A9B);
pub const FAINT: Color = rgb(0x5C6676);
pub const DISABLED: Color = rgb(0x464F5D);

pub const ACCENT: Color = rgb(0x4FC3E8);
pub const ACCENT_INK: Color = rgb(0x061018);
pub const ACCENT_LINE: Color = rgb(0x2E8FBC);
pub const ACCENT_WASH: Color = rgb(0x16384A);

pub const OFFLINE: Color = rgb(0x6B7585);
pub const ONLINE: Color = rgb(0x9AA7B8);
pub const ACTIVE: Color = rgb(0x8AE04A);
pub const OVERLOAD: Color = rgb(0xFF6A3D);

pub const EM: Color = rgb(0x2E86C7);
pub const THERMAL: Color = rgb(0xD0453F);
pub const KINETIC: Color = rgb(0x9AA3AE);
pub const EXPLOSIVE: Color = rgb(0xD9A23B);
pub const TRACK: Color = rgb(0x1C2230);

pub const DANGER_FILL: Color = rgb(0x2E1A1E);
pub const DANGER_LINE: Color = rgb(0x6B2F35);
pub const DANGER_TEXT: Color = rgb(0xFF8F8F);
pub const WARNING_TEXT: Color = rgb(0xF4C77A);
pub const SUCCESS_TEXT: Color = rgb(0x8FDFA5);

pub const GAUGE_OK: Color = rgb(0x7E8A9B);
pub const GAUGE_WARN: Color = rgb(0xF2B24A);
pub const GAUGE_HOT: Color = rgb(0xFF7A3D);
pub const GAUGE_OVER: Color = rgb(0xFF5C5C);

pub const RAIL_HIGH: Color = rgb(0x3D86C6);
pub const RAIL_MEDIUM: Color = rgb(0x2FA5A0);
pub const RAIL_LOW: Color = rgb(0x5BAF5F);
pub const RAIL_RIG: Color = rgb(0xC9963A);
pub const RAIL_SUBSYSTEM: Color = rgb(0x9C6FD6);
pub const RAIL_SERVICE: Color = rgb(0x8A93A3);

pub const SERIES: [Color; 6] = [
    rgb(0x56B4E9),
    rgb(0xE69F00),
    rgb(0x009E73),
    rgb(0xCC79A7),
    rgb(0xF0E442),
    rgb(0xD55E00),
];

const fn inter(weight: Weight) -> Font {
    Font {
        family: Family::Name("Inter"),
        weight,
        stretch: Stretch::Normal,
        style: iced::font::Style::Normal,
    }
}

pub const UI: Font = inter(Weight::Normal);
pub const UI_MEDIUM: Font = inter(Weight::Medium);
pub const UI_SEMI: Font = inter(Weight::Semibold);
pub const LABEL: Font = Font {
    family: Family::Name("IBM Plex Sans Condensed"),
    weight: Weight::Semibold,
    stretch: Stretch::Condensed,
    style: iced::font::Style::Normal,
};

pub const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../assets/fonts/IBMPlexSansCondensed-SemiBold.ttf"),
];

static RUFA: LazyLock<Theme> = LazyLock::new(|| {
    Theme::custom(
        "Rufa",
        Palette {
            background: BASE,
            text: TEXT,
            primary: ACCENT,
            success: SUCCESS_TEXT,
            warning: WARNING_TEXT,
            danger: DANGER_TEXT,
        },
    )
});

pub fn of(_state: &crate::App) -> Theme {
    RUFA.clone()
}

pub fn root(_state: &crate::App, _theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: VOID,
        text_color: TEXT,
    }
}

fn boxed(background: Color, line: Option<Color>, radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: line.unwrap_or(Color::TRANSPARENT),
            width: if line.is_some() { 1.0 } else { 0.0 },
            radius: Radius::new(radius),
        },
        snap: true,
        ..container::Style::default()
    }
}

pub fn pane(_theme: &Theme) -> container::Style {
    boxed(BASE, None, 6.0)
}

pub fn card(_theme: &Theme) -> container::Style {
    boxed(RAISED, Some(HAIRLINE), 6.0)
}

pub fn fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| boxed(color, None, 0.0)
}

pub fn rounded(color: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| boxed(color, None, radius)
}

pub fn row(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        if selected {
            boxed(ACCENT_WASH, None, 4.0)
        } else {
            container::Style::default()
        }
    }
}

pub fn banner(fill: Color, line: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| boxed(fill, Some(line), 4.0)
}

pub fn toolbar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BASE)),
        border: Border {
            color: HAIRLINE,
            width: 0.0,
            radius: Radius::new(0.0),
        },
        snap: true,
        ..container::Style::default()
    }
}

fn flat(
    background: Option<Color>,
    text_color: Color,
    line: Option<Color>,
    radius: f32,
) -> button::Style {
    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border {
            color: line.unwrap_or(Color::TRANSPARENT),
            width: if line.is_some() { 1.0 } else { 0.0 },
            radius: Radius::new(radius),
        },
        snap: true,
        ..button::Style::default()
    }
}

pub fn primary(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => flat(Some(rgb(0x6FD0EE)), ACCENT_INK, None, 4.0),
        button::Status::Pressed => flat(Some(ACCENT_LINE), ACCENT_INK, None, 4.0),
        button::Status::Disabled => flat(Some(RAISED), DISABLED, None, 4.0),
        button::Status::Active => flat(Some(ACCENT), ACCENT_INK, None, 4.0),
    }
}

pub fn secondary(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => flat(Some(OVERLAY), TEXT, Some(STRONG), 4.0),
        button::Status::Pressed => flat(Some(SUNKEN), TEXT, Some(EDGE), 4.0),
        button::Status::Disabled => flat(Some(RAISED), DISABLED, Some(HAIRLINE), 4.0),
        button::Status::Active => flat(Some(RAISED), TEXT_SECONDARY, Some(EDGE), 4.0),
    }
}

pub fn ghost(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => flat(Some(RAISED), TEXT, None, 4.0),
        button::Status::Pressed => flat(Some(SUNKEN), TEXT, None, 4.0),
        button::Status::Disabled => flat(None, DISABLED, None, 4.0),
        button::Status::Active => flat(None, MUTED, None, 4.0),
    }
}

pub fn danger(_theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => flat(Some(rgb(0x3D2227)), DANGER_TEXT, Some(DANGER_LINE), 4.0),
        _ => flat(Some(DANGER_FILL), DANGER_TEXT, Some(DANGER_LINE), 4.0),
    }
}

pub fn chip(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if selected {
            flat(Some(ACCENT_WASH), ACCENT, Some(ACCENT_LINE), 999.0)
        } else {
            match status {
                button::Status::Hovered => flat(Some(OVERLAY), TEXT, Some(STRONG), 999.0),
                _ => flat(Some(RAISED), TEXT_SECONDARY, Some(EDGE), 999.0),
            }
        }
    }
}

pub fn tab(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if active {
            flat(Some(BASE), TEXT, None, 4.0)
        } else {
            match status {
                button::Status::Hovered => flat(Some(RAISED), TEXT, None, 4.0),
                _ => flat(None, MUTED, None, 4.0),
            }
        }
    }
}

pub fn list_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        if selected {
            flat(Some(ACCENT_WASH), TEXT, None, 4.0)
        } else {
            match status {
                button::Status::Hovered => flat(Some(RAISED), TEXT, None, 4.0),
                _ => flat(None, TEXT_SECONDARY, None, 4.0),
            }
        }
    }
}

pub fn input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let line = match status {
        text_input::Status::Focused { .. } => ACCENT_LINE,
        text_input::Status::Hovered => STRONG,
        _ => EDGE,
    };
    text_input::Style {
        background: Background::Color(SUNKEN),
        border: Border {
            color: line,
            width: 1.0,
            radius: Radius::new(4.0),
        },
        icon: MUTED,
        placeholder: FAINT,
        value: TEXT,
        selection: ACCENT_WASH,
    }
}

pub fn picker(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let line = match status {
        pick_list::Status::Opened { .. } => ACCENT_LINE,
        pick_list::Status::Hovered => STRONG,
        _ => EDGE,
    };
    pick_list::Style {
        text_color: TEXT,
        placeholder_color: FAINT,
        handle_color: MUTED,
        background: Background::Color(SUNKEN),
        border: Border {
            color: line,
            width: 1.0,
            radius: Radius::new(4.0),
        },
    }
}

pub fn scroll(_theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let hovered = matches!(
        status,
        scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. }
    );
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(if hovered { STRONG } else { EDGE }),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: Radius::new(3.0),
            },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(OVERLAY),
            border: Border::default(),
            shadow: iced::Shadow::default(),
            icon: MUTED,
        },
    }
}

pub fn range(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle = match status {
        slider::Status::Dragged | slider::Status::Hovered => ACCENT,
        slider::Status::Active => TEXT_SECONDARY,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(ACCENT_LINE), Background::Color(TRACK)),
            width: 3.0,
            border: Border::default(),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: Background::Color(handle),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}
