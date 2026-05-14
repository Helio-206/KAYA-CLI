use crate::theme::{
    accent_style, blue_style, connected_style, cyan_style, danger_style, label_style, muted_style,
    panel_style, shell_style, success_style, title_style, value_style, warning_style,
};
use crate::{UiModal, UiPeer, UiRoom, UiState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap,
};
use ratatui::Frame;

pub(crate) fn draw_frame(frame: &mut Frame, state: &UiState) {
    let height = frame.area().height;
    frame.render_widget(Block::default().style(shell_style()), frame.area());

    let constraints = if state.show_logs {
        if height < 24 {
            vec![
                Constraint::Length(6),
                Constraint::Min(6),
                Constraint::Length(6),
                Constraint::Length(4),
            ]
        } else {
            vec![
                Constraint::Length(7),
                Constraint::Min(10),
                Constraint::Length(10),
                Constraint::Length(4),
            ]
        }
    } else {
        vec![
            Constraint::Length(if height < 20 { 6 } else { 7 }),
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(4),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    draw_header(frame, chunks[0], state);
    draw_social(frame, chunks[1], state);
    draw_network(frame, chunks[2], state);
    draw_input(frame, chunks[3], state);

    if state.show_splash {
        draw_splash(frame, frame.area());
    }
    if let Some(modal) = &state.modal {
        draw_modal(frame, frame.area(), modal);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, state: &UiState) {
    let security_style = if state.security_warnings > 0 {
        warning_style()
    } else {
        cyan_style()
    };
    let lines = vec![
        Line::from(vec![
            Span::styled("KAYA CLI", title_style()),
            Span::raw("   "),
            Span::styled("LAN-FIRST OPS CONSOLE", accent_style()),
            Span::raw("   "),
            Span::styled(
                format!(
                    "[{}]",
                    if state.status.eq_ignore_ascii_case("DEMO") {
                        "DEMO"
                    } else {
                        &state.status
                    }
                ),
                if state.status.eq_ignore_ascii_case("DEMO") {
                    warning_style()
                } else {
                    connected_style()
                },
            ),
            Span::raw(" "),
            Span::styled(format!("[{}]", state.presence), cyan_style()),
        ]),
        Line::from(vec![
            Span::styled("SPACE ", label_style()),
            Span::styled(&state.space, value_style()),
            Span::raw("   "),
            Span::styled("ROOM ", label_style()),
            Span::styled(format!("#{}", state.current_room), cyan_style()),
            Span::raw("   "),
            Span::styled("FINGERPRINT ", label_style()),
            Span::styled(&state.identity_fingerprint, security_style),
        ]),
        Line::from(vec![
            Span::styled("NODE ", label_style()),
            Span::styled(&state.node_id, value_style()),
            Span::raw("   "),
            Span::styled("CALLSIGN ", label_style()),
            Span::styled(&state.callsign, accent_style()),
            Span::raw("   "),
            Span::styled("SESSIONS ", label_style()),
            Span::styled(state.secure_sessions.to_string(), value_style()),
            Span::raw("   "),
            Span::styled("TRUSTED ", label_style()),
            Span::styled(state.trusted_peers.to_string(), success_style()),
            Span::raw("   "),
            Span::styled("WARN ", label_style()),
            Span::styled(state.security_warnings.to_string(), warning_style()),
        ]),
        Line::from(vec![
            Span::styled("INPUT ", label_style()),
            if state.input.is_empty() {
                Span::styled(
                    "/help  /who  /join semana-info  /secure-msg Ana teste",
                    muted_style(),
                )
            } else {
                Span::styled(state.input.as_str(), value_style())
            },
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).block(panel_block(" KAYA ")), area);
}

fn draw_social(frame: &mut Frame, area: Rect, state: &UiState) {
    if area.width < 90 {
        draw_messages(frame, area, state);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(32),
            Constraint::Length(30),
        ])
        .split(area);

    draw_rooms(frame, chunks[0], state);
    draw_messages(frame, chunks[1], state);
    draw_peers(frame, chunks[2], state);
}

fn draw_rooms(frame: &mut Frame, area: Rect, state: &UiState) {
    let items: Vec<ListItem> = state
        .rooms
        .iter()
        .map(|room| {
            let marker = if room.current {
                "LIVE"
            } else if room.joined {
                "JOINED"
            } else {
                "KNOWN"
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("[{marker}] "), room_style(room)),
                    Span::styled(format!("#{}", room.name), room_style(room)),
                ]),
                Line::from(vec![
                    Span::styled("members ", label_style()),
                    Span::styled(room.member_count.to_string(), muted_style()),
                ]),
            ])
        })
        .collect();
    frame.render_widget(List::new(items).block(panel_block(" ROOMS ")), area);
}

fn draw_messages(frame: &mut Frame, area: Rect, state: &UiState) {
    if state.messages.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("ROOM TRAFFIC WILL LAND HERE.", accent_style())),
                Line::from(Span::styled(
                    "Start with /join semana-info and then type a message without slash.",
                    muted_style(),
                )),
                Line::from(Span::styled(
                    "Good live checks: /who  /peers --fingerprints  /status",
                    muted_style(),
                )),
            ])
            .block(panel_block(" CHAT "))
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    let visible_height = area.height.saturating_sub(2) as usize;
    let bottom = state
        .messages
        .len()
        .saturating_sub(state.message_scroll.min(state.messages.len()));
    let start = bottom.saturating_sub(visible_height);
    let items: Vec<ListItem> = state.messages[start..bottom]
        .iter()
        .map(|message| {
            if message.direct && looks_like_decorated_demo_message(&message.body) {
                return ListItem::new(Line::from(vec![
                    Span::styled(short_time(&message.timestamp), muted_style()),
                    Span::raw(" "),
                    Span::styled(message.body.clone(), accent_style()),
                ]));
            }

            let (prefix, prefix_style, body_style) = if message.from == "system" {
                (
                    format!("{} [SYSTEM]", short_time(&message.timestamp)),
                    warning_style(),
                    muted_style(),
                )
            } else if message.direct {
                let target = message.target.as_deref().unwrap_or("me");
                let marker = if message.encrypted {
                    "[SECURE]"
                } else {
                    "[DM]"
                };
                (
                    format!(
                        "{} {} {} -> {}",
                        short_time(&message.timestamp),
                        marker,
                        message.from,
                        target
                    ),
                    if message.encrypted {
                        cyan_style()
                    } else {
                        accent_style()
                    },
                    value_style(),
                )
            } else if message.local {
                (
                    format!(
                        "{} [#{}] YOU",
                        short_time(&message.timestamp),
                        message.room.as_deref().unwrap_or("geral")
                    ),
                    success_style(),
                    value_style(),
                )
            } else {
                (
                    format!(
                        "{} [#{}] {}",
                        short_time(&message.timestamp),
                        message.room.as_deref().unwrap_or("geral"),
                        message.from
                    ),
                    value_style(),
                    value_style(),
                )
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::raw(": "),
                Span::styled(message.body.clone(), body_style),
            ]))
        })
        .collect();

    let title = if state.message_scroll == 0 {
        " CHAT "
    } else {
        " CHAT scroll "
    };
    frame.render_widget(List::new(items).block(panel_block(title)), area);
}

fn draw_peers(frame: &mut Frame, area: Rect, state: &UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let member_items: Vec<ListItem> = state
        .current_members
        .iter()
        .map(|member| {
            ListItem::new(Line::from(vec![
                Span::styled("[room] ", label_style()),
                Span::styled(member.clone(), value_style()),
            ]))
        })
        .collect();
    frame.render_widget(
        List::new(member_items).block(panel_block(" MEMBERS ")),
        chunks[0],
    );

    let peer_limit = (chunks[1].height.saturating_sub(2) as usize / 2).max(1);
    let peer_items: Vec<ListItem> = state
        .peers
        .iter()
        .take(peer_limit)
        .map(|peer| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(peer.callsign.clone(), peer_style(peer)),
                    Span::raw(" "),
                    Span::styled(format!("[{}]", peer.presence), muted_style()),
                    Span::raw(" "),
                    Span::styled(format!("[{}]", peer.trust_status), trust_style(peer)),
                ]),
                Line::from(vec![
                    Span::styled(peer.node_id.clone(), muted_style()),
                    Span::raw("  "),
                    Span::styled(
                        peer.fingerprint.clone().unwrap_or_else(|| "--".into()),
                        muted_style(),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        peer.latency_ms
                            .map(|value| format!("{value}ms"))
                            .unwrap_or_else(|| "--".into()),
                        label_style(),
                    ),
                ]),
            ])
        })
        .collect();
    frame.render_widget(
        List::new(peer_items).block(panel_block(" PEERS ")),
        chunks[1],
    );

    let visible_height = chunks[2].height.saturating_sub(2) as usize;
    let start = state.direct_messages.len().saturating_sub(visible_height);
    let dm_items: Vec<ListItem> = state.direct_messages[start..]
        .iter()
        .map(|message| {
            if looks_like_decorated_demo_message(&message.body) {
                return ListItem::new(Line::from(vec![
                    Span::styled(short_time(&message.timestamp), muted_style()),
                    Span::raw(" "),
                    Span::styled(message.body.clone(), accent_style()),
                ]));
            }
            let target = message.target.as_deref().unwrap_or("me");
            let marker = if message.encrypted {
                "[SECURE]"
            } else {
                "[DM]"
            };
            ListItem::new(Line::from(vec![
                Span::styled(short_time(&message.timestamp), muted_style()),
                Span::raw(" "),
                Span::styled(
                    marker,
                    if message.encrypted {
                        cyan_style()
                    } else {
                        accent_style()
                    },
                ),
                Span::raw(" "),
                Span::styled(format!("{} -> {target}", message.from), value_style()),
                Span::raw(": "),
                Span::styled(message.body.clone(), value_style()),
            ]))
        })
        .collect();
    frame.render_widget(List::new(dm_items).block(panel_block(" DMS ")), chunks[2]);

    let file_limit = (chunks[3].height.saturating_sub(2) as usize / 2).max(1);
    let file_items: Vec<ListItem> = state
        .files
        .iter()
        .take(file_limit)
        .map(|file| {
            let trust = if file.trusted { "trusted" } else { "unknown" };
            let signed = if file.signed { "signed" } else { "unsigned" };
            let hash_label = match file.hash_ok {
                Some(true) => "hash-ok",
                Some(false) => "hash-fail",
                None => "hash-pending",
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(file.file_name.clone(), value_style()),
                    Span::raw(" "),
                    Span::styled(format!("{:.0}%", file.percent), cyan_style()),
                ]),
                Line::from(vec![
                    Span::styled(file.peer.clone(), label_style()),
                    Span::raw("  "),
                    Span::styled(file.status.clone(), muted_style()),
                    Span::raw("  "),
                    Span::styled(file.security.clone(), muted_style()),
                    Span::raw("  "),
                    Span::styled(format!("{trust}/{signed}/{hash_label}"), muted_style()),
                ]),
            ])
        })
        .collect();
    frame.render_widget(
        List::new(file_items).block(panel_block(" FILES ")),
        chunks[3],
    );
}

fn draw_network(frame: &mut Frame, area: Rect, state: &UiState) {
    let online = state.peers.iter().filter(|peer| peer.online).count();
    let avg_latency = average_latency(&state.peers)
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "--".into());
    let mut lines = vec![
        Line::from(vec![
            Span::styled("LAN ", label_style()),
            Span::styled(format!("[{online} peers]"), success_style()),
            Span::raw("   "),
            Span::styled("LAT ", label_style()),
            Span::styled(avg_latency, value_style()),
            Span::raw("   "),
            Span::styled("PKTS ", label_style()),
            Span::styled(
                format!("{} / {}", state.packets_tx, state.packets_rx),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("UP ", label_style()),
            Span::styled(
                format_duration(state.diagnostics.uptime_secs),
                value_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("ROOM ", label_style()),
            Span::styled(format!("#{}", state.current_room), cyan_style()),
            Span::raw("   "),
            Span::styled("HEARTBEAT ", label_style()),
            Span::styled(
                format!("{}s", state.diagnostics.heartbeat_interval_secs),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("TIMEOUT ", label_style()),
            Span::styled(
                format!("{}s", state.diagnostics.peer_timeout_secs),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("RENDER ", label_style()),
            Span::styled(
                format!("{}ms", state.diagnostics.render_time_ms),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("LIMIT ", label_style()),
            Span::styled(
                format!("{}b", state.diagnostics.packet_max_bytes),
                value_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("BYTES ", label_style()),
            Span::styled(
                format!("{} / {}", state.bytes_tx, state.bytes_rx),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("EVENTS ", label_style()),
            Span::styled(
                format!(
                    "{} ({} /s)",
                    state.diagnostics.events_total, state.diagnostics.events_per_sec
                ),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("DROPS ", label_style()),
            Span::styled(
                format!(
                    "{} / {}",
                    state.diagnostics.duplicate_packets, state.diagnostics.malformed_packets
                ),
                value_style(),
            ),
            Span::raw("   "),
            Span::styled("MEM ", label_style()),
            Span::styled(format_memory(state.diagnostics.memory_kb), value_style()),
        ]),
        Line::from(peer_line(state)),
        Line::from(mesh_line(state)),
        Line::from(security_line(state)),
        Line::from(input_echo_line(state)),
    ];

    if !state.diagnostics.event_counters.is_empty() {
        lines.push(Line::from(event_counter_line(state)));
    }

    if state.show_logs {
        let log_height = area.height.saturating_sub(7) as usize;
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
            .block(panel_block(" NETWORK "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_input(frame: &mut Frame, area: Rect, state: &UiState) {
    frame.render_widget(panel_block(" INPUT "), area);

    let inner = Rect {
        x: area.x.saturating_add(2),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let command_text = if state.input.is_empty() {
        "> type here...".to_string()
    } else {
        format!("> {}", state.input)
    };

    let lines = if inner.height >= 2 {
        vec![
            Line::from(Span::styled(command_text, value_style())),
            Line::from(vec![
                Span::styled("hint ", label_style()),
                Span::styled(command_hint(state), muted_style()),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(command_text, value_style()))]
    };

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);

    let cursor_x = inner.x.saturating_add(2).saturating_add(
        state
            .input
            .chars()
            .count()
            .min(inner.width.saturating_sub(3) as usize) as u16,
    );
    let cursor_y = inner.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn draw_splash(frame: &mut Frame, area: Rect) {
    let area = centered_rect(area, 76, 15);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("KAYA CLI", title_style())),
            Line::from(Span::styled(
                "LOCAL-FIRST COMMUNICATION FOR TEMPORARY DIGITAL COMMUNITIES",
                accent_style(),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "No cloud. No server. Shared local space, rooms, DMs, files and mesh diagnostics.",
                value_style(),
            )),
            Line::from(Span::styled(
                "Quick start: /help  /who  /join semana-info  /msg Ana teste",
                muted_style(),
            )),
            Line::from(Span::styled(
                "Demo mode: /demo-peers 3  /demo-message semana-info 4  /demo-mesh-route",
                muted_style(),
            )),
            Line::from(Span::styled(
                "Inspect with: /status  /peers --fingerprints  /routes  /sessions",
                muted_style(),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled("Press any key to begin.", warning_style())),
        ])
        .block(panel_block(" START "))
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_modal(frame: &mut Frame, area: Rect, modal: &UiModal) {
    let area = centered_rect(area, 72, 8);
    frame.render_widget(Clear, area);
    let (title, lines): (&str, Vec<Line>) = match modal {
        UiModal::FileOffer {
            file_id,
            file_name,
            from_callsign,
            encrypted,
        } => (
            " FILE OFFER ",
            vec![
                Line::from(Span::styled(
                    format!("{} offers {}", from_callsign, file_name),
                    accent_style(),
                )),
                Line::from(Span::styled(
                    format!(
                        "id={} security={} accept=/accept-file {} reject=/reject-file {}",
                        file_id,
                        if *encrypted {
                            "encrypted"
                        } else {
                            "unencrypted"
                        },
                        file_id,
                        file_id
                    ),
                    muted_style(),
                )),
            ],
        ),
        UiModal::TrustWarning { node_id, message } => (
            " TRUST WARNING ",
            vec![
                Line::from(Span::styled(
                    node_id
                        .as_ref()
                        .map(|node| format!("peer={} {message}", node))
                        .unwrap_or_else(|| message.clone()),
                    warning_style(),
                )),
                Line::from(Span::styled(
                    "Review fingerprints and trust state before sending secure data.",
                    muted_style(),
                )),
            ],
        ),
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel_block(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn panel_block(title: &'static str) -> Block<'static> {
    Block::default()
        .title(Span::styled(title, title_style()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(blue_style())
        .style(panel_style())
        .padding(Padding::new(1, 1, 0, 0))
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
    let fp = peer.fingerprint.as_deref().unwrap_or("--");
    format!(
        "{}({},{status},{latency},{fp},{})",
        peer.callsign, peer.presence, peer.trust_status
    )
}

fn peer_style(peer: &UiPeer) -> ratatui::style::Style {
    if peer.online {
        accent_style()
    } else {
        muted_style()
    }
}

fn trust_style(peer: &UiPeer) -> ratatui::style::Style {
    match peer.trust_status.as_str() {
        "trusted" => success_style(),
        "blocked" => danger_style(),
        _ => muted_style(),
    }
}

fn room_style(room: &UiRoom) -> ratatui::style::Style {
    if room.current {
        accent_style()
    } else if room.joined {
        value_style()
    } else {
        muted_style()
    }
}

fn short_time(timestamp: &str) -> String {
    let Ok(ms) = timestamp.parse::<u64>() else {
        return "--:--".into();
    };
    let secs = (ms / 1000) % 86_400;
    format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
}

fn event_counter_line(state: &UiState) -> Vec<Span<'_>> {
    let mut spans = vec![Span::styled("event counters: ", label_style())];
    for (index, (kind, count)) in state.diagnostics.event_counters.iter().take(5).enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!("{kind}={count}"), accent_style()));
    }
    spans
}

fn mesh_line(state: &UiState) -> Vec<Span<'_>> {
    let enabled = if state.mesh.enabled { "yes" } else { "no" };
    let trace = if state.mesh.current_route_trace.is_empty() {
        state
            .mesh
            .last_route_discovered
            .as_deref()
            .unwrap_or("--")
            .to_string()
    } else {
        state.mesh.current_route_trace.join(" -> ")
    };
    vec![
        Span::styled("mesh: ", label_style()),
        Span::styled(
            enabled,
            if state.mesh.enabled {
                success_style()
            } else {
                muted_style()
            },
        ),
        Span::raw("    "),
        Span::styled("routes: ", label_style()),
        Span::styled(state.mesh.routes.to_string(), value_style()),
        Span::raw("    "),
        Span::styled("relayed/dropped: ", label_style()),
        Span::styled(
            format!(
                "{} / {}",
                state.mesh.relayed_packets, state.mesh.dropped_packets
            ),
            value_style(),
        ),
        Span::raw("    "),
        Span::styled("avg hops: ", label_style()),
        Span::styled(state.mesh.avg_hop_count.to_string(), value_style()),
        Span::raw("    "),
        Span::styled("trace: ", label_style()),
        Span::styled(trace, accent_style()),
    ]
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

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height.min(area.height)),
            Constraint::Fill(1),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width.min(area.width)),
            Constraint::Fill(1),
        ])
        .split(vertical[1])[1]
}

fn command_hint(state: &UiState) -> String {
    const HINTS: &[&str] = &[
        "/help",
        "/who",
        "/join semana-info",
        "/msg Ana teste",
        "/secure-msg Ana segredo",
        "/send Ana ./docs/PROTOCOL.md",
        "/demo-peers 3",
    ];
    let input = state.input.trim();
    if input.starts_with('/') {
        let filtered: Vec<_> = HINTS
            .iter()
            .copied()
            .filter(|hint| hint.starts_with(input))
            .take(3)
            .collect();
        if !filtered.is_empty() {
            return filtered.join("  |  ");
        }
    }
    HINTS[..4].join("  |  ")
}

fn looks_like_decorated_demo_message(body: &str) -> bool {
    body.starts_with("[SECURE][MESH:") || body.starts_with("[MESH:")
}

fn security_line(state: &UiState) -> Vec<Span<'_>> {
    vec![
        Span::styled("security: ", label_style()),
        Span::styled(&state.identity_fingerprint, cyan_style()),
        Span::raw("    "),
        Span::styled("trusted: ", label_style()),
        Span::styled(state.trusted_peers.to_string(), success_style()),
        Span::raw("    "),
        Span::styled("blocked: ", label_style()),
        Span::styled(state.blocked_peers.to_string(), danger_style()),
        Span::raw("    "),
        Span::styled("sessions: ", label_style()),
        Span::styled(state.secure_sessions.to_string(), accent_style()),
        Span::raw("    "),
        Span::styled("warnings: ", label_style()),
        Span::styled(state.security_warnings.to_string(), warning_style()),
    ]
}

fn input_echo_line(state: &UiState) -> Vec<Span<'_>> {
    vec![
        Span::styled("input: ", label_style()),
        if state.input.is_empty() {
            Span::styled("<empty>", muted_style())
        } else {
            Span::styled(state.input.as_str(), value_style())
        },
    ]
}
