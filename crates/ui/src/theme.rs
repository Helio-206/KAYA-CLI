use ratatui::style::{Color, Modifier, Style};

pub(crate) fn blue_style() -> Style {
    Style::default().fg(Color::Rgb(91, 140, 255))
}

pub(crate) fn cyan_style() -> Style {
    Style::default().fg(Color::Rgb(94, 234, 212))
}

pub(crate) fn label_style() -> Style {
    Style::default().fg(Color::Rgb(120, 141, 169))
}

pub(crate) fn value_style() -> Style {
    Style::default().fg(Color::Rgb(232, 238, 247))
}

pub(crate) fn muted_style() -> Style {
    Style::default().fg(Color::Rgb(88, 104, 130))
}

pub(crate) fn accent_style() -> Style {
    Style::default()
        .fg(Color::Rgb(167, 139, 250))
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn success_style() -> Style {
    Style::default()
        .fg(Color::Rgb(74, 222, 128))
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn warning_style() -> Style {
    Style::default()
        .fg(Color::Rgb(251, 191, 36))
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn danger_style() -> Style {
    Style::default()
        .fg(Color::Rgb(248, 113, 113))
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn shell_style() -> Style {
    Style::default()
        .bg(Color::Rgb(6, 10, 21))
        .fg(Color::Rgb(232, 238, 247))
}

pub(crate) fn panel_style() -> Style {
    Style::default()
        .bg(Color::Rgb(10, 16, 30))
        .fg(Color::Rgb(232, 238, 247))
}

pub(crate) fn title_style() -> Style {
    Style::default()
        .fg(Color::Rgb(125, 211, 252))
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn connected_style() -> Style {
    success_style()
}
