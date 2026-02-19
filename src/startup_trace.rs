use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static ENABLED: OnceLock<bool> = OnceLock::new();
static STDERR_ENABLED: OnceLock<bool> = OnceLock::new();
#[derive(Clone, Copy)]
struct TraceMark {
    elapsed_ms: u128,
    seq: u64,
}

static MARKS: OnceLock<Mutex<HashMap<&'static str, TraceMark>>> = OnceLock::new();
static ONCE_MARKS: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
static SUMMARY_PRINTED: AtomicBool = AtomicBool::new(false);
static DECK_SUMMARY_PRINTED: AtomicBool = AtomicBool::new(false);
static MARK_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| true)
}

fn stderr_enabled() -> bool {
    *STDERR_ENABLED.get_or_init(|| {
        std::env::var("CT_STARTUP_TRACE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

pub fn init() {
    if !enabled() {
        return;
    }
    let _ = START.get_or_init(Instant::now);
    let _ = MARKS.get_or_init(|| Mutex::new(HashMap::new()));
    let _ = ONCE_MARKS.get_or_init(|| Mutex::new(HashSet::new()));
}

pub fn mark(label: &'static str) {
    if !enabled() {
        return;
    }
    let start = START.get_or_init(Instant::now);
    let elapsed_ms = start.elapsed().as_millis();
    if let Some(marks) = MARKS.get() {
        if let Ok(mut marks) = marks.lock() {
            let seq = MARK_SEQ.fetch_add(1, Ordering::Relaxed);
            marks.insert(label, TraceMark { elapsed_ms, seq });
        }
    }
    if stderr_enabled() {
        eprintln!("[startup] t={}ms {}", elapsed_ms, label);
    }
}

pub fn mark_once(label: &'static str) {
    if !enabled() {
        return;
    }
    let Some(set) = ONCE_MARKS.get() else {
        mark(label);
        return;
    };
    if let Ok(mut set) = set.lock() {
        if set.insert(label) {
            drop(set);
            mark(label);
        }
    }
}

fn delta(a: &'static str, b: &'static str) -> Option<u128> {
    let marks = MARKS.get()?;
    let marks = marks.lock().ok()?;
    let av = marks.get(a)?.elapsed_ms;
    let bv = marks.get(b)?.elapsed_ms;
    Some(bv.saturating_sub(av))
}

fn summary_line() -> String {
    let total = delta("main:start", "window:first-map").unwrap_or(0);
    let activate = delta("app:activate-enter", "app:activate-exit").unwrap_or(0);
    let window_new = delta("window:new-enter", "window:new-exit").unwrap_or(0);
    let constructed = delta("window:constructed-enter", "window:constructed-exit").unwrap_or(0);
    let session = delta("session:restore-start", "session:restore-end").unwrap_or(0);
    let first_render = delta("render:first-enter", "render:first-exit").unwrap_or(0);
    let first_images = delta("render:first-images-enter", "render:first-images-exit").unwrap_or(0);
    let first_deck_load = delta(
        "render:first-deck-load-enter",
        "render:first-deck-load-exit",
    )
    .unwrap_or(0);
    let first_deck_worker = delta(
        "render:first-deck-worker-enter",
        "render:first-deck-worker-exit",
    )
    .unwrap_or(0);
    let first_deck_main_build = delta(
        "render:first-deck-main-build-enter",
        "render:first-deck-main-build-exit",
    )
    .unwrap_or(0);
    let first_metrics =
        delta("render:first-metrics-enter", "render:first-metrics-exit").unwrap_or(0);
    let first_toprow = delta("render:first-toprow-enter", "render:first-toprow-exit").unwrap_or(0);
    let first_tableau =
        delta("render:first-tableau-enter", "render:first-tableau-exit").unwrap_or(0);
    format!(
        "[startup] summary total_ms={} activate_ms={} window_new_ms={} constructed_ms={} session_restore_ms={} first_render_ms={} first_images_ms={} first_deck_load_ms={} first_deck_worker_ms={} first_deck_main_build_ms={} first_metrics_ms={} first_toprow_ms={} first_tableau_ms={}",
        total,
        activate,
        window_new,
        constructed,
        session,
        first_render,
        first_images,
        first_deck_load,
        first_deck_worker,
        first_deck_main_build,
        first_metrics,
        first_toprow,
        first_tableau
    )
}

fn deck_ready_line() -> String {
    let total_to_deck = delta("main:start", "render:first-deck-load-exit").unwrap_or(0);
    let map_to_deck = delta("window:first-map", "render:first-deck-load-exit").unwrap_or(0);
    let deck_total = delta(
        "render:first-deck-load-enter",
        "render:first-deck-load-exit",
    )
    .unwrap_or(0);
    let deck_worker = delta(
        "render:first-deck-worker-enter",
        "render:first-deck-worker-exit",
    )
    .unwrap_or(0);
    let deck_main = delta(
        "render:first-deck-main-build-enter",
        "render:first-deck-main-build-exit",
    )
    .unwrap_or(0);
    format!(
        "[startup] deck_ready total_to_cards_ms={} map_to_cards_ms={} first_deck_load_ms={} first_deck_worker_ms={} first_deck_main_build_ms={}",
        total_to_deck, map_to_deck, deck_total, deck_worker, deck_main
    )
}

pub fn history_lines() -> Vec<String> {
    if !enabled() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    if let Some(marks) = MARKS.get() {
        if let Ok(marks) = marks.lock() {
            let mut sorted: Vec<_> = marks.iter().map(|(k, v)| (*k, *v)).collect();
            sorted.sort_by_key(|(_, mark)| (mark.elapsed_ms, mark.seq));
            for (label, mark) in sorted {
                let elapsed_ms = mark.elapsed_ms;
                lines.push(format!("[startup] t={}ms {}", elapsed_ms, label));
            }
        }
    }
    lines.push(summary_line());
    lines
}

pub fn print_summary_once() {
    if !enabled() || SUMMARY_PRINTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if stderr_enabled() {
        eprintln!("{}", summary_line());
    }
}

pub fn deck_history_lines() -> Vec<String> {
    if !enabled() {
        return Vec::new();
    }
    vec![deck_ready_line()]
}

pub fn print_deck_summary_once() {
    if !enabled() || DECK_SUMMARY_PRINTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if stderr_enabled() {
        eprintln!("{}", deck_ready_line());
    }
}
