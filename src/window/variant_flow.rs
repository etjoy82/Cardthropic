use super::*;
use crate::engine::variant::spec_for_id;
use crate::engine::variant_engine::engine_for_mode;

fn mode_switch_pun(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Klondike => "🥇 Klondike mode: classic pressure, clean lines.",
        GameMode::Spider => "🕷️ Spider mode: one web, many traps.",
        GameMode::Freecell => "🗽 FreeCell mode: freedom through structure.",
    }
}

impl CardthropicWindow {
    pub(super) fn active_game_mode(&self) -> GameMode {
        self.imp().current_game_mode.get()
    }

    pub(super) fn current_klondike_draw_mode(&self) -> DrawMode {
        self.imp().klondike_draw_mode.get()
    }

    pub(super) fn current_spider_suit_mode(&self) -> SpiderSuitMode {
        self.imp().spider_suit_mode.get()
    }

    pub(super) fn current_freecell_card_count_mode(&self) -> FreecellCardCountMode {
        self.imp().freecell_card_count_mode.get()
    }

    pub(super) fn set_klondike_draw_mode(&self, draw_mode: DrawMode) {
        let imp = self.imp();
        if imp.klondike_draw_mode.get() == draw_mode {
            return;
        }
        let undo_anchor = self.snapshot();
        imp.klondike_draw_mode.set(draw_mode);
        let seed = imp.current_seed.get();
        self.start_new_game_with_seed(
            seed,
            format!(
                "Deal {} selected. Redealt current seed {}.",
                draw_mode.count(),
                seed
            ),
        );
        self.imp().history.borrow_mut().push(undo_anchor);
        self.imp().future.borrow_mut().clear();
        self.render();
    }

    pub(super) fn select_klondike_draw_mode(&self, draw_mode: DrawMode) {
        if self.active_game_mode() != GameMode::Klondike {
            self.select_game_mode("klondike");
        }
        if self.current_klondike_draw_mode() == draw_mode {
            let seed = self.imp().current_seed.get();
            self.start_new_game_with_seed(
                seed,
                format!(
                    "Deal {} selected. Redealt current seed {}.",
                    draw_mode.count(),
                    seed
                ),
            );
        } else {
            self.set_klondike_draw_mode(draw_mode);
        }
        self.set_ephemeral_status(
            mode_switch_pun(GameMode::Klondike),
            Duration::from_millis(2200),
        );
    }

    pub(super) fn set_spider_suit_mode(&self, suit_mode: SpiderSuitMode, persist: bool) {
        let imp = self.imp();
        if imp.spider_suit_mode.get() == suit_mode {
            return;
        }
        imp.spider_suit_mode.set(suit_mode);
        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_int(
                    SETTINGS_KEY_SPIDER_SUIT_MODE,
                    i32::from(suit_mode.suit_count()),
                );
            }
        }
        if self.active_game_mode() == GameMode::Spider {
            let undo_anchor = self.snapshot();
            let seed = imp.current_seed.get();
            self.start_new_game_with_seed(
                seed,
                format!(
                    "Spider suits {} selected. Redealt current seed {}.",
                    suit_mode.suit_count(),
                    seed
                ),
            );
            self.imp().history.borrow_mut().push(undo_anchor);
            self.imp().future.borrow_mut().clear();
            self.render();
        } else {
            self.update_game_settings_menu();
        }
    }

    pub(super) fn select_spider_suit_mode(&self, suit_mode: SpiderSuitMode) {
        if self.active_game_mode() != GameMode::Spider {
            self.select_game_mode("spider");
        }
        if self.current_spider_suit_mode() == suit_mode {
            let seed = self.imp().current_seed.get();
            self.start_new_game_with_seed(
                seed,
                format!(
                    "Spider suits {} selected. Redealt current seed {}.",
                    suit_mode.suit_count(),
                    seed
                ),
            );
        } else {
            self.set_spider_suit_mode(suit_mode, true);
        }
        self.set_ephemeral_status(
            mode_switch_pun(GameMode::Spider),
            Duration::from_millis(2200),
        );
    }

    pub(super) fn set_freecell_card_count_mode(
        &self,
        card_count_mode: FreecellCardCountMode,
        persist: bool,
    ) {
        let imp = self.imp();
        if imp.freecell_card_count_mode.get() == card_count_mode {
            return;
        }
        imp.freecell_card_count_mode.set(card_count_mode);
        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_int(
                    SETTINGS_KEY_FREECELL_CARD_COUNT_MODE,
                    i32::from(card_count_mode.card_count()),
                );
            }
        }
        if self.active_game_mode() == GameMode::Freecell {
            let undo_anchor = self.snapshot();
            let seed = imp.current_seed.get();
            self.start_new_game_with_seed(
                seed,
                format!(
                    "Card Count {} selected. Redealt current seed {}.",
                    card_count_mode.card_count(),
                    seed
                ),
            );
            self.imp().history.borrow_mut().push(undo_anchor);
            self.imp().future.borrow_mut().clear();
            self.render();
        } else {
            self.update_game_settings_menu();
        }
    }

    pub(super) fn select_freecell_card_count_mode(&self, card_count_mode: FreecellCardCountMode) {
        if self.active_game_mode() != GameMode::Freecell {
            self.select_game_mode("freecell");
        }
        if self.current_freecell_card_count_mode() == card_count_mode {
            let seed = self.imp().current_seed.get();
            self.start_new_game_with_seed(
                seed,
                format!(
                    "Card Count {} selected. Redealt current seed {}.",
                    card_count_mode.card_count(),
                    seed
                ),
            );
        } else {
            self.set_freecell_card_count_mode(card_count_mode, true);
        }
        self.set_ephemeral_status(
            mode_switch_pun(GameMode::Freecell),
            Duration::from_millis(2200),
        );
    }

    pub(super) fn is_mode_engine_ready(&self) -> bool {
        engine_for_mode(self.active_game_mode()).engine_ready()
    }

    pub(super) fn guard_mode_engine(&self, action: &str) -> bool {
        let spec = self.mode_spec();
        if spec.engine_ready {
            return true;
        }

        *self.imp().status_override.borrow_mut() = Some(format!(
            "{action} is not available in {} yet. Engine refactor in progress.",
            spec.label
        ));
        self.render();
        false
    }

    pub(super) fn select_game_mode(&self, mode: &str) {
        let imp = self.imp();
        let previous_mode = imp.current_game_mode.get();
        let undo_anchor = self.snapshot();
        self.stop_robot_mode();
        let status = match spec_for_id(mode) {
            Some(spec) => {
                imp.current_game_mode.set(spec.mode);
                if spec.mode == GameMode::Spider {
                    imp.spider_suit_mode
                        .set(imp.game.borrow().spider().suit_mode());
                }
                if spec.engine_ready {
                    format!("{} selected.", spec.label)
                } else {
                    format!(
                        "{} selected. Gameplay engine is being refactored for this mode.",
                        spec.label
                    )
                }
            }
            None => "Unknown game mode.".to_string(),
        };
        self.cancel_seed_winnable_check(None);
        *imp.selected_run.borrow_mut() = None;
        imp.selected_freecell.set(None);
        self.clear_hint_effects();
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        self.update_game_mode_menu_selection();
        self.update_game_settings_menu();
        *imp.status_override.borrow_mut() = Some(status);
        if imp.current_game_mode.get() != previous_mode {
            imp.history.borrow_mut().push(undo_anchor);
            imp.future.borrow_mut().clear();
            // Hard reset geometry-sensitive caches on mode transitions so the
            // next paint recomputes all width/spacing requests.
            imp.last_metrics_key.set(0);
            imp.last_stock_waste_foundation_size
                .set((0, 0, imp.current_game_mode.get()));
            self.handle_window_geometry_change();
            self.set_ephemeral_status(
                mode_switch_pun(imp.current_game_mode.get()),
                Duration::from_millis(2200),
            );
        }
        self.popdown_main_menu_later();
        self.render();
    }
}
