use kaya_shared::{normalize_room, KayaError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedInput {
    Empty,
    Message(String),
    Command(Command),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Who,
    Rooms,
    Join { room: String },
    Room { room: Option<String> },
    Msg { target: String, body: String },
    Status,
    Logs,
    Clear,
    Exit,
}

pub fn parse_input(input: &str) -> Result<ParsedInput> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(ParsedInput::Empty);
    }

    if !trimmed.starts_with('/') {
        return Ok(ParsedInput::Message(trimmed.to_string()));
    }

    let without_slash = &trimmed[1..];
    let mut parts = without_slash.split_whitespace();
    let Some(name) = parts.next() else {
        return Ok(ParsedInput::Empty);
    };

    let command = match name {
        "help" | "h" => Command::Help,
        "who" | "peers" => Command::Who,
        "rooms" => Command::Rooms,
        "join" | "j" => {
            let room = parts
                .next()
                .ok_or_else(|| KayaError::InvalidCommand("usage: /join <room>".to_string()))?;
            Command::Join {
                room: normalize_room(room),
            }
        }
        "room" => Command::Room {
            room: parts.next().map(normalize_room),
        },
        "msg" | "dm" => parse_msg_command(without_slash)?,
        "status" => Command::Status,
        "logs" => Command::Logs,
        "clear" => Command::Clear,
        "exit" | "quit" | "q" => Command::Exit,
        unknown => {
            return Err(KayaError::InvalidCommand(format!(
                "unknown command /{unknown}; try /help"
            )))
        }
    };

    Ok(ParsedInput::Command(command))
}

fn parse_msg_command(without_slash: &str) -> Result<Command> {
    let rest = without_slash
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or_default();
    let (target, body) = rest.split_once(char::is_whitespace).ok_or_else(|| {
        KayaError::InvalidCommand("usage: /msg <callsign|node-id> <message>".into())
    })?;

    let target = target.trim();
    let body = body.trim();
    if target.is_empty() || body.is_empty() {
        return Err(KayaError::InvalidCommand(
            "usage: /msg <callsign|node-id> <message>".into(),
        ));
    }

    Ok(Command::Msg {
        target: target.to_string(),
        body: body.to_string(),
    })
}

pub fn help_text() -> &'static str {
    "/help /who /rooms /join <room> /room [room] /msg <peer> <text> /status /logs /clear /exit"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_messages() {
        assert_eq!(
            parse_input("  alguem recebe? ").unwrap(),
            ParsedInput::Message("alguem recebe?".into())
        );
    }

    #[test]
    fn parses_join_and_normalizes_room() {
        assert_eq!(
            parse_input("/join #Semana-Info").unwrap(),
            ParsedInput::Command(Command::Join {
                room: "semana-info".into()
            })
        );
    }

    #[test]
    fn parses_direct_message_with_spaces() {
        assert_eq!(
            parse_input("/msg Ana teste privado agora").unwrap(),
            ParsedInput::Command(Command::Msg {
                target: "Ana".into(),
                body: "teste privado agora".into()
            })
        );
    }

    #[test]
    fn rejects_missing_message_body() {
        assert!(parse_input("/msg Ana").is_err());
    }

    #[test]
    fn parses_exit_alias() {
        assert_eq!(
            parse_input("/q").unwrap(),
            ParsedInput::Command(Command::Exit)
        );
    }
}
