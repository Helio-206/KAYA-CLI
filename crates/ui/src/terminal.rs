use crate::render::draw_frame;
use crate::UiState;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

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
                    state.record_submitted_input(&submitted);
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
                KeyCode::Up => {
                    state.history_previous();
                    Ok(None)
                }
                KeyCode::Down => {
                    state.history_next();
                    Ok(None)
                }
                KeyCode::PageUp => {
                    state.scroll_messages_up();
                    Ok(None)
                }
                KeyCode::PageDown => {
                    state.scroll_messages_down();
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
