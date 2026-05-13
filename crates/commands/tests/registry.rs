use kaya_commands::{Command, CommandRegistry, ParsedInput};

#[test]
fn registry_lists_every_command_usage() {
    let registry = CommandRegistry::default();
    let usages = registry.usages();

    assert!(usages.contains(&"/help"));
    assert!(usages.contains(&"/join <room>"));
    assert!(usages.contains(&"/msg <callsign|node-id> <message>"));
}

#[test]
fn registry_validates_required_arguments() {
    let registry = CommandRegistry::default();

    assert!(registry.parse("/join").is_err());
    assert!(registry.parse("/msg Ana").is_err());
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
