use kaya_shared::{validate_room_name, KayaError, PresenceStatus, Result};

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
    Create { room: String },
    Join { room: String },
    Leave { room: String },
    Current,
    RoomMessage { body: String },
    Msg { target: String, body: String },
    Presence { status: PresenceStatus },
    History { room: Option<String> },
    DmHistory { peer: String },
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
    Create,
    Join,
    Leave,
    Current,
    RoomMessage,
    Msg,
    Presence,
    History,
    DmHistory,
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
        description: "list discovered peers and presence",
    },
    CommandSpec {
        kind: CommandKind::Rooms,
        name: "rooms",
        aliases: &[],
        usage: "/rooms",
        description: "list known rooms",
    },
    CommandSpec {
        kind: CommandKind::Create,
        name: "create",
        aliases: &[],
        usage: "/create <room>",
        description: "create and announce a room",
    },
    CommandSpec {
        kind: CommandKind::Join,
        name: "join",
        aliases: &["j"],
        usage: "/join <room>",
        description: "join a room",
    },
    CommandSpec {
        kind: CommandKind::Leave,
        name: "leave",
        aliases: &["part"],
        usage: "/leave <room>",
        description: "leave a room",
    },
    CommandSpec {
        kind: CommandKind::Current,
        name: "current",
        aliases: &["here"],
        usage: "/current",
        description: "show current room",
    },
    CommandSpec {
        kind: CommandKind::RoomMessage,
        name: "room",
        aliases: &["say"],
        usage: "/room <message>",
        description: "send a message to the current room",
    },
    CommandSpec {
        kind: CommandKind::Msg,
        name: "msg",
        aliases: &["dm"],
        usage: "/msg <callsign|node-id> <message>",
        description: "send a direct message",
    },
    CommandSpec {
        kind: CommandKind::Presence,
        name: "presence",
        aliases: &["p"],
        usage: "/presence <online|away|busy|invisible>",
        description: "update local presence",
    },
    CommandSpec {
        kind: CommandKind::History,
        name: "history",
        aliases: &[],
        usage: "/history [room]",
        description: "show local room history",
    },
    CommandSpec {
        kind: CommandKind::DmHistory,
        name: "dm-history",
        aliases: &["dmhistory"],
        usage: "/dm-history <peer>",
        description: "show local DM history with a peer",
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
            CommandKind::Create => Ok(Command::Create {
                room: parse_room_arg(args, spec.usage)?,
            }),
            CommandKind::Join => Ok(Command::Join {
                room: parse_room_arg(args, spec.usage)?,
            }),
            CommandKind::Leave => Ok(Command::Leave {
                room: parse_room_arg(args, spec.usage)?,
            }),
            CommandKind::Current => Ok(Command::Current),
            CommandKind::RoomMessage => {
                let body = args.trim();
                if body.is_empty() {
                    return Err(KayaError::InvalidCommand(format!("usage: {}", spec.usage)));
                }
                Ok(Command::RoomMessage {
                    body: body.to_string(),
                })
            }
            CommandKind::Msg => parse_msg_command(args, spec.usage),
            CommandKind::Presence => {
                let Some(status) = first_arg(args).and_then(PresenceStatus::parse) else {
                    return Err(KayaError::InvalidCommand(format!("usage: {}", spec.usage)));
                };
                if status == PresenceStatus::Offline {
                    return Err(KayaError::InvalidCommand(
                        "presence cannot be set to offline manually".into(),
                    ));
                }
                Ok(Command::Presence { status })
            }
            CommandKind::History => Ok(Command::History {
                room: first_arg(args).map(validate_room_name).transpose()?,
            }),
            CommandKind::DmHistory => {
                let peer = first_arg(args)
                    .ok_or_else(|| KayaError::InvalidCommand(format!("usage: {}", spec.usage)))?;
                Ok(Command::DmHistory {
                    peer: peer.to_string(),
                })
            }
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

fn parse_room_arg(args: &str, usage: &str) -> Result<String> {
    let room =
        first_arg(args).ok_or_else(|| KayaError::InvalidCommand(format!("usage: {usage}")))?;
    validate_room_name(room)
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
    fn parses_new_phase_two_commands() {
        let registry = CommandRegistry::default();

        assert_eq!(
            registry.parse("/create semana-info").unwrap(),
            ParsedInput::Command(Command::Create {
                room: "semana-info".into()
            })
        );
        assert_eq!(
            registry.parse("/presence busy").unwrap(),
            ParsedInput::Command(Command::Presence {
                status: PresenceStatus::Busy
            })
        );
        assert_eq!(
            registry.parse("/room sistema online").unwrap(),
            ParsedInput::Command(Command::RoomMessage {
                body: "sistema online".into()
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
    fn rejects_invalid_presence() {
        assert!(parse_input("/presence asleep").is_err());
        assert!(parse_input("/presence offline").is_err());
    }

    #[test]
    fn generates_help_from_specs() {
        let help = help_text();
        assert!(help.contains("/join <room>"));
        assert!(help.contains("/presence <online|away|busy|invisible>"));
    }

    #[test]
    fn exposes_usages_for_future_autocomplete() {
        let registry = CommandRegistry::default();
        assert!(registry.usages().contains(&"/status"));
    }
}
