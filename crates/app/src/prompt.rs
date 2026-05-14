use kaya_persistence::ConfigProfile;
use kaya_shared::{normalize_callsign, Result};
use std::io::{self, Write};

pub fn prompt_callsign(default: Option<&str>) -> Result<String> {
    let mut stdout = io::stdout();
    match default {
        Some(value) if !value.trim().is_empty() => {
            write!(stdout, "KAYA callsign [{value}]: ")?;
        }
        _ => {
            write!(stdout, "KAYA callsign: ")?;
        }
    }
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let callsign = normalize_callsign(&input);
    if !callsign.is_empty() {
        return Ok(callsign);
    }

    if let Some(default) = default {
        let default = normalize_callsign(default);
        if !default.is_empty() {
            return Ok(default);
        }
    }

    Ok("operator".into())
}

pub fn prompt_callsign_if_needed(
    default: Option<&str>,
    demo_profile: Option<ConfigProfile>,
) -> Result<String> {
    if let Some(value) = default {
        let value = normalize_callsign(value);
        if !value.is_empty() {
            return Ok(value);
        }
    }

    if matches!(demo_profile, Some(ConfigProfile::Demo)) {
        return Ok(fake_callsign());
    }

    prompt_callsign(default)
}

fn fake_callsign() -> String {
    let suffix = kaya_shared::now_millis() % 10_000;
    format!("Demo-{suffix:04}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_existing_callsign_without_prompt() {
        let callsign = prompt_callsign_if_needed(Some("Helio"), None).unwrap();
        assert_eq!(callsign, "Helio");
    }

    #[test]
    fn generates_demo_callsign_when_missing() {
        let callsign = prompt_callsign_if_needed(None, Some(ConfigProfile::Demo)).unwrap();
        assert!(callsign.starts_with("Demo-"));
    }
}
