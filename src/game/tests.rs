use super::*;
use crate::engine::moves::map_solver_line_to_hint_line;

fn card(suit: Suit, rank: u8, face_up: bool) -> Card {
    Card {
        suit,
        rank,
        face_up,
    }
}

fn empty_game() -> KlondikeGame {
    KlondikeGame {
        draw_mode: DrawMode::One,
        stock: Vec::new(),
        waste: Vec::new(),
        foundations: std::array::from_fn(|_| Vec::new()),
        tableau: std::array::from_fn(|_| Vec::new()),
    }
}

#[test]
fn new_game_has_full_deck_accounted_for() {
    let game = KlondikeGame::new_shuffled();

    let tableau_count: usize = game.tableau.iter().map(Vec::len).sum();
    let foundations_count: usize = game.foundations.iter().map(Vec::len).sum();
    let total = game.stock.len() + game.waste.len() + foundations_count + tableau_count;

    assert_eq!(total, 52);
    assert_eq!(tableau_count, 28);
    assert_eq!(game.stock.len(), 24);
    assert_eq!(game.waste.len(), 0);
}

#[test]
fn seeded_games_are_deterministic() {
    let game_a = KlondikeGame::new_with_seed(42);
    let game_b = KlondikeGame::new_with_seed(42);
    let game_c = KlondikeGame::new_with_seed(43);

    assert_eq!(game_a, game_b);
    assert_ne!(game_a, game_c);
}

#[test]
fn draw_moves_one_card_from_stock_to_waste_face_up() {
    let mut game = empty_game();
    game.stock.push(card(Suit::Spades, 7, false));

    let result = game.draw_or_recycle();

    assert_eq!(result, DrawResult::DrewFromStock);
    assert_eq!(game.stock.len(), 0);
    assert_eq!(game.waste.len(), 1);
    assert!(game.waste[0].face_up);
    assert_eq!(game.waste[0].rank, 7);
}

#[test]
fn draw_recycles_waste_back_to_stock_face_down() {
    let mut game = empty_game();
    game.waste.push(card(Suit::Hearts, 2, true));
    game.waste.push(card(Suit::Clubs, 9, true));

    let result = game.draw_or_recycle();

    assert_eq!(result, DrawResult::RecycledWaste);
    assert_eq!(game.waste.len(), 0);
    assert_eq!(game.stock.len(), 2);
    assert!(game.stock.iter().all(|c| !c.face_up));
}

#[test]
fn draw_three_moves_up_to_three_cards_from_stock() {
    let mut game = empty_game();
    game.set_draw_mode(DrawMode::Three);
    game.stock.push(card(Suit::Clubs, 1, false));
    game.stock.push(card(Suit::Diamonds, 2, false));
    game.stock.push(card(Suit::Hearts, 3, false));
    game.stock.push(card(Suit::Spades, 4, false));

    let result = game.draw_or_recycle();

    assert_eq!(result, DrawResult::DrewFromStock);
    assert_eq!(game.stock.len(), 1);
    assert_eq!(game.waste.len(), 3);
    assert!(game.waste.iter().all(|card| card.face_up));
}

#[test]
fn draw_three_with_low_stock_draws_remaining_cards_only() {
    let mut game = empty_game();
    game.set_draw_mode(DrawMode::Three);
    game.stock.push(card(Suit::Spades, 12, false));
    game.stock.push(card(Suit::Spades, 13, false));

    let result = game.draw_or_recycle();

    assert_eq!(result, DrawResult::DrewFromStock);
    assert_eq!(game.stock.len(), 0);
    assert_eq!(game.waste.len(), 2);
    assert!(game.waste.iter().all(|card| card.face_up));
}

#[test]
fn waste_to_foundation_requires_ace_then_next_rank_same_suit() {
    let mut game = empty_game();
    game.waste.push(card(Suit::Clubs, 2, true));
    assert!(!game.move_waste_to_foundation());

    game.waste.clear();
    game.waste.push(card(Suit::Clubs, 1, true));
    assert!(game.move_waste_to_foundation());
    assert_eq!(game.foundations[Suit::Clubs.foundation_index()].len(), 1);

    game.waste.push(card(Suit::Clubs, 2, true));
    assert!(game.move_waste_to_foundation());
    assert_eq!(game.foundations[Suit::Clubs.foundation_index()].len(), 2);

    game.waste.push(card(Suit::Spades, 3, true));
    assert!(!game.move_waste_to_foundation());
}

#[test]
fn waste_to_tableau_enforces_klondike_rules() {
    let mut game = empty_game();

    game.waste.push(card(Suit::Hearts, 13, true));
    assert!(game.move_waste_to_tableau(0));
    assert_eq!(game.tableau[0].len(), 1);

    game.waste.push(card(Suit::Diamonds, 12, true));
    assert!(!game.move_waste_to_tableau(0));

    game.waste.pop();
    game.waste.push(card(Suit::Spades, 12, true));
    assert!(game.move_waste_to_tableau(0));
    assert_eq!(game.tableau[0].len(), 2);
}

#[test]
fn tableau_move_flips_new_top_card() {
    let mut game = empty_game();
    game.tableau[0].push(card(Suit::Clubs, 6, false));
    game.tableau[0].push(card(Suit::Hearts, 5, true));
    game.tableau[1].push(card(Suit::Spades, 6, true));

    assert!(game.move_tableau_top_to_tableau(0, 1));
    assert!(game.tableau[0][0].face_up);
    assert_eq!(game.tableau[1].last().map(|c| c.rank), Some(5));
}

#[test]
fn tableau_to_foundation_rejects_face_down_cards() {
    let mut game = empty_game();
    game.tableau[0].push(card(Suit::Diamonds, 1, false));

    assert!(!game.move_tableau_top_to_foundation(0));
    assert!(game.foundations[Suit::Diamonds.foundation_index()].is_empty());
}

#[test]
fn tableau_run_move_requires_valid_face_up_sequence() {
    let mut game = empty_game();
    game.tableau[0].push(card(Suit::Spades, 9, false));
    game.tableau[0].push(card(Suit::Hearts, 8, true));
    game.tableau[0].push(card(Suit::Clubs, 7, true));
    game.tableau[1].push(card(Suit::Clubs, 9, true));

    assert!(game.can_move_tableau_run_to_tableau(0, 1, 1));
    assert!(game.move_tableau_run_to_tableau(0, 1, 1));
    assert_eq!(game.tableau[0].len(), 1);
    assert!(game.tableau[0][0].face_up);
    assert_eq!(game.tableau[1].len(), 3);
    assert_eq!(game.tableau[1][1].rank, 8);
    assert_eq!(game.tableau[1][2].rank, 7);
}

#[test]
fn tableau_run_move_rejects_invalid_start() {
    let mut game = empty_game();
    game.tableau[0].push(card(Suit::Spades, 9, true));
    game.tableau[0].push(card(Suit::Hearts, 8, true));
    game.tableau[0].push(card(Suit::Diamonds, 7, true));
    game.tableau[1].push(card(Suit::Diamonds, 10, true));

    assert!(!game.can_move_tableau_run_to_tableau(0, 0, 1));
    assert!(!game.move_tableau_run_to_tableau(0, 0, 1));
}

#[test]
fn rank_labels_are_correct() {
    assert_eq!(rank_label(1), "A");
    assert_eq!(rank_label(11), "J");
    assert_eq!(rank_label(12), "Q");
    assert_eq!(rank_label(13), "K");
    assert_eq!(rank_label(99), "?");
}

#[test]
fn winnability_marks_completed_game_as_won() {
    let mut game = empty_game();
    for suit in Suit::ALL {
        let foundation = &mut game.foundations[suit.foundation_index()];
        for rank in 1..=13 {
            foundation.push(card(suit, rank, true));
        }
    }

    let result = game.analyze_winnability(10);
    assert!(result.winnable);
    assert!(!result.hit_state_limit);
    assert_eq!(result.explored_states, 1);
}

#[test]
fn winnability_honors_state_limit() {
    let game = KlondikeGame::new_with_seed(7);
    let result = game.analyze_winnability(0);
    assert!(!result.winnable);
    assert!(result.hit_state_limit);
    assert_eq!(result.explored_states, 0);
}

#[test]
fn cyclone_shuffle_preserves_tableau_geometry_and_card_set() {
    let mut game = empty_game();
    game.tableau[0] = vec![card(Suit::Clubs, 2, true)];
    game.tableau[1] = vec![card(Suit::Hearts, 10, false), card(Suit::Spades, 5, true)];
    game.tableau[2] = vec![
        card(Suit::Diamonds, 9, false),
        card(Suit::Clubs, 4, false),
        card(Suit::Hearts, 3, true),
    ];
    game.tableau[3] = vec![];
    game.tableau[4] = vec![
        card(Suit::Spades, 8, false),
        card(Suit::Diamonds, 7, true),
        card(Suit::Clubs, 6, true),
    ];
    game.tableau[5] = vec![card(Suit::Hearts, 1, true), card(Suit::Spades, 13, true)];
    game.tableau[6] = vec![card(Suit::Diamonds, 12, false)];

    let before_geometry: Vec<(usize, usize)> = game
        .tableau
        .iter()
        .map(|pile| {
            let down = pile.iter().filter(|card| !card.face_up).count();
            let up = pile.iter().filter(|card| card.face_up).count();
            (down, up)
        })
        .collect();

    let mut before_cards: std::collections::HashMap<(Suit, u8), usize> =
        std::collections::HashMap::new();
    for card in game.tableau.iter().flat_map(|pile| pile.iter()) {
        *before_cards.entry((card.suit, card.rank)).or_insert(0) += 1;
    }

    let _ = game.cyclone_shuffle_tableau();

    let after_geometry: Vec<(usize, usize)> = game
        .tableau
        .iter()
        .map(|pile| {
            let down = pile.iter().filter(|card| !card.face_up).count();
            let up = pile.iter().filter(|card| card.face_up).count();
            (down, up)
        })
        .collect();
    assert_eq!(after_geometry, before_geometry);

    let mut after_cards: std::collections::HashMap<(Suit, u8), usize> =
        std::collections::HashMap::new();
    for card in game.tableau.iter().flat_map(|pile| pile.iter()) {
        *after_cards.entry((card.suit, card.rank)).or_insert(0) += 1;
    }
    assert_eq!(after_cards, before_cards);
}

#[test]
fn solver_line_maps_to_hint_line_for_valid_sequence() {
    let mut game = empty_game();
    game.set_draw_mode(DrawMode::One);
    game.stock.push(card(Suit::Clubs, 1, false));

    let line = vec![SolverMove::Draw, SolverMove::WasteToFoundation];
    let mapped = map_solver_line_to_hint_line(&game, &line);
    assert!(mapped.is_some());
    assert_eq!(mapped.as_ref().map(Vec::len), Some(2));
}

#[test]
fn solver_line_mapping_rejects_illegal_sequence() {
    let mut game = empty_game();
    game.waste.push(card(Suit::Clubs, 7, true));

    let line = vec![SolverMove::WasteToFoundation];
    let mapped = map_solver_line_to_hint_line(&game, &line);
    assert!(mapped.is_none());
}

#[test]
fn cyclone_shuffle_noops_for_tiny_tableau() {
    let mut game = empty_game();
    game.tableau[0].push(card(Suit::Clubs, 1, true));
    assert!(!game.cyclone_shuffle_tableau());
}

#[test]
fn game_mode_metadata_is_stable() {
    assert_eq!(GameMode::from_id("klondike"), Some(GameMode::Klondike));
    assert_eq!(GameMode::from_id("spider"), Some(GameMode::Spider));
    assert_eq!(GameMode::from_id("freecell"), Some(GameMode::Freecell));
    assert_eq!(GameMode::from_id("unknown"), None);

    assert_eq!(GameMode::Klondike.label(), "Klondike");
    assert_eq!(GameMode::Spider.label(), "Spider");
    assert_eq!(GameMode::Freecell.label(), "FreeCell");
    assert_eq!(GameMode::Klondike.emoji(), "🥇");
    assert_eq!(GameMode::Spider.emoji(), "🕷️");
    assert_eq!(GameMode::Freecell.emoji(), "🗽");
    assert!(GameMode::Klondike.engine_ready());
    assert!(GameMode::Spider.engine_ready());
    assert!(!GameMode::Freecell.engine_ready());
}

#[test]
fn spider_seeded_games_are_deterministic() {
    let a = SpiderGame::new_with_seed(1234);
    let b = SpiderGame::new_with_seed(1234);
    let c = SpiderGame::new_with_seed(1235);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn spider_setup_accounts_for_all_104_cards() {
    let game = SpiderGame::new_with_seed(7);
    let tableau_count: usize = game.tableau().iter().map(Vec::len).sum();
    let total = tableau_count + game.stock_len();
    assert_eq!(tableau_count, 54);
    assert_eq!(game.stock_len(), 50);
    assert_eq!(total, 104);
}

#[test]
fn spider_three_suit_setup_accounts_for_all_104_cards() {
    let game = SpiderGame::new_with_seed_and_mode(7, SpiderSuitMode::Three);
    let tableau_count: usize = game.tableau().iter().map(Vec::len).sum();
    let total = tableau_count + game.stock_len();
    assert_eq!(tableau_count, 54);
    assert_eq!(game.stock_len(), 50);
    assert_eq!(total, 104);
}

#[test]
fn spider_setup_has_expected_column_geometry() {
    let game = SpiderGame::new_with_seed(99);
    for col in 0..10 {
        let pile = &game.tableau()[col];
        let expected = if col < 4 { 6 } else { 5 };
        assert_eq!(pile.len(), expected);
        assert_eq!(
            pile.iter().filter(|card| card.face_up).count(),
            1,
            "column {col} should have exactly one face-up card"
        );
        assert!(pile.last().is_some_and(|card| card.face_up));
    }
}

#[test]
fn spider_deal_adds_one_face_up_card_per_column() {
    let mut game = SpiderGame::new_with_seed(1);
    assert!(game.can_deal_from_stock());
    assert!(game.deal_from_stock());
    assert_eq!(game.stock_len(), 40);
    for pile in game.tableau() {
        assert!(pile.last().is_some_and(|card| card.face_up));
    }
}

#[test]
fn spider_deal_allows_empty_tableau() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0].push(card(Suit::Spades, 13, true));
    for pile in &mut tableau[1..9] {
        pile.push(card(Suit::Spades, 12, true));
    }
    let stock = vec![card(Suit::Spades, 1, false); 10];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::One, stock, tableau, 0);
    assert!(game.can_deal_from_stock());
    assert!(game.deal_from_stock());
    assert_eq!(game.stock_len(), 0);
    assert_eq!(game.tableau()[9].len(), 1);
    assert!(game.tableau()[9][0].face_up);
}

#[test]
fn spider_move_run_moves_descending_sequence_and_flips_source() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![
        card(Suit::Spades, 9, false),
        card(Suit::Hearts, 8, true),
        card(Suit::Spades, 7, true),
    ];
    tableau[1] = vec![card(Suit::Diamonds, 9, true)];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Two, Vec::new(), tableau, 0);

    assert!(game.can_move_run(0, 1, 1));
    assert!(game.move_run(0, 1, 1));
    assert_eq!(game.tableau()[0].len(), 1);
    assert!(game.tableau()[0][0].face_up);
    assert_eq!(game.tableau()[1].len(), 3);
    assert_eq!(game.tableau()[1][1].rank, 8);
    assert_eq!(game.tableau()[1][2].rank, 7);
}

#[test]
fn spider_removes_completed_suited_king_to_ace_run() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![
        card(Suit::Spades, 13, true),
        card(Suit::Spades, 12, true),
        card(Suit::Spades, 11, true),
        card(Suit::Spades, 10, true),
        card(Suit::Spades, 9, true),
        card(Suit::Spades, 8, true),
        card(Suit::Spades, 7, true),
        card(Suit::Spades, 6, true),
        card(Suit::Spades, 5, true),
        card(Suit::Spades, 4, true),
        card(Suit::Spades, 3, true),
        card(Suit::Spades, 2, true),
        card(Suit::Spades, 1, true),
    ];
    tableau[1] = vec![card(Suit::Spades, 13, true)];

    let mut game = SpiderGame::debug_new(SpiderSuitMode::One, Vec::new(), tableau, 0);
    assert!(game.move_run(1, 0, 2));
    assert_eq!(game.completed_runs(), 1);
    assert!(game.tableau()[0].is_empty());
}

#[test]
fn spider_session_codec_round_trip_preserves_state() {
    let mut game = SpiderGame::new_with_seed_and_mode(777, SpiderSuitMode::Two);
    let _ = game.deal_from_stock();
    let encoded = game.encode_for_session();
    let decoded = SpiderGame::decode_from_session(&encoded).expect("decode spider session");
    assert_eq!(decoded, game);
}

#[test]
fn spider_session_codec_round_trip_preserves_three_suit_mode() {
    let mut game = SpiderGame::new_with_seed_and_mode(777, SpiderSuitMode::Three);
    let _ = game.deal_from_stock();
    let encoded = game.encode_for_session();
    let decoded =
        SpiderGame::decode_from_session(&encoded).expect("decode spider session with three suits");
    assert_eq!(decoded.suit_mode(), SpiderSuitMode::Three);
    assert_eq!(decoded, game);
}

// Spider rollout scaffolding stubs (see SPIDER_TEST_PLAN.md).
// Keep ignored until each case is implemented.

#[test]
fn spider_rule_001_valid_descending_same_suit_run_can_move() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![card(Suit::Spades, 9, true), card(Suit::Hearts, 8, true)];
    tableau[1] = vec![card(Suit::Clubs, 9, true)];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Four, Vec::new(), tableau, 0);

    assert!(game.can_move_run(0, 1, 1));
    assert!(game.move_run(0, 1, 1));
    assert_eq!(game.tableau()[0].len(), 1);
    assert_eq!(game.tableau()[1].len(), 2);
    assert_eq!(game.tableau()[1][0].rank, 9);
    assert_eq!(game.tableau()[1][1].rank, 8);
    assert!(game.tableau()[1][1].face_up);
}

#[test]
fn spider_rule_002_run_starting_on_face_down_card_is_rejected() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![card(Suit::Spades, 9, false), card(Suit::Hearts, 8, true)];
    tableau[1] = vec![card(Suit::Clubs, 10, true)];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Four, Vec::new(), tableau, 0);
    let before = game.clone();

    assert!(!game.can_move_run(0, 0, 1));
    assert!(!game.move_run(0, 0, 1));
    assert_eq!(game, before);
}

#[test]
fn spider_rule_003_run_containing_face_down_card_is_rejected() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![
        card(Suit::Spades, 9, true),
        card(Suit::Hearts, 8, false),
        card(Suit::Clubs, 7, true),
    ];
    tableau[1] = vec![card(Suit::Diamonds, 10, true)];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Four, Vec::new(), tableau, 0);
    let before = game.clone();

    assert!(!game.can_move_run(0, 0, 1));
    assert!(!game.move_run(0, 0, 1));
    assert_eq!(game, before);
}

#[test]
fn spider_rule_004_invalid_sequence_is_rejected() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![
        card(Suit::Spades, 9, true),
        card(Suit::Hearts, 7, true),
        card(Suit::Clubs, 6, true),
    ];
    tableau[1] = vec![card(Suit::Diamonds, 10, true)];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Four, Vec::new(), tableau, 0);
    let before = game.clone();

    assert!(!game.can_move_run(0, 0, 1));
    assert!(!game.move_run(0, 0, 1));
    assert_eq!(game, before);
}

#[test]
fn spider_rule_005_move_to_illegal_destination_is_rejected() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![card(Suit::Spades, 9, true), card(Suit::Hearts, 8, true)];
    tableau[1] = vec![card(Suit::Clubs, 12, true)];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Four, Vec::new(), tableau, 0);
    let before = game.clone();

    assert!(!game.can_move_run(0, 1, 1));
    assert!(!game.move_run(0, 1, 1));
    assert_eq!(game, before);
}

#[test]
fn spider_rule_006_move_to_empty_tableau_follows_spider_rules() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![card(Suit::Spades, 9, true), card(Suit::Hearts, 8, true)];
    // destination column 1 intentionally empty
    let mut game = SpiderGame::debug_new(SpiderSuitMode::Four, Vec::new(), tableau, 0);

    assert!(game.can_move_run(0, 1, 1));
    assert!(game.move_run(0, 1, 1));
    assert_eq!(game.tableau()[0].len(), 1);
    assert_eq!(game.tableau()[1].len(), 1);
    assert_eq!(game.tableau()[1][0].rank, 8);
    assert!(game.tableau()[1][0].face_up);
}

#[test]
fn spider_rule_007_completed_suit_run_is_extracted_correctly() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![
        card(Suit::Spades, 13, true),
        card(Suit::Spades, 12, true),
        card(Suit::Spades, 11, true),
        card(Suit::Spades, 10, true),
        card(Suit::Spades, 9, true),
        card(Suit::Spades, 8, true),
        card(Suit::Spades, 7, true),
        card(Suit::Spades, 6, true),
        card(Suit::Spades, 5, true),
        card(Suit::Spades, 4, true),
        card(Suit::Spades, 3, true),
        card(Suit::Spades, 2, true),
        card(Suit::Spades, 1, true),
    ];
    tableau[1] = vec![card(Suit::Spades, 13, true)];

    let mut game = SpiderGame::debug_new(SpiderSuitMode::One, Vec::new(), tableau, 0);
    assert!(game.move_run(1, 0, 2));
    assert_eq!(game.completed_runs(), 1);
    assert!(game.tableau()[0].is_empty());
}

#[test]
fn spider_rule_008_deal_allowed_with_empty_tableau() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    // Make col 9 empty; all others non-empty.
    for pile in &mut tableau[0..9] {
        pile.push(card(Suit::Spades, 12, true));
    }
    let stock = vec![card(Suit::Spades, 1, false); 10];
    let mut game = SpiderGame::debug_new(SpiderSuitMode::One, stock, tableau.clone(), 0);

    assert!(game.can_deal_from_stock());
    assert!(game.deal_from_stock());
    assert_eq!(game.stock_len(), 0);
    for col in 0..10 {
        let expected = tableau[col].len() + 1;
        assert_eq!(game.tableau()[col].len(), expected);
        assert!(game.tableau()[col].last().is_some_and(|card| card.face_up));
    }
}

#[test]
fn spider_rule_009_deal_adds_one_face_up_card_per_tableau_when_legal() {
    let mut game = SpiderGame::new_with_seed(123);
    let before_lengths: Vec<usize> = game.tableau().iter().map(Vec::len).collect();
    let before_stock = game.stock_len();

    assert!(game.can_deal_from_stock());
    assert!(game.deal_from_stock());
    assert_eq!(game.stock_len(), before_stock - 10);

    for (idx, pile) in game.tableau().iter().enumerate() {
        assert_eq!(pile.len(), before_lengths[idx] + 1);
        assert!(
            pile.last().is_some_and(|card| card.face_up),
            "column {idx} last dealt card should be face-up"
        );
    }
}

#[test]
fn spider_rule_010_win_condition_only_when_all_required_runs_complete() {
    let tableau_empty: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    let game_near_win = SpiderGame::debug_new(SpiderSuitMode::One, Vec::new(), tableau_empty, 7);
    assert!(!game_near_win.is_won());

    let tableau_empty: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    let game_full_win = SpiderGame::debug_new(SpiderSuitMode::One, Vec::new(), tableau_empty, 8);
    assert!(game_full_win.is_won());
}

#[test]
fn spider_cyclone_shuffle_preserves_tableau_geometry_and_card_set() {
    let mut game = SpiderGame::new_with_seed(1234);
    let before_stock = game.stock_len();
    let before_runs = game.completed_runs();
    let before_geometry: Vec<(usize, usize)> = game
        .tableau()
        .iter()
        .map(|pile| {
            let down = pile.iter().filter(|card| !card.face_up).count();
            let up = pile.len().saturating_sub(down);
            (down, up)
        })
        .collect();

    let mut before_cards: std::collections::HashMap<(Suit, u8), usize> =
        std::collections::HashMap::new();
    for card in game.tableau().iter().flat_map(|pile| pile.iter()) {
        *before_cards.entry((card.suit, card.rank)).or_insert(0) += 1;
    }

    assert!(game.cyclone_shuffle_tableau());

    let after_geometry: Vec<(usize, usize)> = game
        .tableau()
        .iter()
        .map(|pile| {
            let down = pile.iter().filter(|card| !card.face_up).count();
            let up = pile.len().saturating_sub(down);
            (down, up)
        })
        .collect();
    assert_eq!(after_geometry, before_geometry);
    assert_eq!(game.stock_len(), before_stock);
    assert_eq!(game.completed_runs(), before_runs);

    let mut after_cards: std::collections::HashMap<(Suit, u8), usize> =
        std::collections::HashMap::new();
    for card in game.tableau().iter().flat_map(|pile| pile.iter()) {
        *after_cards.entry((card.suit, card.rank)).or_insert(0) += 1;
    }
    assert_eq!(after_cards, before_cards);
}

#[test]
fn spider_cyclone_shuffle_noops_for_tiny_tableau() {
    let mut tableau: [Vec<Card>; 10] = std::array::from_fn(|_| Vec::new());
    tableau[0] = vec![card(Suit::Spades, 13, true)];
    let before = SpiderGame::debug_new(SpiderSuitMode::One, Vec::new(), tableau, 0);
    let mut game = before.clone();

    assert!(!game.cyclone_shuffle_tableau());
    assert_eq!(game, before);
}
