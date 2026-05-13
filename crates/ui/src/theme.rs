use ratatui::style::{Color, Modifier, Style};

pub(crate) fn blue_style() -> Style {
    Style::default().fg(Color::Rgb(47, 95, 255))
}

pub(crate) fn cyan_style() -> Style {
    Style::default().fg(Color::Rgb(99, 221, 224))
}

pub(crate) fn label_style() -> Style {
    Style::default().fg(Color::Rgb(132, 146, 166))
}

pub(crate) fn value_style() -> Style {
    Style::default().fg(Color::Rgb(219, 226, 239))
}

pub(crate) fn muted_style() -> Style {
    Style::default().fg(Color::Rgb(91, 102, 122))
}

pub(crate) fn connected_style() -> Style {
    Style::default()
        .fg(Color::Rgb(99, 221, 224))
        .add_modifier(Modifier::BOLD)
}
