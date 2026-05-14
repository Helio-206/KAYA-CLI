use kaya_commands::{Command, CommandRegistry, ParsedInput};

#[test]
fn registry_lists_every_command_usage() {
    let registry = CommandRegistry::default();
    let usages = registry.usages();

    assert!(usages.contains(&"/help"));
    assert!(usages.contains(&"/join <room>"));
    assert!(usages.contains(&"/msg <callsign|node-id> <message>"));
    assert!(usages.contains(&"/secure-msg <callsign|node-id> <message>"));
    assert!(usages.contains(&"/send <callsign|node-id> <path>"));
    assert!(usages.contains(&"/trust <peer>"));
}

#[test]
fn registry_validates_required_arguments() {
    let registry = CommandRegistry::default();

    assert!(registry.parse("/join").is_err());
    assert!(registry.parse("/msg Ana").is_err());
    assert!(registry.parse("/secure-msg Ana").is_err());
    assert!(registry.parse("/send Ana").is_err());
    assert!(registry.parse("/block").is_err());
}

#[test]
fn registry_parses_room_alias_and_normalizes() {
    let registry = CommandRegistry::default();

    assert_eq!(
        registry.parse("/j #Semana-Info").unwrap(),
        ParsedInput::Command(Command::Join {
            room: "semana-info".into()
        })
    );
}

#[test]
fn registry_parses_file_commands() {
    let registry = CommandRegistry::default();

    assert_eq!(
        registry.parse("/send Ana ./docs/PROTOCOL.md").unwrap(),
        ParsedInput::Command(Command::SendFile {
            target: "Ana".into(),
            path: "./docs/PROTOCOL.md".into()
        })
    );
    assert_eq!(
        registry.parse("/reject-file KF-ABCDEF123456").unwrap(),
        ParsedInput::Command(Command::RejectFile {
            file_id: "KF-ABCDEF123456".into()
        })
    );
}

#[test]
fn registry_parses_peer_fingerprints_alias() {
    let registry = CommandRegistry::default();

    assert_eq!(
        registry.parse("/peers --fingerprints").unwrap(),
        ParsedInput::Command(Command::Who { fingerprints: true })
    );
}
