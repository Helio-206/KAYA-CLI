use crate::theme::{
    blue_style, connected_style, cyan_style, label_style, muted_style, value_style,
};
use crate::{UiPeer, UiState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub(crate) fn draw_frame(frame: &mut Frame, state: &UiState) {
    let height = frame.area().height;
    let constraints = if state.show_logs {
        if height < 24 {
            vec![
                Constraint::Length(5),
                Constraint::Min(6),
                Constraint::Length(7),
                Constraint::Length(3),
            ]
        } else {
            vec![
                Constraint::Length(6),
                Constraint::Min(10),
                Constraint::Length(10),
                Constraint::Length(3),
            ]
        }
    } else {
        vec![
            Constraint::Length(if height < 20 { 5 } else { 6 }),
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(3),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    draw_header(frame, chunks[0], state);
    draw_messages(frame, chunks[1], state);
    draw_network(frame, chunks[2], state);
    draw_input(frame, chunks[3], state);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &UiState) {
    let lines = vec![
        Line::from(vec![
            Span::styled("SPACE: ", label_style()),
            Span::styled(&state.space, value_style()),
            Span::raw("    "),
            Span::styled("ROOM: ", label_style()),
            Span::styled(format!("#{}", state.current_room), cyan_style()),
        ]),
        Line::from(vec![
            Span::styled("NODE: ", label_style()),
            Span::styled(&state.node_id, value_style()),
            Span::raw("    "),
            Span::styled("CALLSIGN: ", label_style()),
            Span::styled(&state.callsign, value_style()),
        ]),
        Line::from(vec![
            Span::styled("STATUS: ", label_style()),
            Span::styled(&state.status, connected_style()),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).block(kaya_block(" KAYA ")), area);
}

fn draw_messages(frame: &mut Frame, area: Rect, state: &UiState) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let bottom = state
        .messages
        .len()
        .saturating_sub(state.message_scroll.min(state.messages.len()));
    let start = bottom.saturating_sub(visible_height);
    let items: Vec<ListItem> = state.messages[start..bottom]
        .iter()
        .map(|message| {
            let prefix = if message.direct {
                let target = message.target.as_deref().unwrap_or("me");
                format!("[DM] {} -> {}: ", message.from, target)
            } else {
                format!(
                    "[#{}] {}: ",
                    message.room.as_deref().unwrap_or("geral"),
                    message.from
                )
            };
            let style = if message.direct {
                cyan_style()
            } else {
                value_style()
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::raw(message.body.clone()),
            ]))
        })
        .collect();

    let title = if state.message_scroll == 0 {
        " TRAFFIC "
    } else {
        " TRAFFIC scroll "
    };
    frame.render_widget(List::new(items).block(kaya_block(title)), area);
}

fn draw_network(frame: &mut Frame, area: Rect, state: &UiState) {
    let online = state.peers.iter().filter(|peer| peer.online).count();
    let avg_latency = average_latency(&state.peers)
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "--".into());
    let mut lines = vec![
        Line::from(vec![
            Span::styled("peers: ", label_style()),
            Span::styled(online.to_string(), value_style()),
            Span::raw("    "),
            Span::styled("latency avg: ", label_style()),
            Span::styled(avg_latency, value_style()),
            Span::raw("    "),
            Span::styled("packets tx/rx: ", label_style()),
            Span::styled(
                format!("{} / {}", state.packets_tx, state.packets_rx),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("uptime: ", label_style()),
            Span::styled(
                format_duration(state.diagnostics.uptime_secs),
                value_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("room: ", label_style()),
            Span::styled(format!("#{}", state.current_room), cyan_style()),
            Span::raw("    "),
            Span::styled("heartbeat: ", label_style()),
            Span::styled(
                format!("{}s", state.diagnostics.heartbeat_interval_secs),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("timeout: ", label_style()),
            Span::styled(
                format!("{}s", state.diagnostics.peer_timeout_secs),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("render: ", label_style()),
            Span::styled(
                format!("{}ms", state.diagnostics.render_time_ms),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("limit: ", label_style()),
            Span::styled(
                format!("{}b", state.diagnostics.packet_max_bytes),
                value_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("bytes tx/rx: ", label_style()),
            Span::styled(
                format!("{} / {}", state.bytes_tx, state.bytes_rx),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("events: ", label_style()),
            Span::styled(
                format!(
                    "{} ({} /s)",
                    state.diagnostics.events_total, state.diagnostics.events_per_sec
                ),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("drops/malformed: ", label_style()),
            Span::styled(
                format!(
                    "{} / {}",
                    state.diagnostics.duplicate_packets, state.diagnostics.malformed_packets
                ),
                value_style(),
            ),
            Span::raw("    "),
            Span::styled("mem: ", label_style()),
            Span::styled(format_memory(state.diagnostics.memory_kb), value_style()),
        ]),
        Line::from(peer_line(state)),
    ];

    if !state.diagnostics.event_counters.is_empty() {
        lines.push(Line::from(event_counter_line(state)));
    }

    if state.show_logs {
        let log_height = area.height.saturating_sub(6) as usize;
        let start = state.logs.len().saturating_sub(log_height);
        for log in &state.logs[start..] {
            lines.push(Line::from(vec![
                Span::styled("log: ", label_style()),
                Span::styled(log, muted_style()),
            ]));
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(kaya_block(" NETWORK "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_input(frame: &mut Frame, area: Rect, state: &UiState) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", cyan_style()),
        Span::raw(&state.input),
    ]))
    .block(kaya_block(" INPUT "));
    frame.render_widget(input, area);
}

fn kaya_block(title: &'static str) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(blue_style())
}

fn peer_line(state: &UiState) -> Vec<Span<'_>> {
    if state.peers.is_empty() {
        return vec![
            Span::styled("peers nearby: ", label_style()),
            Span::styled("scanning LAN...", muted_style()),
        ];
    }

    let mut spans = vec![Span::styled("peers nearby: ", label_style())];
    for (index, peer) in state.peers.iter().take(6).enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(peer_summary(peer), peer_style(peer)));
    }
    spans
}

fn peer_summary(peer: &UiPeer) -> String {
    let status = if peer.online { "online" } else { "offline" };
    let latency = peer
        .latency_ms
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "--".into());
    format!("{}({status},{latency})", peer.callsign)
}

fn peer_style(peer: &UiPeer) -> ratatui::style::Style {
    if peer.online {
        value_style()
    } else {
        muted_style()
    }
}

fn event_counter_line(state: &UiState) -> Vec<Span<'_>> {
    let mut spans = vec![Span::styled("event counters: ", label_style())];
    for (index, (kind, count)) in state.diagnostics.event_counters.iter().take(5).enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!("{kind}={count}"), muted_style()));
    }
    spans
}

fn average_latency(peers: &[UiPeer]) -> Option<u64> {
    let values: Vec<u64> = peers.iter().filter_map(|peer| peer.latency_ms).collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<u64>() / values.len() as u64)
    }
}

fn format_duration(secs: u64) -> String {
    let minutes = secs / 60;
    let seconds = secs % 60;
    if minutes == 0 {
        format!("{seconds}s")
    } else {
        format!("{minutes}m{seconds:02}s")
    }
}

fn format_memory(memory_kb: Option<u64>) -> String {
    memory_kb
        .map(|value| format!("{value}kb"))
        .unwrap_or_else(|| "--".into())
}
