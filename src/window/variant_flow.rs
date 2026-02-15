use super::*;
use crate::engine::variant::{spec_for_id, spec_for_mode};
use crate::engine::variant_engine::engine_for_mode;

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

    pub(super) fn set_klondike_draw_mode(&self, draw_mode: DrawMode) {
        let imp = self.imp();
        if imp.klondike_draw_mode.get() == draw_mode {
            return;
        }
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
            let seed = imp.current_seed.get();
            self.start_new_game_with_seed(
                seed,
                format!(
                    "Spider suits {} selected. Redealt current seed {}.",
                    suit_mode.suit_count(),
                    seed
                ),
            );
        } else {
            self.update_game_settings_menu();
        }
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
        self.clear_hint_effects();
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        self.update_game_mode_menu_selection();
        self.update_game_settings_menu();
        *imp.status_override.borrow_mut() = Some(status);
        self.popdown_main_menu_later();
        self.render();
    }

    pub(super) fn mode_settings_spec(&self) -> &'static crate::engine::variant::VariantSpec {
        spec_for_mode(self.imp().current_game_mode.get())
    }
}
