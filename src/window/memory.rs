use super::*;
use std::fs;

fn read_status_kib(field: &str) -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(parse_kib_line)
}

fn read_smaps_rollup_kib(field: &str) -> Option<u64> {
    let smaps = fs::read_to_string("/proc/self/smaps_rollup").ok()?;
    smaps
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(parse_kib_line)
}

fn parse_kib_line(line: &str) -> Option<u64> {
    line.split_whitespace().next()?.parse::<u64>().ok()
}

impl CardthropicWindow {
    pub(super) fn current_memory_mib_text(&self) -> String {
        // GNOME System Monitor's "Memory" for processes maps closer to private
        // resident usage than full RSS. RssAnon tracks that best for us.
        let kib = read_status_kib("RssAnon:")
            .or_else(|| read_smaps_rollup_kib("RssAnon:"))
            .or_else(|| read_status_kib("VmRSS:"));
        match kib {
            Some(kib) => {
                let mib = (kib as f64 / 1024.0).round() as u64;
                format!("{mib} MiB")
            }
            None => "n/a".to_string(),
        }
    }

    pub(super) fn trim_process_memory_if_supported(&self) {
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        unsafe {
            unsafe extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            let _ = malloc_trim(0);
        }
    }
}
