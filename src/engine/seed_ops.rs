pub fn parse_seed_input(input: &str) -> Result<Option<u64>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let normalized = trimmed.replace('_', "");
    normalized.parse::<u64>().map(Some).map_err(|_| {
        "Seed must be an unsigned whole number (0 to 18446744073709551615).".to_string()
    })
}

pub fn random_seed() -> u64 {
    rand::random()
}

pub fn seed_from_text_or_random(input: &str) -> Result<u64, String> {
    Ok(parse_seed_input(input)?.unwrap_or_else(random_seed))
}

pub fn seed_dropdown_tooltip(
    total_seed_count: usize,
    max_dropdown_entries: usize,
) -> Option<String> {
    if total_seed_count > max_dropdown_entries {
        Some(format!(
            "Showing latest {} of {} seeds. Type any seed number to load.",
            max_dropdown_entries, total_seed_count
        ))
    } else {
        None
    }
}

pub fn msg_seed_search_running() -> String {
    "A winnable-seed search is already running.".to_string()
}

pub fn msg_seed_search_still_running() -> String {
    "A winnable-seed search is still running. Please wait.".to_string()
}

pub fn msg_started_seed(seed: u64) -> String {
    format!("Started a new game. Seed {seed}.")
}

pub fn msg_repeated_seed(seed: u64) -> String {
    format!("Dealt again. Seed {seed}.")
}

pub fn msg_winnability_check_canceled(deal_count: u8) -> String {
    format!("Winnability check canceled (Deal {deal_count}).")
}

pub fn msg_winnability_check_timed_out(deal_count: u8, seconds: u32, iterations: usize) -> String {
    format!(
        "Winnability check timed out after {seconds}s (Deal {deal_count}): no wins found in {iterations} iterations before giving up."
    )
}

pub fn msg_winnability_check_stopped_unexpectedly(deal_count: u8) -> String {
    format!("Winnability check stopped unexpectedly (Deal {deal_count}).")
}

pub fn msg_seed_winnable(seed: u64, deal_count: u8, moves: u32, iterations: usize) -> String {
    format!(
        "Seed {seed} is winnable for Deal {deal_count} from a fresh deal (solver line: {moves} moves, {iterations} iterations). Use Robot as first action to see win."
    )
}

pub fn msg_seed_unwinnable_limited(seed: u64, deal_count: u8, iterations: usize) -> String {
    format!(
        "Seed {seed} not proven winnable for Deal {deal_count} from a fresh deal ({iterations} iterations, limits hit)."
    )
}

pub fn msg_seed_unwinnable(seed: u64, deal_count: u8, iterations: usize) -> String {
    format!(
        "Seed {seed} is not winnable for Deal {deal_count} from a fresh deal ({iterations} iterations)."
    )
}

pub fn msg_searching_winnable_seed(
    start_seed: u64,
    deal_count: u8,
    attempts: u32,
    max_states: usize,
) -> String {
    format!(
        "Searching Deal {deal_count} winnable seed from {start_seed} (attempts: {attempts}, state budget: {max_states})..."
    )
}

pub fn msg_started_winnable_seed(seed: u64, deal_count: u8, tested: u32) -> String {
    format!(
        "Started Deal {deal_count} winnable game. Seed {seed} (checked {tested} seed(s)). Use Robot as first action to see win."
    )
}

pub fn msg_no_winnable_seed(start_seed: u64, deal_count: u8, attempts: u32) -> String {
    format!(
        "No Deal {deal_count} winnable seed found in {attempts} attempt(s) from seed {start_seed}."
    )
}

pub fn msg_seed_search_stopped_unexpectedly(deal_count: u8) -> String {
    format!("Deal {deal_count} seed search stopped unexpectedly.")
}
