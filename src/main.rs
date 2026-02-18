/* main.rs
 *
 * Copyright 2026 emviolet
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod application;
mod config;
mod deck;
mod engine;
mod game;
mod window;
mod winnability;

use self::application::CardthropicApplication;
use self::window::CardthropicWindow;
use crate::engine::automation::FREECELL_AUTOMATION_PROFILE;
use crate::game::FreecellCardCountMode;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::prelude::*;
use gtk::{gio, glib};
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct FreecellBenchmarkOptions {
    start_seed: u64,
    attempts: u32,
    card_count_mode: FreecellCardCountMode,
    out_path: String,
    wcheck_attempts: u32,
    wcheck_seed_time_ms: u64,
}

impl Default for FreecellBenchmarkOptions {
    fn default() -> Self {
        Self {
            start_seed: 4_954_934_047_608_000_431,
            attempts: 10_000,
            card_count_mode: FreecellCardCountMode::FiftyTwo,
            out_path: "benchmarks/freecell_baseline.json".to_string(),
            wcheck_attempts: 500,
            wcheck_seed_time_ms: 200,
        }
    }
}

fn parse_u64(value: Option<String>, flag: &str) -> Result<u64, String> {
    value
        .ok_or_else(|| format!("missing value for {flag}"))?
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}"))
}

fn parse_u32(value: Option<String>, flag: &str) -> Result<u32, String> {
    value
        .ok_or_else(|| format!("missing value for {flag}"))?
        .parse::<u32>()
        .map_err(|_| format!("invalid value for {flag}"))
}

fn parse_freecell_card_count_mode(value: &str) -> Result<FreecellCardCountMode, String> {
    match value {
        "26" => Ok(FreecellCardCountMode::TwentySix),
        "39" => Ok(FreecellCardCountMode::ThirtyNine),
        "52" => Ok(FreecellCardCountMode::FiftyTwo),
        _ => Err("card count must be one of: 26, 39, 52".to_string()),
    }
}

fn parse_benchmark_args(args: &[String]) -> Result<Option<FreecellBenchmarkOptions>, String> {
    let mut idx = 1usize;
    let mut options = FreecellBenchmarkOptions::default();
    let mut enabled = false;

    while idx < args.len() {
        match args[idx].as_str() {
            "--benchmark-freecell" => {
                enabled = true;
                idx += 1;
            }
            "--start-seed" => {
                options.start_seed = parse_u64(args.get(idx + 1).cloned(), "--start-seed")?;
                idx += 2;
            }
            "--attempts" => {
                options.attempts = parse_u32(args.get(idx + 1).cloned(), "--attempts")?;
                idx += 2;
            }
            "--freecell-card-count" => {
                let raw = args
                    .get(idx + 1)
                    .ok_or_else(|| "missing value for --freecell-card-count".to_string())?;
                options.card_count_mode = parse_freecell_card_count_mode(raw)?;
                idx += 2;
            }
            "--out" => {
                options.out_path = args
                    .get(idx + 1)
                    .cloned()
                    .ok_or_else(|| "missing value for --out".to_string())?;
                idx += 2;
            }
            "--wcheck-attempts" => {
                options.wcheck_attempts =
                    parse_u32(args.get(idx + 1).cloned(), "--wcheck-attempts")?;
                idx += 2;
            }
            "--wcheck-seed-time-ms" => {
                options.wcheck_seed_time_ms =
                    parse_u64(args.get(idx + 1).cloned(), "--wcheck-seed-time-ms")?;
                idx += 2;
            }
            "--help" | "-h" => {
                println!(
                    "Cardthropic\n\
                     --benchmark-freecell [--start-seed N] [--attempts N] [--freecell-card-count 26|39|52] [--out PATH] [--wcheck-attempts N] [--wcheck-seed-time-ms N]"
                );
                return Ok(None);
            }
            _ => {
                idx += 1;
            }
        }
    }

    if enabled {
        Ok(Some(options))
    } else {
        Ok(None)
    }
}

fn current_rss_mib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kb = line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())?;
    Some((kb + 1023) / 1024)
}

fn run_freecell_benchmark(options: &FreecellBenchmarkOptions) -> Result<(), String> {
    let profile = FREECELL_AUTOMATION_PROFILE;
    let cancel = Arc::new(AtomicBool::new(false));
    let progress_checked = Arc::new(AtomicU32::new(0));
    let progress_stats = Arc::new(winnability::FreecellFindProgress::default());
    let started = Instant::now();
    let start_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut peak_rss_mib = current_rss_mib().unwrap_or(0);

    let (tx, rx) = mpsc::channel();
    let thread_cancel = Arc::clone(&cancel);
    let thread_progress = Arc::clone(&progress_checked);
    let thread_progress_stats = Arc::clone(&progress_stats);
    let start_seed = options.start_seed;
    let attempts = options.attempts;
    let card_count_mode = options.card_count_mode;
    thread::spawn(move || {
        let result = winnability::find_winnable_freecell_seed_parallel(
            start_seed,
            attempts,
            profile.dialog_seed_guided_budget,
            profile.dialog_seed_exhaustive_budget,
            card_count_mode,
            thread_cancel,
            Some(thread_progress),
            Some(thread_progress_stats),
        );
        let _ = tx.send(result);
    });

    let mut last_logged = Instant::now();
    let find_result = loop {
        if let Some(mib) = current_rss_mib() {
            if mib > peak_rss_mib {
                peak_rss_mib = mib;
            }
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => break result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_logged.elapsed() >= Duration::from_secs(1) {
                    let checked = progress_checked.load(Ordering::Relaxed);
                    eprintln!(
                        "[bench] find-winnable progress: checked={checked}/{} current_seed={} expanded={} branches={} elapsed_ms={} stop={}",
                        options.attempts,
                        options.start_seed.wrapping_add(u64::from(checked.saturating_sub(1))),
                        progress_stats.last_expanded_states.load(Ordering::Relaxed),
                        progress_stats.last_generated_branches.load(Ordering::Relaxed),
                        progress_stats.last_elapsed_ms.load(Ordering::Relaxed),
                        winnability::freecell_find_stop_reason_label(
                            progress_stats.last_stop_reason.load(Ordering::Relaxed)
                        ),
                    );
                    last_logged = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("benchmark worker disconnected".to_string());
            }
        }
    };
    cancel.store(true, Ordering::Relaxed);
    let find_elapsed_s = started.elapsed().as_secs_f64();
    let checked = progress_checked.load(Ordering::Relaxed);
    let checked_final = if checked == 0 {
        options.attempts
    } else {
        checked
    };
    let seeds_per_sec = if find_elapsed_s > 0.0 {
        f64::from(checked_final) / find_elapsed_s
    } else {
        0.0
    };

    let wcheck_attempts = options.wcheck_attempts.min(options.attempts);
    let wcheck_card_count_mode = options.card_count_mode;
    let wcheck_guided_budget = profile.dialog_seed_guided_budget;
    let wcheck_exhaustive_budget = profile.dialog_seed_exhaustive_budget;
    let wcheck_timeout_ms = options.wcheck_seed_time_ms;
    let mut wcheck_wins: u32 = 0;
    let wcheck_started = Instant::now();
    let mut wcheck_timed_out: u32 = 0;
    for idx in 0..wcheck_attempts {
        let seed = options.start_seed.wrapping_add(u64::from(idx));
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_worker = Arc::clone(&cancel);
        let (seed_tx, seed_rx) = mpsc::channel();
        thread::spawn(move || {
            let result = winnability::is_freecell_seed_winnable(
                seed,
                wcheck_card_count_mode,
                wcheck_guided_budget,
                wcheck_exhaustive_budget,
                cancel_worker.as_ref(),
            );
            let _ = seed_tx.send(result);
        });
        let maybe_result = seed_rx.recv_timeout(Duration::from_millis(wcheck_timeout_ms));
        let result = match maybe_result {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancel.store(true, Ordering::Relaxed);
                wcheck_timed_out = wcheck_timed_out.saturating_add(1);
                None
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => None,
        };
        if let Some(result) = result {
            if result.winnable {
                wcheck_wins = wcheck_wins.saturating_add(1);
            }
        }
    }
    let wcheck_elapsed_s = wcheck_started.elapsed().as_secs_f64();
    if let Some(mib) = current_rss_mib() {
        if mib > peak_rss_mib {
            peak_rss_mib = mib;
        }
    }

    let found_seed_json = match find_result {
        Some((seed, tested, _line)) => {
            format!("{{\"seed\":{seed},\"checked\":{tested}}}")
        }
        None => "null".to_string(),
    };
    let card_count = options.card_count_mode.card_count();
    let wcheck_rate = if wcheck_attempts > 0 {
        f64::from(wcheck_wins) / f64::from(wcheck_attempts)
    } else {
        0.0
    };
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(start_unix);

    let payload = format!(
        "{{\n  \"version\": 1,\n  \"generated_at_unix\": {generated_at},\n  \"mode\": \"freecell\",\n  \"card_count\": {card_count},\n  \"start_seed\": {},\n  \"attempts\": {},\n  \"find_winnable\": {{\n    \"checked\": {checked_final},\n    \"elapsed_seconds\": {:.3},\n    \"seeds_per_second\": {:.3},\n    \"found\": {found_seed_json}\n  }},\n  \"wcheck\": {{\n    \"checked\": {wcheck_attempts},\n    \"wins\": {wcheck_wins},\n    \"win_rate\": {:.6},\n    \"timed_out\": {wcheck_timed_out},\n    \"seed_timeout_ms\": {},\n    \"elapsed_seconds\": {:.3}\n  }},\n  \"memory\": {{\n    \"peak_rss_mib\": {peak_rss_mib}\n  }}\n}}\n",
        options.start_seed,
        options.attempts,
        find_elapsed_s,
        seeds_per_sec,
        wcheck_rate,
        wcheck_timeout_ms,
        wcheck_elapsed_s,
    );

    if let Some(parent) = std::path::Path::new(&options.out_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(&options.out_path, payload).map_err(|e| e.to_string())?;
    eprintln!("Wrote benchmark baseline: {}", options.out_path);
    Ok(())
}

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match parse_benchmark_args(&args) {
        Ok(Some(options)) => match run_freecell_benchmark(&options) {
            Ok(()) => return glib::ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                return glib::ExitCode::FAILURE;
            }
        },
        Ok(None) => {}
        Err(message) => {
            eprintln!("{message}");
            return glib::ExitCode::FAILURE;
        }
    }

    glib::set_prgname(Some("cardthropic"));
    glib::set_application_name("Cardthropic");

    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let resources = gio::Resource::load(PKGDATADIR.to_owned() + "/cardthropic.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    let app = CardthropicApplication::new(
        "io.codeberg.emviolet.cardthropic",
        &gio::ApplicationFlags::empty(),
    );
    app.run()
}
