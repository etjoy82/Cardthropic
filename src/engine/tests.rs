use crate::engine::moves::HintMove;
use crate::engine::robot::RobotPlayback;
use crate::engine::session::{decode_persisted_session, encode_persisted_session};
use crate::engine::smart_move;
use crate::engine::variant::{
    all_variant_specs, all_variants, spec_for_id, spec_for_mode, variant_for_mode,
};
use crate::engine::variant_engine::{all_engines, engine_for_mode};
use crate::engine::variant_state::VariantStateStore;
use crate::engine::{
    automation::AutomationProfile, automation::FREECELL_AUTOMATION_PROFILE,
    automation::KLONDIKE_AUTOMATION_PROFILE, automation::SPIDER_AUTOMATION_PROFILE,
};
use crate::engine::{boundary, commands::EngineCommand};
use crate::game::{DrawMode, GameMode, KlondikeGame, SpiderGame};

#[test]
fn robot_playback_anchor_matching_and_mismatch() {
    let mut playback = RobotPlayback::<u8>::default();
    playback.arm(42, DrawMode::Three, 7, 123_456, vec![1, 2, 3]);

    assert!(playback.matches_current(42, DrawMode::Three, 7, 123_456));
    assert!(!playback.matches_current(42, DrawMode::Three, 8, 123_456));
    assert!(!playback.matches_current(42, DrawMode::One, 7, 123_456));
    assert!(!playback.matches_current(43, DrawMode::Three, 7, 123_456));
}

#[test]
fn robot_playback_scripted_line_lifecycle() {
    let mut playback = RobotPlayback::<u8>::default();
    playback.arm(9, DrawMode::One, 0, 77, vec![10, 20]);

    assert!(playback.has_scripted_line());
    assert!(!playback.use_scripted_line());

    playback.set_use_scripted_line(true);
    assert!(playback.use_scripted_line());
    assert_eq!(playback.pop_scripted_move(), Some(10));
    assert_eq!(playback.pop_scripted_move(), Some(20));
    assert_eq!(playback.pop_scripted_move(), None);

    playback.clear_scripted_line();
    assert!(!playback.has_scripted_line());
    assert!(!playback.use_scripted_line());

    playback.clear();
    assert!(!playback.matches_current(9, DrawMode::One, 0, 77));
}

#[test]
fn automation_profile_is_defined_per_mode() {
    for mode in [GameMode::Klondike, GameMode::Spider, GameMode::Freecell] {
        let profile = AutomationProfile::for_mode(mode);
        assert!(profile.auto_play_node_budget > 0);
        assert!(profile.auto_play_beam_width > 0);
        assert!(profile.auto_play_lookahead_depth > 0);
        assert!(profile.robot_step_interval_ms > 0);
        assert!(profile.rapid_wand_total_steps > 0);
    }
}

#[test]
fn klondike_profile_matches_current_tuning_defaults() {
    let p = KLONDIKE_AUTOMATION_PROFILE;
    assert_eq!(p.hint_guided_analysis_budget, 120_000);
    assert_eq!(p.hint_exhaustive_analysis_budget, 220_000);
    assert_eq!(p.auto_play_lookahead_depth, 3);
    assert_eq!(p.auto_play_beam_width, 10);
    assert_eq!(p.auto_play_node_budget, 3_200);
    assert_eq!(p.auto_play_win_score, 1_200_000);
    assert_eq!(p.dialog_seed_guided_budget, 180_000);
    assert_eq!(p.dialog_seed_exhaustive_budget, 300_000);
    assert_eq!(p.dialog_find_winnable_state_budget, 15_000);
    assert_eq!(p.rapid_wand_interval_ms, 750);
    assert_eq!(p.rapid_wand_total_steps, 5);
    assert_eq!(p.robot_step_interval_ms, 250);
}

#[test]
fn variant_registry_has_unique_ids_and_modes() {
    let specs = all_variant_specs();
    assert!(!specs.is_empty());
    assert_eq!(all_variants().len(), specs.len());

    let mut ids = std::collections::HashSet::new();
    let mut modes = std::collections::HashSet::new();
    for spec in specs {
        assert!(ids.insert(spec.id));
        assert!(modes.insert(spec.mode));
        assert_eq!(spec_for_mode(spec.mode).id, spec.id);
        assert_eq!(spec_for_id(spec.id).map(|s| s.mode), Some(spec.mode));
        assert_eq!(variant_for_mode(spec.mode).spec().id, spec.id);
    }
}

#[test]
fn variant_state_store_tracks_klondike_and_spider_separately() {
    let mut store = VariantStateStore::new(10);

    let klondike = KlondikeGame::new_with_seed(111);
    let spider = SpiderGame::new_with_seed(222);
    store.set_klondike(klondike.clone());
    store.set_spider(spider.clone());

    assert_eq!(store.klondike(), &klondike);
    assert_eq!(store.spider(), &spider);
    assert_eq!(
        store.runtime_for_mode(GameMode::Klondike).mode(),
        GameMode::Klondike
    );
    assert_eq!(
        store.runtime_for_mode(GameMode::Spider).mode(),
        GameMode::Spider
    );
}

#[test]
fn variant_engine_registry_matches_modes() {
    let mut registered_modes = std::collections::HashSet::new();
    for engine in all_engines() {
        assert!(registered_modes.insert(engine.mode()));
    }
    assert_eq!(registered_modes.len(), all_variant_specs().len());

    for mode in [GameMode::Klondike, GameMode::Spider, GameMode::Freecell] {
        let engine = engine_for_mode(mode);
        assert_eq!(engine.mode(), mode);
    }
    assert!(engine_for_mode(GameMode::Klondike).engine_ready());
    assert!(!engine_for_mode(GameMode::Spider).engine_ready());
    assert!(!engine_for_mode(GameMode::Freecell).engine_ready());
    assert_eq!(
        engine_for_mode(GameMode::Klondike).automation_profile(),
        KLONDIKE_AUTOMATION_PROFILE
    );
    assert_eq!(
        engine_for_mode(GameMode::Spider).automation_profile(),
        SPIDER_AUTOMATION_PROFILE
    );
    assert_eq!(
        engine_for_mode(GameMode::Freecell).automation_profile(),
        FREECELL_AUTOMATION_PROFILE
    );
    let klondike_caps = engine_for_mode(GameMode::Klondike).capabilities();
    assert!(klondike_caps.draw);
    assert!(klondike_caps.undo_redo);
    assert!(klondike_caps.robot_mode);
    assert!(klondike_caps.winnability);

    let spider_caps = engine_for_mode(GameMode::Spider).capabilities();
    assert!(!spider_caps.draw);
    assert!(!spider_caps.undo_redo);
    assert!(!spider_caps.winnability);

    let freecell_caps = engine_for_mode(GameMode::Freecell).capabilities();
    assert!(!freecell_caps.draw);
    assert!(!freecell_caps.undo_redo);
    assert!(!freecell_caps.winnability);
}

#[test]
fn boundary_execute_command_routes_draw_and_move_commands() {
    let mut state = VariantStateStore::new(17);

    let draw = boundary::execute_command(
        &mut state,
        GameMode::Klondike,
        EngineCommand::DrawOrRecycle {
            draw_mode: DrawMode::One,
        },
    );
    assert!(draw.changed);
    assert!(draw.draw_result.is_some());

    let can_before = state.klondike().can_move_waste_to_foundation();
    let move_try = boundary::execute_command(
        &mut state,
        GameMode::Klondike,
        EngineCommand::MoveWasteToFoundation,
    );
    assert_eq!(move_try.changed, can_before);
}

#[test]
fn boundary_initialize_seeded_with_draw_mode_sets_both() {
    let mut state = VariantStateStore::new(1);
    let ok = boundary::initialize_seeded_with_draw_mode(
        &mut state,
        GameMode::Klondike,
        99,
        DrawMode::Five,
    );
    assert!(ok);
    assert_eq!(state.klondike().draw_mode(), DrawMode::Five);
}

#[test]
fn boundary_is_won_is_mode_aware() {
    let state = VariantStateStore::new(3);

    assert!(!boundary::is_won(&state, GameMode::Klondike));
    assert!(!boundary::is_won(&state, GameMode::Spider));
    assert!(!boundary::is_won(&state, GameMode::Freecell));
}

#[test]
fn smart_move_fallback_helpers_return_legal_moves() {
    let mut state = VariantStateStore::new(17);
    let mode = GameMode::Klondike;

    let _ = boundary::execute_command(
        &mut state,
        mode,
        EngineCommand::DrawOrRecycle {
            draw_mode: DrawMode::One,
        },
    );

    if let Some(HintMove::WasteToTableau { dst }) =
        smart_move::fallback_waste_to_tableau_move(&state, mode)
    {
        assert!(boundary::can_move_waste_to_tableau(&state, mode, dst));
    }

    for col in 0..7 {
        let len = boundary::tableau_len(&state, mode, col).unwrap_or(0);
        if len == 0 {
            continue;
        }
        let start = len - 1;
        if let Some(HintMove::TableauRunToTableau { src, start, dst }) =
            smart_move::fallback_tableau_run_move(&state, mode, col, start)
        {
            assert_eq!(src, col);
            assert!(boundary::can_move_tableau_run_to_tableau(
                &state, mode, src, start, dst
            ));
            return;
        }
    }
}

#[test]
fn persisted_session_v2_round_trip_for_klondike_runtime() {
    let mut state = VariantStateStore::new(42);
    let mode = GameMode::Klondike;
    let _ = boundary::execute_command(
        &mut state,
        mode,
        EngineCommand::DrawOrRecycle {
            draw_mode: DrawMode::Three,
        },
    );
    let encoded = encode_persisted_session(&state, 42, mode, 9, 33, true, DrawMode::Three);
    let decoded = decode_persisted_session(&encoded).expect("decode persisted session");
    assert_eq!(decoded.seed, 42);
    assert_eq!(decoded.mode, GameMode::Klondike);
    assert_eq!(decoded.move_count, 9);
    assert_eq!(decoded.elapsed_seconds, 33);
    assert!(decoded.timer_started);
    assert_eq!(decoded.klondike_draw_mode, DrawMode::Three);
}

#[test]
fn autoplay_hash_and_counts_are_stable_for_same_state() {
    let game = KlondikeGame::new_with_seed(99);
    let h1 = crate::engine::autoplay::hash_game_state(&game);
    let h2 = crate::engine::autoplay::hash_game_state(&game);
    assert_eq!(h1, h2);
    assert!(crate::engine::autoplay::non_draw_move_count(&game) >= 0);
    assert!(crate::engine::autoplay::foundation_count(&game) >= 0);
}

#[test]
fn autoplay_search_can_pick_a_candidate_slot() {
    let game = KlondikeGame::new_with_seed(17);
    let mut seen = std::collections::HashSet::new();
    seen.insert(crate::engine::autoplay::hash_game_state(&game));
    let slots = vec![Some(HintMove::Draw)];
    let picked = crate::engine::autoplay_search::best_candidate_slot_index(
        &game,
        &slots,
        &seen,
        crate::engine::autoplay::hash_game_state(&game),
        KLONDIKE_AUTOMATION_PROFILE,
        |_idx, _mv| true,
    );
    assert_eq!(picked, Some(0));
}

#[test]
fn foundation_safety_helpers_match_legal_move_primitives() {
    let game = KlondikeGame::new_with_seed(17);
    assert!(!crate::engine::foundation_safety::can_auto_move_waste_to_foundation(&game));
    for src in 0..7 {
        let auto_ok =
            crate::engine::foundation_safety::can_auto_move_tableau_to_foundation(&game, src);
        if auto_ok {
            assert!(game.can_move_tableau_top_to_foundation(src));
        }
    }
}

#[test]
fn keyboard_nav_top_row_horizontal_progression_is_stable() {
    let game = KlondikeGame::new_with_seed(17);
    use crate::engine::keyboard_nav::{self, KeyboardTarget};

    assert_eq!(
        keyboard_nav::move_horizontal(&game, KeyboardTarget::Stock, 1),
        KeyboardTarget::Waste
    );
    assert_eq!(
        keyboard_nav::move_horizontal(&game, KeyboardTarget::Waste, 1),
        KeyboardTarget::Foundation(0)
    );
    assert_eq!(
        keyboard_nav::move_horizontal(&game, KeyboardTarget::Foundation(0), -1),
        KeyboardTarget::Waste
    );
}

#[test]
fn keyboard_nav_normalizes_out_of_bounds_targets() {
    let game = KlondikeGame::new_with_seed(17);
    use crate::engine::keyboard_nav::{self, KeyboardTarget};

    let normalized = keyboard_nav::normalize_target(
        &game,
        KeyboardTarget::Tableau {
            col: 99,
            start: Some(999),
        },
    );
    match normalized {
        KeyboardTarget::Tableau { col, .. } => assert_eq!(col, 6),
        _ => panic!("expected tableau target"),
    }
}

#[test]
fn status_text_reports_win_and_selection_paths() {
    let game = KlondikeGame::new_with_seed(17);
    let selected_text = crate::engine::status_text::build_status_text(
        &game,
        Some((0, 0)),
        false,
        false,
        true,
        "Klondike",
        "double-click",
        None,
        None,
    );
    assert!(selected_text.contains("Selected"));

    let override_text = crate::engine::status_text::build_status_text(
        &game,
        None,
        false,
        false,
        true,
        "Klondike",
        "double-click",
        None,
        Some("manual override"),
    );
    assert_eq!(override_text, "manual override");
}

#[test]
fn render_plan_control_sensitivity_respects_caps_and_history() {
    let caps = crate::engine::variant_engine::VariantCapabilities::klondike_default();
    let controls = crate::engine::render_plan::plan_controls(caps, 1, 0);
    assert!(controls.undo_enabled);
    assert!(!controls.redo_enabled);
    assert!(controls.seed_combo_enabled);
    assert!(controls.auto_hint_enabled);
}

#[test]
fn render_plan_waste_visible_count_is_capped() {
    assert_eq!(
        crate::engine::render_plan::waste_visible_count(DrawMode::Five, 12),
        5
    );
    assert_eq!(
        crate::engine::render_plan::waste_visible_count(DrawMode::Three, 1),
        1
    );
}

#[test]
fn seed_ops_parse_and_random_fallback_work() {
    assert_eq!(
        crate::engine::seed_ops::parse_seed_input("12_345").unwrap(),
        Some(12345)
    );
    assert_eq!(
        crate::engine::seed_ops::parse_seed_input("   ").unwrap(),
        None
    );
    assert!(crate::engine::seed_ops::parse_seed_input("not-a-seed").is_err());

    let seed = crate::engine::seed_ops::seed_from_text_or_random("").unwrap();
    // random is allowed to be any u64, but function should always return something.
    let _ = seed;
}

#[test]
fn seed_ops_messages_are_stable() {
    let msg = crate::engine::seed_ops::msg_started_winnable_seed(42, 3, 9);
    assert!(msg.contains("Seed 42"));
    assert!(msg.contains("Deal 3"));
    assert!(msg.contains("checked 9 seed(s)"));
}
