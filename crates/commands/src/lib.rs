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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Help,
    Who,
    Rooms,
    Join,
    Room,
    Msg,
    Status,
    Logs,
    Clear,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub kind: CommandKind,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub usage: &'static str,
    pub description: &'static str,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        kind: CommandKind::Help,
        name: "help",
        aliases: &["h"],
        usage: "/help",
        description: "show command summary",
    },
    CommandSpec {
        kind: CommandKind::Who,
        name: "who",
        aliases: &["peers"],
        usage: "/who",
        description: "list discovered peers",
    },
    CommandSpec {
        kind: CommandKind::Rooms,
        name: "rooms",
        aliases: &[],
        usage: "/rooms",
        description: "list known rooms",
    },
    CommandSpec {
        kind: CommandKind::Join,
        name: "join",
        aliases: &["j"],
        usage: "/join <room>",
        description: "join or create a room",
    },
    CommandSpec {
        kind: CommandKind::Room,
        name: "room",
        aliases: &[],
        usage: "/room [room]",
        description: "show or switch current room",
    },
    CommandSpec {
        kind: CommandKind::Msg,
        name: "msg",
        aliases: &["dm"],
        usage: "/msg <callsign|node-id> <message>",
        description: "send a direct message",
    },
    CommandSpec {
        kind: CommandKind::Status,
        name: "status",
        aliases: &[],
        usage: "/status",
        description: "show runtime diagnostics",
    },
    CommandSpec {
        kind: CommandKind::Logs,
        name: "logs",
        aliases: &[],
        usage: "/logs",
        description: "toggle technical logs",
    },
    CommandSpec {
        kind: CommandKind::Clear,
        name: "clear",
        aliases: &[],
        usage: "/clear",
        description: "clear visible traffic",
    },
    CommandSpec {
        kind: CommandKind::Exit,
        name: "exit",
        aliases: &["quit", "q"],
        usage: "/exit",
        description: "leave and quit",
    },
];

#[derive(Debug, Clone)]
pub struct CommandRegistry {
    specs: &'static [CommandSpec],
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self {
            specs: COMMAND_SPECS,
        }
    }
}

impl CommandRegistry {
    pub fn parse(&self, input: &str) -> Result<ParsedInput> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(ParsedInput::Empty);
        }

        if !trimmed.starts_with('/') {
            return Ok(ParsedInput::Message(trimmed.to_string()));
        }

        let without_slash = &trimmed[1..];
        let (name, args) = split_name_and_args(without_slash);
        let Some(spec) = self.find(name) else {
            return Err(KayaError::InvalidCommand(format!(
                "unknown command /{name}; try /help"
            )));
        };

        Ok(ParsedInput::Command(self.parse_command(spec, args)?))
    }

    pub fn specs(&self) -> &'static [CommandSpec] {
        self.specs
    }

    pub fn help_text(&self) -> String {
        self.specs
            .iter()
            .map(|spec| format!("{} - {}", spec.usage, spec.description))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    pub fn usages(&self) -> Vec<&'static str> {
        self.specs.iter().map(|spec| spec.usage).collect()
    }

    fn find(&self, name: &str) -> Option<&CommandSpec> {
        self.specs
            .iter()
            .find(|spec| spec.name == name || spec.aliases.contains(&name))
    }

    fn parse_command(&self, spec: &CommandSpec, args: &str) -> Result<Command> {
        match spec.kind {
            CommandKind::Help => Ok(Command::Help),
            CommandKind::Who => Ok(Command::Who),
            CommandKind::Rooms => Ok(Command::Rooms),
            CommandKind::Join => {
                let room = first_arg(args)
                    .ok_or_else(|| KayaError::InvalidCommand(format!("usage: {}", spec.usage)))?;
                Ok(Command::Join {
                    room: normalize_room(room),
                })
            }
            CommandKind::Room => Ok(Command::Room {
                room: first_arg(args).map(normalize_room),
            }),
            CommandKind::Msg => parse_msg_command(args, spec.usage),
            CommandKind::Status => Ok(Command::Status),
            CommandKind::Logs => Ok(Command::Logs),
            CommandKind::Clear => Ok(Command::Clear),
            CommandKind::Exit => Ok(Command::Exit),
        }
    }
}

pub fn parse_input(input: &str) -> Result<ParsedInput> {
    CommandRegistry::default().parse(input)
}

pub fn help_text() -> String {
    CommandRegistry::default().help_text()
}

fn split_name_and_args(input: &str) -> (&str, &str) {
    input
        .split_once(char::is_whitespace)
        .map(|(name, args)| (name, args.trim()))
        .unwrap_or((input, ""))
}

fn first_arg(args: &str) -> Option<&str> {
    args.split_whitespace().next()
}

fn parse_msg_command(args: &str, usage: &str) -> Result<Command> {
    let (target, body) = args
        .trim()
        .split_once(char::is_whitespace)
        .ok_or_else(|| KayaError::InvalidCommand(format!("usage: {usage}")))?;

    let target = target.trim();
    let body = body.trim();
    if target.is_empty() || body.is_empty() {
        return Err(KayaError::InvalidCommand(format!("usage: {usage}")));
    }

    Ok(Command::Msg {
        target: target.to_string(),
        body: body.to_string(),
    })
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
    fn parses_aliases_from_registry() {
        let registry = CommandRegistry::default();

        assert_eq!(
            registry.parse("/dm Ana teste privado agora").unwrap(),
            ParsedInput::Command(Command::Msg {
                target: "Ana".into(),
                body: "teste privado agora".into()
            })
        );
        assert_eq!(
            registry.parse("/q").unwrap(),
            ParsedInput::Command(Command::Exit)
        );
    }

    #[test]
    fn rejects_missing_message_body() {
        assert!(parse_input("/msg Ana").is_err());
    }

    #[test]
    fn generates_help_from_specs() {
        let help = help_text();
        assert!(help.contains("/join <room>"));
        assert!(help.contains("send a direct message"));
    }

    #[test]
    fn exposes_usages_for_future_autocomplete() {
        let registry = CommandRegistry::default();
        assert!(registry.usages().contains(&"/status"));
    }
}
