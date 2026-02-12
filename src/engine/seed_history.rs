use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub struct SeedHistoryStats {
    pub plays: u32,
    pub wins: u32,
    pub last_play_order: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SeedHistoryStore {
    seeds: HashMap<u64, SeedHistoryStats>,
    next_play_order: u64,
}

impl SeedHistoryStore {
    pub fn load_from_path(path: &Path, max_entries: usize) -> Self {
        let mut data = SeedHistoryStore::default();
        let mut max_play_order = 0_u64;
        if let Ok(contents) = fs::read_to_string(path) {
            for line in contents.lines() {
                let mut parts = line.split_whitespace();
                let Some(seed_raw) = parts.next() else {
                    continue;
                };
                let Some(plays_raw) = parts.next() else {
                    continue;
                };
                let Some(wins_raw) = parts.next() else {
                    continue;
                };
                let Ok(seed) = seed_raw.parse::<u64>() else {
                    continue;
                };
                let Ok(plays) = plays_raw.parse::<u32>() else {
                    continue;
                };
                let Ok(wins) = wins_raw.parse::<u32>() else {
                    continue;
                };
                let play_order = parts
                    .next()
                    .and_then(|raw| raw.parse::<u64>().ok())
                    .unwrap_or(0);
                max_play_order = max_play_order.max(play_order);
                data.seeds.insert(
                    seed,
                    SeedHistoryStats {
                        plays,
                        wins: wins.min(plays),
                        last_play_order: play_order,
                    },
                );
            }
        }
        data.next_play_order = max_play_order.saturating_add(1).max(1);
        data.prune(max_entries);
        data
    }

    pub fn save_to_path(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut rows: Vec<(u64, SeedHistoryStats)> = self
            .seeds
            .iter()
            .map(|(seed, stats)| (*seed, *stats))
            .collect();
        rows.sort_unstable_by_key(|(seed, stats)| (Reverse(stats.last_play_order), Reverse(*seed)));

        let mut serialized = String::new();
        for (seed, stats) in rows {
            let wins = stats.wins.min(stats.plays);
            serialized.push_str(&format!(
                "{seed} {} {wins} {}\n",
                stats.plays, stats.last_play_order
            ));
        }
        let _ = fs::write(path, serialized);
    }

    pub fn note_play_started(&mut self, seed: u64, max_entries: usize) {
        if self.next_play_order == 0 {
            self.next_play_order = 1;
        }
        let order = self.next_play_order;
        self.next_play_order = self.next_play_order.saturating_add(1).max(1);
        let stats = self.seeds.entry(seed).or_default();
        stats.plays = stats.plays.saturating_add(1);
        stats.last_play_order = order;
        self.prune(max_entries);
    }

    pub fn note_win(&mut self, seed: u64) {
        let stats = self.seeds.entry(seed).or_default();
        if stats.plays == 0 {
            stats.plays = 1;
        }
        let next_wins = stats.wins.saturating_add(1);
        stats.wins = next_wins.min(stats.plays);
    }

    pub fn dropdown_entries(
        &self,
        max_dropdown_entries: usize,
    ) -> (Vec<(u64, SeedHistoryStats)>, usize) {
        let mut seeds: Vec<(u64, SeedHistoryStats)> = self
            .seeds
            .iter()
            .map(|(seed, stats)| (*seed, *stats))
            .collect();
        seeds
            .sort_unstable_by_key(|(seed, stats)| (Reverse(stats.last_play_order), Reverse(*seed)));
        let total = seeds.len();
        seeds.truncate(max_dropdown_entries);
        (seeds, total)
    }

    fn prune(&mut self, max_entries: usize) {
        if self.seeds.len() <= max_entries {
            return;
        }

        let mut rows: Vec<(u64, SeedHistoryStats)> = self
            .seeds
            .iter()
            .map(|(seed, stats)| (*seed, *stats))
            .collect();
        rows.sort_unstable_by_key(|(seed, stats)| (Reverse(stats.last_play_order), Reverse(*seed)));
        rows.truncate(max_entries);

        self.seeds.clear();
        for (seed, stats) in rows {
            self.seeds.insert(seed, stats);
        }
    }
}
