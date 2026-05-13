use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;

const MAX_MESSAGES: usize = 200;
const MAX_LOGS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMessage {
    pub room: Option<String>,
    pub from: String,
    pub target: Option<String>,
    pub body: String,
    pub direct: bool,
    pub local: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPeer {
    pub node_id: String,
    pub callsign: String,
    pub online: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub space: String,
    pub node_id: String,
    pub callsign: String,
    pub status: String,
    pub current_room: String,
    pub input: String,
    pub messages: Vec<UiMessage>,
    pub peers: Vec<UiPeer>,
    pub logs: Vec<String>,
    pub show_logs: bool,
    pub packets_tx: u64,
    pub packets_rx: u64,
}

impl UiState {
    pub fn new(
        node_id: impl Into<String>,
        callsign: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        let room = room.into();
        Self {
            space: room.clone(),
            node_id: node_id.into(),
            callsign: callsign.into(),
            status: "CONNECTED".into(),
            current_room: room,
            input: String::new(),
            messages: Vec::new(),
            peers: Vec::new(),
            logs: Vec::new(),
            show_logs: true,
            packets_tx: 0,
            packets_rx: 0,
        }
    }

    pub fn push_message(&mut self, message: UiMessage) {
        self.messages.push(message);
        if self.messages.len() > MAX_MESSAGES {
            let overflow = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..overflow);
        }
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        self.logs.push(line.into());
        if self.logs.len() > MAX_LOGS {
            let overflow = self.logs.len() - MAX_LOGS;
            self.logs.drain(0..overflow);
        }
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }
}

pub struct TerminalUi {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalUi {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self { terminal })
    }

    pub fn draw(&mut self, state: &UiState) -> io::Result<()> {
        self.terminal.draw(|frame| draw_frame(frame, state))?;
        Ok(())
    }

    pub fn poll_input(
        &mut self,
        state: &mut UiState,
        timeout: Duration,
    ) -> io::Result<Option<String>> {
        if !event::poll(timeout)? {
            return Ok(None);
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Enter => {
                    let submitted = state.input.trim().to_string();
                    state.input.clear();
                    if submitted.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(submitted))
                    }
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Ok(Some("/exit".into()))
                }
                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Ok(Some("/clear".into()))
                }
                KeyCode::Char(ch) => {
                    state.input.push(ch);
                    Ok(None)
                }
                KeyCode::Backspace => {
                    state.input.pop();
                    Ok(None)
                }
                KeyCode::Esc => Ok(Some("/exit".into())),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }
}

impl Drop for TerminalUi {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn draw_frame(frame: &mut Frame, state: &UiState) {
    let constraints = if state.show_logs {
        vec![
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(8),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(5),
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

fn draw_header(frame: &mut Frame, area: ratatui::layout::Rect, state: &UiState) {
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

    let block = Block::default()
        .title(" KAYA ")
        .borders(Borders::ALL)
        .border_style(blue_style());
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_messages(frame: &mut Frame, area: ratatui::layout::Rect, state: &UiState) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = state.messages.len().saturating_sub(visible_height);
    let items: Vec<ListItem> = state.messages[start..]
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

    let block = Block::default()
        .title(" TRAFFIC ")
        .borders(Borders::ALL)
        .border_style(blue_style());
    frame.render_widget(List::new(items).block(block), area);
}

fn draw_network(frame: &mut Frame, area: ratatui::layout::Rect, state: &UiState) {
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
        ]),
        Line::from(peer_line(state)),
    ];

    if state.show_logs {
        let log_height = area.height.saturating_sub(4) as usize;
        let start = state.logs.len().saturating_sub(log_height);
        for log in &state.logs[start..] {
            lines.push(Line::from(vec![
                Span::styled("log: ", label_style()),
                Span::styled(log, muted_style()),
            ]));
        }
    }

    let block = Block::default()
        .title(" NETWORK ")
        .borders(Borders::ALL)
        .border_style(blue_style());
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_input(frame: &mut Frame, area: ratatui::layout::Rect, state: &UiState) {
    let block = Block::default()
        .title(" INPUT ")
        .borders(Borders::ALL)
        .border_style(blue_style());
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", cyan_style()),
        Span::raw(&state.input),
    ]))
    .block(block);
    frame.render_widget(input, area);
}

fn peer_line(state: &UiState) -> Vec<Span<'_>> {
    if state.peers.is_empty() {
        return vec![
            Span::styled("peers nearby: ", label_style()),
            Span::styled("none", muted_style()),
        ];
    }

    let mut spans = vec![Span::styled("peers nearby: ", label_style())];
    for (index, peer) in state.peers.iter().take(6).enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let status = if peer.online { "online" } else { "offline" };
        spans.push(Span::styled(
            format!("{}({})", peer.callsign, status),
            if peer.online {
                value_style()
            } else {
                muted_style()
            },
        ));
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

fn blue_style() -> Style {
    Style::default().fg(Color::Rgb(47, 95, 255))
}

fn cyan_style() -> Style {
    Style::default().fg(Color::Rgb(99, 221, 224))
}

fn label_style() -> Style {
    Style::default().fg(Color::Rgb(132, 146, 166))
}

fn value_style() -> Style {
    Style::default().fg(Color::Rgb(219, 226, 239))
}

fn muted_style() -> Style {
    Style::default().fg(Color::Rgb(91, 102, 122))
}

fn connected_style() -> Style {
    Style::default()
        .fg(Color::Rgb(99, 221, 224))
        .add_modifier(Modifier::BOLD)
}
