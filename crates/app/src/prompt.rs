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
