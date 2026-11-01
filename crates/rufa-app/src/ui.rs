use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::widget::{
    button, canvas as canvas_widget, column, container, row, scrollable, space, text,
};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};
use rufa_sim::State;

use crate::Message;
use crate::theme;

pub const MICRO: f32 = 10.0;
pub const LABEL: f32 = 11.0;
pub const SMALL: f32 = 12.0;
pub const BODY: f32 = 13.0;
pub const VALUE: f32 = 14.0;
pub const TITLE: f32 = 16.0;

pub const ROW: f32 = 26.0;
pub const STAT_ROW: f32 = 22.0;
pub const LIST_ROW: f32 = 26.0;
pub const TOOLBAR: f32 = 44.0;
pub const STATUS: f32 = 26.0;

/// Every inset picks one of these steps. A call site that needs space names a
/// step; it does not invent a number.
pub const TIGHT: f32 = 2.0;
pub const GAP: f32 = 4.0;
pub const GAP_ROW: f32 = 6.0;
pub const PAD: f32 = 8.0;
pub const PAD_WIDE: f32 = 12.0;
pub const PAD_CARD: f32 = 24.0;

/// A pill's text sits on its cap height, and the line box keeps room for
/// descenders below the baseline. Symmetric padding therefore reads as two
/// pixels of extra space under the word, so the top carries one more.
pub fn pill(horizontal: f32) -> iced::Padding {
    iced::Padding {
        top: 3.0,
        right: horizontal,
        bottom: 2.0,
        left: horizontal,
    }
}

/// A row of options where exactly one is picked. Pass the label, whether it is
/// the current one, and the message that picks it.
pub fn segmented<'a>(options: Vec<(String, bool, Message)>) -> El<'a> {
    let mut line = row![].spacing(1).align_y(iced::Alignment::Center);
    for (label, on, msg) in options {
        line = line.push(
            button(text(label).size(MICRO))
                .padding(pill_tight(7.0))
                .style(theme::segment(on))
                .on_press(msg),
        );
    }
    container(line)
        .padding(1)
        .style(theme::segment_frame)
        .into()
}

/// The same control at the size a tab strip needs.
pub fn segmented_wide<'a>(options: Vec<(String, bool, Message)>) -> El<'a> {
    let mut line = row![].spacing(1).align_y(iced::Alignment::Center);
    for (label, on, msg) in options {
        line = line.push(
            button(text(label).size(BODY).font(theme::UI_MEDIUM))
                .padding(pill(8.0))
                .style(theme::segment(on))
                .on_press(msg),
        );
    }
    container(line)
        .padding(1)
        .style(theme::segment_frame)
        .into()
}

/// A text field carries the same line box as a pill, so its caps also sit
/// higher than the baseline gap below them.
pub fn field(vertical: f32, horizontal: f32) -> iced::Padding {
    iced::Padding {
        top: vertical + 1.0,
        right: horizontal,
        bottom: vertical - 1.0,
        left: horizontal,
    }
}

/// The same for a pill one step smaller, where the line box leaves even more
/// room under the baseline.
pub fn pill_tight(horizontal: f32) -> iced::Padding {
    iced::Padding {
        top: 2.0,
        right: horizontal,
        bottom: 0.0,
        left: horizontal,
    }
}

/// A pill with the air a dialog or a toolbar asks for, keeping the same
/// one-pixel lift off the baseline.
pub fn pill_tall(horizontal: f32) -> iced::Padding {
    iced::Padding {
        top: 5.0,
        right: horizontal,
        bottom: 4.0,
        left: horizontal,
    }
}

pub type El<'a> = Element<'a, Message>;

pub fn label<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s)
        .font(theme::LABEL)
        .size(LABEL)
        .color(theme::TEXT_SECONDARY)
        .wrapping(text::Wrapping::None)
}

pub fn eyebrow<'a>(s: &str) -> text::Text<'a> {
    text(s.to_uppercase())
        .font(theme::LABEL)
        .size(LABEL)
        .color(theme::TEXT_SECONDARY)
}

pub fn body<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s).font(theme::UI).size(BODY).color(theme::TEXT)
}

pub fn small<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s)
        .font(theme::UI)
        .size(SMALL)
        .color(theme::TEXT_SECONDARY)
        .wrapping(text::Wrapping::None)
}

pub fn muted<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s).font(theme::UI).size(BODY).color(theme::MUTED)
}

pub fn value<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s)
        .font(theme::UI_MEDIUM)
        .size(VALUE)
        .color(theme::TEXT)
        .wrapping(text::Wrapping::None)
}

pub fn value_colored<'a>(s: impl text::IntoFragment<'a>, color: Color) -> text::Text<'a> {
    text(s)
        .font(theme::UI_MEDIUM)
        .size(VALUE)
        .color(color)
        .wrapping(text::Wrapping::None)
}

pub fn name_cell<'a>(name: &'a str) -> El<'a> {
    container(small(name)).clip(true).width(Length::Fill).into()
}

pub fn title<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s).font(theme::UI_SEMI).size(TITLE).color(theme::TEXT)
}

pub fn scroll<'a>(content: impl Into<El<'a>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6)
                .scroller_width(6)
                .spacing(6),
        ))
        .style(theme::scroll)
}

pub fn scroll_x<'a>(content: impl Into<El<'a>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(content)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new()
                .width(5)
                .scroller_width(5)
                .spacing(3),
        ))
        .style(theme::scroll)
}

pub fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn rule<'a>() -> El<'a> {
    container(space().width(Length::Fill).height(1))
        .style(theme::fill(theme::HAIRLINE))
        .width(Length::Fill)
        .height(1)
        .into()
}

pub fn stat_row<'a>(narrow: bool, name: &'a str, shown: String) -> El<'a> {
    stat_row_colored(narrow, name, shown, theme::TEXT)
}

pub fn stat_row_colored<'a>(narrow: bool, name: &'a str, shown: String, color: Color) -> El<'a> {
    if narrow {
        return column![
            small(name),
            container(value_colored(shown, color))
                .clip(true)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(1)
        .into();
    }
    container(row![name_cell(name), value_colored(shown, color)].align_y(iced::Alignment::Center))
        .height(STAT_ROW)
        .into()
}

pub fn section<'a>(
    name: &'static str,
    headline: Option<String>,
    folded: bool,
    body: El<'a>,
) -> El<'a> {
    section_colored(name, headline, theme::TEXT, folded, body)
}

/// A section keeps its headline when folded, so a closed one still answers the
/// question it is named after.
pub fn section_colored<'a>(
    name: &'static str,
    headline: Option<String>,
    color: Color,
    folded: bool,
    body: El<'a>,
) -> El<'a> {
    let mut head = row![eyebrow(name).width(Length::Fill)].align_y(iced::Alignment::Center);
    if let Some(h) = headline {
        head = head.push(value_colored(h, color));
    }
    head = head.push(
        text(if folded { "\u{25b8}" } else { "\u{25be}" })
            .size(MICRO)
            .color(theme::MUTED),
    );
    let head = button(head.spacing(GAP_ROW))
        .padding(0)
        .width(Length::Fill)
        .style(theme::bare)
        .on_press(Message::ToggleStatSection(name));
    let inside: El<'a> = if folded {
        space().height(0).into()
    } else {
        body
    };
    container(column![head, inside].spacing(GAP))
        .padding([GAP_ROW, PAD])
        .width(Length::Fill)
        .style(theme::card)
        .into()
}

pub fn gauge_text_color(fraction: f32) -> Color {
    if fraction <= 1.0 {
        theme::TEXT
    } else if fraction <= 1.01 {
        theme::WARNING_TEXT
    } else {
        theme::DANGER_TEXT
    }
}

pub fn gauge_color(fraction: f32) -> Color {
    if fraction <= 1.0 {
        theme::GAUGE_OK
    } else if fraction <= 1.01 {
        theme::GAUGE_WARN
    } else if fraction <= 1.03 {
        theme::GAUGE_HOT
    } else {
        theme::GAUGE_OVER
    }
}

pub fn bar<'a>(fraction: f32, fill: Color, height: f32) -> El<'a> {
    let fraction = fraction.clamp(0.0, 1.0);
    let filled = (fraction * 1000.0).round() as u16;
    let rest = 1000 - filled;
    let mut line = row![];
    if filled > 0 {
        line = line.push(
            container(space().width(Length::Fill).height(height))
                .style(theme::rounded(fill, 2.0))
                .width(Length::FillPortion(filled))
                .height(height),
        );
    }
    if rest > 0 {
        line = line.push(space().width(Length::FillPortion(rest)).height(height));
    }
    container(line)
        .style(theme::rounded(theme::TRACK, 2.0))
        .width(Length::Fill)
        .height(height)
        .into()
}

pub fn split_bar<'a>(parts: [f64; 4], colors: [Color; 4], height: f32) -> El<'a> {
    let total: f64 = parts.iter().fold(0.0, |sum, part| sum + part);
    if total <= 0.0 {
        return container(space().width(Length::Fill).height(height))
            .style(theme::rounded(theme::TRACK, 2.0))
            .width(Length::Fill)
            .height(height)
            .into();
    }
    let mut line = row![];
    for (part, color) in parts.into_iter().zip(colors) {
        let share = ((part / total) * 1000.0).round() as u16;
        if share == 0 {
            continue;
        }
        line = line.push(
            container(space().width(Length::Fill).height(height))
                .style(theme::fill(color))
                .width(Length::FillPortion(share))
                .height(height),
        );
    }
    container(line)
        .style(theme::rounded(theme::TRACK, 2.0))
        .width(Length::Fill)
        .height(height)
        .into()
}

pub fn gauge<'a>(narrow: bool, name: &'a str, used: f64, total: f64, unit: &'static str) -> El<'a> {
    let fraction = if total > 0.0 {
        (used / total) as f32
    } else if used > 0.0 {
        2.0
    } else {
        0.0
    };
    let color = gauge_color(fraction);
    let text_color = gauge_text_color(fraction);
    let shown = format!(
        "{} / {} {unit}",
        crate::fmt::num(used, 1),
        crate::fmt::num(total, 1)
    );
    let percent = text(format!(
        "{}%",
        crate::fmt::num(f64::from(fraction) * 100.0, 0)
    ))
    .font(theme::UI_MEDIUM)
    .size(MICRO)
    .color(text_color)
    .wrapping(text::Wrapping::None);
    let named = container(
        row![small(name), percent]
            .spacing(GAP)
            .align_y(iced::Alignment::Center),
    )
    .clip(true);
    let head: El<'a> = if narrow {
        column![
            named.width(Length::Fill),
            container(value_colored(shown, text_color))
                .clip(true)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(1)
        .into()
    } else {
        row![named.width(Length::Fill), value_colored(shown, text_color)]
            .spacing(GAP)
            .align_y(iced::Alignment::Center)
            .into()
    };
    column![head, bar(fraction, color, 4.0)].spacing(2).into()
}

pub fn state_glyph<'a>(state: State) -> El<'a> {
    canvas_widget(StateGlyph { state })
        .width(14)
        .height(14)
        .into()
}

struct StateGlyph {
    state: State,
}

impl canvas::Program<Message> for StateGlyph {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        match self.state {
            State::Offline => {
                frame.stroke(
                    &Path::circle(c, 4.0),
                    Stroke::default().with_color(theme::OFFLINE).with_width(1.5),
                );
            }
            State::Online => {
                frame.fill(&Path::circle(c, 4.5), theme::ONLINE);
            }
            State::Active => {
                let tri = Path::new(|b| {
                    b.move_to(Point::new(c.x - 4.0, c.y - 5.0));
                    b.line_to(Point::new(c.x + 5.0, c.y));
                    b.line_to(Point::new(c.x - 4.0, c.y + 5.0));
                    b.close();
                });
                frame.fill(&tri, theme::ACTIVE);
            }
            State::Overload => {
                let tri = Path::new(|b| {
                    b.move_to(Point::new(c.x - 4.0, c.y - 5.0));
                    b.line_to(Point::new(c.x + 5.0, c.y));
                    b.line_to(Point::new(c.x - 4.0, c.y + 5.0));
                    b.close();
                });
                frame.fill(&tri, theme::OVERLOAD);
                frame.stroke(
                    &Path::circle(c, 6.5),
                    Stroke::default()
                        .with_color(theme::OVERLOAD)
                        .with_width(1.0),
                );
            }
        }
        vec![frame.into_geometry()]
    }
}
