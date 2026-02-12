use super::*;
use std::collections::HashMap;

pub(super) fn format_time(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    format!("{minutes:02}:{remainder:02}")
}

pub(super) fn parse_seed_input(input: &str) -> Result<Option<u64>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let normalized = trimmed.replace('_', "");
    normalized.parse::<u64>().map(Some).map_err(|_| {
        "Seed must be an unsigned whole number (0 to 18446744073709551615).".to_string()
    })
}

pub(super) fn parse_saved_session(raw: &str) -> Option<PersistedSession> {
    let mut fields = HashMap::<&str, &str>::new();
    for line in raw.lines() {
        let (key, value) = line.split_once('=')?;
        fields.insert(key.trim(), value.trim());
    }

    if fields.get("v").copied() != Some("1") {
        return None;
    }

    let seed = fields.get("seed")?.parse::<u64>().ok()?;
    let mode = GameMode::from_id(fields.get("mode")?)?;
    let move_count = fields.get("moves")?.parse::<u32>().ok()?;
    let elapsed_seconds = fields.get("elapsed")?.parse::<u32>().ok()?;
    let timer_started = match *fields.get("timer")? {
        "1" => true,
        "0" => false,
        _ => return None,
    };
    let game = KlondikeGame::decode_from_session(fields.get("game")?)?;

    Some(PersistedSession {
        seed,
        mode,
        move_count,
        elapsed_seconds,
        timer_started,
        game,
    })
}

pub(super) fn random_seed() -> u64 {
    rand::random()
}

pub(super) fn parse_tableau_payload(payload: &str) -> Option<(usize, usize)> {
    let rest = payload.strip_prefix("tableau:")?;
    let (src, start) = rest.split_once(':')?;
    let src = src.parse::<usize>().ok()?;
    let start = start.parse::<usize>().ok()?;
    Some((src, start))
}
