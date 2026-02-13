use std::collections::HashMap;

use crate::engine::game_mode::VariantRuntime;
use crate::engine::variant_state::VariantStateStore;
use crate::game::{DrawMode, GameMode, KlondikeGame};

#[derive(Debug, Clone)]
pub struct PersistedSession {
    pub seed: u64,
    pub mode: GameMode,
    pub move_count: u32,
    pub elapsed_seconds: u32,
    pub timer_started: bool,
    pub runtime: VariantRuntime,
    pub klondike_draw_mode: DrawMode,
}

pub fn encode_persisted_session(
    state: &VariantStateStore,
    seed: u64,
    mode: GameMode,
    move_count: u32,
    elapsed_seconds: u32,
    timer_started: bool,
    klondike_draw_mode: DrawMode,
) -> String {
    format!(
        "v=2\nseed={}\nmode={}\nmoves={}\nelapsed={}\ntimer={}\ndraw={}\nruntime={}",
        seed,
        mode.id(),
        move_count,
        elapsed_seconds,
        if timer_started { 1 } else { 0 },
        klondike_draw_mode.count(),
        state.encode_runtime_for_session(mode),
    )
}

pub fn decode_persisted_session(raw: &str) -> Option<PersistedSession> {
    let mut fields = HashMap::<&str, &str>::new();
    for line in raw.lines() {
        let (key, value) = line.split_once('=')?;
        fields.insert(key.trim(), value.trim());
    }

    let version = fields.get("v").copied()?;
    let seed = fields.get("seed")?.parse::<u64>().ok()?;
    let mode = GameMode::from_id(fields.get("mode")?)?;
    let move_count = fields.get("moves")?.parse::<u32>().ok()?;
    let elapsed_seconds = fields.get("elapsed")?.parse::<u32>().ok()?;
    let timer_started = match *fields.get("timer")? {
        "1" => true,
        "0" => false,
        _ => return None,
    };

    match version {
        "2" => {
            let draw_mode = DrawMode::from_count(fields.get("draw")?.parse::<u8>().ok()?)?;
            let runtime =
                VariantStateStore::decode_runtime_for_session(mode, fields.get("runtime")?)?;
            Some(PersistedSession {
                seed,
                mode,
                move_count,
                elapsed_seconds,
                timer_started,
                runtime,
                klondike_draw_mode: draw_mode,
            })
        }
        // Backward compatibility for older Klondike-only sessions.
        "1" => {
            let game = KlondikeGame::decode_from_session(fields.get("game")?)?;
            let draw_mode = game.draw_mode();
            Some(PersistedSession {
                seed,
                mode,
                move_count,
                elapsed_seconds,
                timer_started,
                runtime: VariantRuntime::Klondike(game),
                klondike_draw_mode: draw_mode,
            })
        }
        _ => None,
    }
}
