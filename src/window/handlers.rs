use super::*;

impl CardthropicWindow {
    #[allow(deprecated)]
    pub(super) fn setup_handlers(&self) {
        self.setup_primary_action_handlers();
        self.setup_robot_stop_capture_handler();
        self.setup_keyboard_navigation_handler();
        self.setup_seed_handlers();
        self.setup_board_click_handlers();
        self.setup_geometry_handlers();

        self.setup_board_color_dropdown();
        self.setup_game_mode_menu_item();
        self.setup_game_settings_menu();
        self.setup_drag_and_drop();
        self.setup_tableau_overflow_hints();
    }
}
