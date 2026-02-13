pub(super) fn format_time(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    format!("{minutes:02}:{remainder:02}")
}

pub(super) fn parse_tableau_payload(payload: &str) -> Option<(usize, usize)> {
    let rest = payload.strip_prefix("tableau:")?;
    let (src, start) = rest.split_once(':')?;
    let src = src.parse::<usize>().ok()?;
    let start = start.parse::<usize>().ok()?;
    Some((src, start))
}
