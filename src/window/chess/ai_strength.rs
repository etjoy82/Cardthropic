use super::super::{
    SETTINGS_KEY_CHESS_AI_STRENGTH, SETTINGS_KEY_CHESS_ROBOT_BLACK_AI_STRENGTH,
    SETTINGS_KEY_CHESS_ROBOT_WHITE_AI_STRENGTH, SETTINGS_KEY_CHESS_WAND_AI_STRENGTH,
    SETTINGS_KEY_CHESS_W_QUESTION_AI_STRENGTH,
};
use crate::engine::chess::ai::SearchLimits;
use crate::game::{ChessColor, ChessVariant};
use crate::CardthropicWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use std::rc::Rc;

const CHESS_AI_STRENGTH_FAST: &str = "fast";
const CHESS_AI_STRENGTH_BALANCED: &str = "balanced";
const CHESS_AI_STRENGTH_STRONG: &str = "strong";
const CHESS_AI_STRENGTH_ULTRA: &str = "ultra";
const CHESS_AI_STRENGTH_LUDICROUS: &str = "ludicrous";
const CHESS_AI_STRENGTH_OMEGA: &str = "omega";
const CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL: &str = "extra-dimensional";
const CHESS_AI_STRENGTH_INTERNATIONAL: &str = "international";
const CHESS_AI_STRENGTH_GRANDEUR: &str = "grandeur";

const CHESS_AI_STRENGTH_OPTIONS: [&str; 9] = [
    CHESS_AI_STRENGTH_FAST,
    CHESS_AI_STRENGTH_BALANCED,
    CHESS_AI_STRENGTH_STRONG,
    CHESS_AI_STRENGTH_ULTRA,
    CHESS_AI_STRENGTH_LUDICROUS,
    CHESS_AI_STRENGTH_OMEGA,
    CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL,
    CHESS_AI_STRENGTH_INTERNATIONAL,
    CHESS_AI_STRENGTH_GRANDEUR,
];

impl CardthropicWindow {
    pub(in crate::window) fn chess_time_budget_seconds_one_decimal(time_budget_ms: u64) -> f64 {
        ((time_budget_ms as f64) / 100.0).round() / 10.0
    }

    pub(in crate::window) fn chess_time_budget_seconds_label(time_budget_ms: u64) -> String {
        format!(
            "{:.1}s",
            Self::chess_time_budget_seconds_one_decimal(time_budget_ms)
        )
    }

    pub(in crate::window) fn chess_think_will_finish_label(
        &self,
        started_elapsed_seconds: u32,
        time_budget_ms: u64,
    ) -> String {
        let finish_seconds = (f64::from(started_elapsed_seconds)
            + Self::chess_time_budget_seconds_one_decimal(time_budget_ms))
        .max(0.0)
        .floor() as u32;
        crate::window::parsing::format_time(finish_seconds)
    }

    pub(in crate::window) fn chess_think_will_finish_suffix(
        &self,
        started_elapsed_seconds: u32,
        time_budget_ms: u64,
    ) -> String {
        let imp = self.imp();
        if imp.move_count.get() == 0 || !imp.timer_started.get() {
            String::new()
        } else {
            format!(
                " Will Finish {}.",
                self.chess_think_will_finish_label(started_elapsed_seconds, time_budget_ms)
            )
        }
    }

    fn normalize_chess_ai_strength(raw: &str) -> &'static str {
        match raw.trim().to_ascii_lowercase().as_str() {
            CHESS_AI_STRENGTH_FAST => CHESS_AI_STRENGTH_FAST,
            CHESS_AI_STRENGTH_STRONG => CHESS_AI_STRENGTH_STRONG,
            CHESS_AI_STRENGTH_ULTRA => CHESS_AI_STRENGTH_ULTRA,
            CHESS_AI_STRENGTH_LUDICROUS => CHESS_AI_STRENGTH_LUDICROUS,
            CHESS_AI_STRENGTH_OMEGA => CHESS_AI_STRENGTH_OMEGA,
            CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL | "extra_dimensional" | "extradimensional" => {
                CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL
            }
            CHESS_AI_STRENGTH_INTERNATIONAL => CHESS_AI_STRENGTH_INTERNATIONAL,
            CHESS_AI_STRENGTH_GRANDEUR => CHESS_AI_STRENGTH_GRANDEUR,
            _ => CHESS_AI_STRENGTH_BALANCED,
        }
    }

    fn chess_ai_strength_label(strength: &str) -> &'static str {
        match strength {
            CHESS_AI_STRENGTH_FAST => "Fast",
            CHESS_AI_STRENGTH_STRONG => "Strong",
            CHESS_AI_STRENGTH_ULTRA => "Ultra",
            CHESS_AI_STRENGTH_LUDICROUS => "Ludicrous",
            CHESS_AI_STRENGTH_OMEGA => "Omega",
            CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL => "Extra-Dimensional",
            CHESS_AI_STRENGTH_INTERNATIONAL => "International",
            CHESS_AI_STRENGTH_GRANDEUR => "Grandeur",
            _ => "Balanced",
        }
    }

    fn chess_ai_strength_triplet_classical(strength: &str) -> (u8, u64, u64) {
        match strength {
            CHESS_AI_STRENGTH_FAST => (3, 80, 100_000),
            CHESS_AI_STRENGTH_STRONG => (5, 320, 600_000),
            CHESS_AI_STRENGTH_ULTRA => (6, 900, 1_500_000),
            CHESS_AI_STRENGTH_LUDICROUS => (7, 1800, 4_000_000),
            CHESS_AI_STRENGTH_OMEGA => (8, 10_000, 20_000_000),
            CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL => (9, 30_000, 60_000_000),
            CHESS_AI_STRENGTH_INTERNATIONAL => (10, 60_000, 120_000_000),
            CHESS_AI_STRENGTH_GRANDEUR => (11, 120_000, 240_000_000),
            _ => (4, 160, 200_000),
        }
    }

    fn chess_ai_strength_triplet_atomic(strength: &str) -> (u8, u64, u64) {
        match strength {
            CHESS_AI_STRENGTH_FAST => (10, 80, 20_000_000),
            CHESS_AI_STRENGTH_STRONG => (14, 320, 80_000_000),
            CHESS_AI_STRENGTH_ULTRA => (16, 900, 225_000_000),
            CHESS_AI_STRENGTH_LUDICROUS => (20, 1800, 450_000_000),
            CHESS_AI_STRENGTH_OMEGA => (32, 10_000, 2_500_000_000),
            CHESS_AI_STRENGTH_EXTRA_DIMENSIONAL => (40, 30_000, 7_500_000_000),
            CHESS_AI_STRENGTH_INTERNATIONAL => (48, 60_000, 15_000_000_000),
            CHESS_AI_STRENGTH_GRANDEUR => (56, 120_000, 30_000_000_000),
            _ => (12, 160, 40_000_000),
        }
    }

    fn chess_ai_strength_triplet_for_variant(
        strength: &str,
        variant: ChessVariant,
    ) -> (u8, u64, u64) {
        match variant {
            ChessVariant::Atomic => Self::chess_ai_strength_triplet_atomic(strength),
            ChessVariant::Standard | ChessVariant::Chess960 => {
                Self::chess_ai_strength_triplet_classical(strength)
            }
        }
    }

    fn active_chess_variant_for_strength_profiles(&self) -> ChessVariant {
        if self.imp().chess_mode_active.get() {
            self.imp().chess_variant.get()
        } else {
            ChessVariant::Standard
        }
    }

    fn chess_ai_strength_triplet_for_active_variant(&self, strength: &str) -> (u8, u64, u64) {
        Self::chess_ai_strength_triplet_for_variant(
            strength,
            self.active_chess_variant_for_strength_profiles(),
        )
    }

    fn settings_has_chess_strength_key(settings: &gio::Settings, key: &str) -> bool {
        settings
            .settings_schema()
            .map(|schema| schema.has_key(key))
            .unwrap_or(false)
    }

    pub(in crate::window) fn settings_has_chess_ai_strength_key(settings: &gio::Settings) -> bool {
        Self::settings_has_chess_strength_key(settings, SETTINGS_KEY_CHESS_AI_STRENGTH)
    }

    pub(in crate::window) fn settings_has_chess_wand_ai_strength_key(
        settings: &gio::Settings,
    ) -> bool {
        Self::settings_has_chess_strength_key(settings, SETTINGS_KEY_CHESS_WAND_AI_STRENGTH)
    }

    pub(in crate::window) fn settings_has_chess_w_question_ai_strength_key(
        settings: &gio::Settings,
    ) -> bool {
        Self::settings_has_chess_strength_key(settings, SETTINGS_KEY_CHESS_W_QUESTION_AI_STRENGTH)
    }

    pub(in crate::window) fn settings_has_chess_robot_white_ai_strength_key(
        settings: &gio::Settings,
    ) -> bool {
        Self::settings_has_chess_strength_key(settings, SETTINGS_KEY_CHESS_ROBOT_WHITE_AI_STRENGTH)
    }

    pub(in crate::window) fn settings_has_chess_robot_black_ai_strength_key(
        settings: &gio::Settings,
    ) -> bool {
        Self::settings_has_chess_strength_key(settings, SETTINGS_KEY_CHESS_ROBOT_BLACK_AI_STRENGTH)
    }

    fn chess_strength_setting_for_key(&self, key: &str, fallback: &'static str) -> &'static str {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return fallback;
        };
        if !Self::settings_has_chess_strength_key(settings, key) {
            return fallback;
        }
        let raw = settings.string(key).to_string();
        Self::normalize_chess_ai_strength(&raw)
    }

    pub(in crate::window) fn chess_ai_strength_setting(&self) -> &'static str {
        self.chess_strength_setting_for_key(
            SETTINGS_KEY_CHESS_AI_STRENGTH,
            CHESS_AI_STRENGTH_BALANCED,
        )
    }

    pub(in crate::window) fn chess_wand_ai_strength_setting(&self) -> &'static str {
        self.chess_strength_setting_for_key(
            SETTINGS_KEY_CHESS_WAND_AI_STRENGTH,
            self.chess_ai_strength_setting(),
        )
    }

    pub(in crate::window) fn chess_w_question_ai_strength_setting(&self) -> &'static str {
        self.chess_strength_setting_for_key(
            SETTINGS_KEY_CHESS_W_QUESTION_AI_STRENGTH,
            self.chess_ai_strength_setting(),
        )
    }

    pub(in crate::window) fn chess_robot_white_ai_strength_setting(&self) -> &'static str {
        self.chess_strength_setting_for_key(
            SETTINGS_KEY_CHESS_ROBOT_WHITE_AI_STRENGTH,
            self.chess_ai_strength_setting(),
        )
    }

    pub(in crate::window) fn chess_robot_black_ai_strength_setting(&self) -> &'static str {
        self.chess_strength_setting_for_key(
            SETTINGS_KEY_CHESS_ROBOT_BLACK_AI_STRENGTH,
            self.chess_ai_strength_setting(),
        )
    }

    pub(in crate::window) fn chess_robot_ai_strength_setting_for_side(
        &self,
        side: ChessColor,
    ) -> &'static str {
        match side {
            ChessColor::White => self.chess_robot_white_ai_strength_setting(),
            ChessColor::Black => self.chess_robot_black_ai_strength_setting(),
        }
    }

    pub(in crate::window) fn chess_auto_response_ai_strength_label(&self) -> &'static str {
        Self::chess_ai_strength_label(self.chess_ai_strength_setting())
    }

    pub(in crate::window) fn chess_wand_ai_strength_label(&self) -> &'static str {
        Self::chess_ai_strength_label(self.chess_wand_ai_strength_setting())
    }

    pub(in crate::window) fn chess_w_question_ai_strength_label(&self) -> &'static str {
        Self::chess_ai_strength_label(self.chess_w_question_ai_strength_setting())
    }

    pub(in crate::window) fn chess_robot_ai_strength_label_for_side(
        &self,
        side: ChessColor,
    ) -> &'static str {
        Self::chess_ai_strength_label(self.chess_robot_ai_strength_setting_for_side(side))
    }

    pub(in crate::window) fn chess_auto_response_ai_search_limits(&self) -> SearchLimits {
        let (depth, time_ms, nodes) =
            self.chess_ai_strength_triplet_for_active_variant(self.chess_ai_strength_setting());
        SearchLimits::new(depth, time_ms, nodes)
    }

    pub(in crate::window) fn chess_wand_ai_search_limits(&self) -> SearchLimits {
        let (depth, time_ms, nodes) = self
            .chess_ai_strength_triplet_for_active_variant(self.chess_wand_ai_strength_setting());
        SearchLimits::new(depth, time_ms, nodes)
    }

    pub(in crate::window) fn chess_w_question_ai_search_limits(&self) -> SearchLimits {
        let (depth, time_ms, nodes) = self.chess_ai_strength_triplet_for_active_variant(
            self.chess_w_question_ai_strength_setting(),
        );
        SearchLimits::new(depth, time_ms, nodes)
    }

    pub(in crate::window) fn chess_robot_ai_search_limits_for_side(
        &self,
        side: ChessColor,
    ) -> SearchLimits {
        let (depth, time_ms, nodes) = self.chess_ai_strength_triplet_for_active_variant(
            self.chess_robot_ai_strength_setting_for_side(side),
        );
        SearchLimits::new(depth, time_ms, nodes)
    }

    pub(in crate::window) fn chess_ai_search_limits(&self) -> SearchLimits {
        self.chess_auto_response_ai_search_limits()
    }

    fn persist_chess_strength_setting_if_supported(&self, key: &str, normalized: &str) {
        if let Some(settings) = self.imp().settings.borrow().as_ref() {
            if Self::settings_has_chess_strength_key(settings, key)
                && settings.string(key).as_str() != normalized
            {
                let _ = settings.set_string(key, normalized);
            }
        }
    }

    pub(in crate::window) fn set_chess_auto_response_ai_strength_setting(
        &self,
        strength: &str,
        persist: bool,
        announce: bool,
    ) {
        let normalized = Self::normalize_chess_ai_strength(strength);
        if persist {
            self.persist_chess_strength_setting_if_supported(
                SETTINGS_KEY_CHESS_AI_STRENGTH,
                normalized,
            );
        }
        if announce {
            let (depth, time_ms, nodes) =
                self.chess_ai_strength_triplet_for_active_variant(normalized);
            *self.imp().status_override.borrow_mut() = Some(format!(
                "Auto-Response AI set to {} (depth={}, ply={}, time={}, nodes={}).",
                Self::chess_ai_strength_label(normalized),
                depth,
                depth,
                Self::chess_time_budget_seconds_label(time_ms),
                nodes
            ));
            self.render();
        }
    }

    pub(in crate::window) fn set_chess_wand_ai_strength_setting(
        &self,
        strength: &str,
        persist: bool,
        announce: bool,
    ) {
        let normalized = Self::normalize_chess_ai_strength(strength);
        if persist {
            self.persist_chess_strength_setting_if_supported(
                SETTINGS_KEY_CHESS_WAND_AI_STRENGTH,
                normalized,
            );
        }
        if announce {
            let (depth, time_ms, nodes) =
                self.chess_ai_strength_triplet_for_active_variant(normalized);
            *self.imp().status_override.borrow_mut() = Some(format!(
                "Your Wand AI set to {} (depth={}, ply={}, time={}, nodes={}).",
                Self::chess_ai_strength_label(normalized),
                depth,
                depth,
                Self::chess_time_budget_seconds_label(time_ms),
                nodes
            ));
            self.render();
        }
    }

    pub(in crate::window) fn set_chess_w_question_ai_strength_setting(
        &self,
        strength: &str,
        persist: bool,
        announce: bool,
    ) {
        let normalized = Self::normalize_chess_ai_strength(strength);
        if persist {
            self.persist_chess_strength_setting_if_supported(
                SETTINGS_KEY_CHESS_W_QUESTION_AI_STRENGTH,
                normalized,
            );
        }
        if announce {
            let (depth, time_ms, nodes) =
                self.chess_ai_strength_triplet_for_active_variant(normalized);
            *self.imp().status_override.borrow_mut() = Some(format!(
                "W? AI set to {} (depth={}, ply={}, time={}, nodes={}).",
                Self::chess_ai_strength_label(normalized),
                depth,
                depth,
                Self::chess_time_budget_seconds_label(time_ms),
                nodes
            ));
            self.render();
        }
    }

    pub(in crate::window) fn set_chess_robot_white_ai_strength_setting(
        &self,
        strength: &str,
        persist: bool,
        announce: bool,
    ) {
        let normalized = Self::normalize_chess_ai_strength(strength);
        if persist {
            self.persist_chess_strength_setting_if_supported(
                SETTINGS_KEY_CHESS_ROBOT_WHITE_AI_STRENGTH,
                normalized,
            );
        }
        if announce {
            let (depth, time_ms, nodes) =
                self.chess_ai_strength_triplet_for_active_variant(normalized);
            *self.imp().status_override.borrow_mut() = Some(format!(
                "Robot White AI set to {} (depth={}, ply={}, time={}, nodes={}).",
                Self::chess_ai_strength_label(normalized),
                depth,
                depth,
                Self::chess_time_budget_seconds_label(time_ms),
                nodes
            ));
            self.render();
        }
    }

    pub(in crate::window) fn set_chess_robot_black_ai_strength_setting(
        &self,
        strength: &str,
        persist: bool,
        announce: bool,
    ) {
        let normalized = Self::normalize_chess_ai_strength(strength);
        if persist {
            self.persist_chess_strength_setting_if_supported(
                SETTINGS_KEY_CHESS_ROBOT_BLACK_AI_STRENGTH,
                normalized,
            );
        }
        if announce {
            let (depth, time_ms, nodes) =
                self.chess_ai_strength_triplet_for_active_variant(normalized);
            *self.imp().status_override.borrow_mut() = Some(format!(
                "Robot Black AI set to {} (depth={}, ply={}, time={}, nodes={}).",
                Self::chess_ai_strength_label(normalized),
                depth,
                depth,
                Self::chess_time_budget_seconds_label(time_ms),
                nodes
            ));
            self.render();
        }
    }

    pub(in crate::window) fn set_chess_ai_strength_setting(
        &self,
        strength: &str,
        persist: bool,
        announce: bool,
    ) {
        self.set_chess_auto_response_ai_strength_setting(strength, persist, announce);
    }

    fn show_chess_strength_dialog_with<F>(
        &self,
        title: &str,
        heading_text: &str,
        body_text: &str,
        current: &'static str,
        on_pick: F,
    ) where
        F: Fn(&CardthropicWindow, &'static str) + 'static,
    {
        self.popdown_main_menu_later();

        let dialog = gtk::Window::builder()
            .title(title)
            .modal(true)
            .transient_for(self)
            .default_width(560)
            .default_height(300)
            .build();
        dialog.set_resizable(false);
        dialog.set_destroy_with_parent(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let heading = gtk::Label::new(Some(heading_text));
        heading.set_xalign(0.0);
        heading.add_css_class("title-4");
        root.append(&heading);

        let body = gtk::Label::new(Some(body_text));
        body.set_xalign(0.0);
        body.set_wrap(true);
        body.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        root.append(&body);

        let options_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let mut group_anchor: Option<gtk::CheckButton> = None;
        let on_pick: Rc<dyn Fn(&CardthropicWindow, &'static str)> = Rc::new(on_pick);
        let variant = self.active_chess_variant_for_strength_profiles();
        for strength in CHESS_AI_STRENGTH_OPTIONS {
            let (depth, time_ms, nodes) =
                Self::chess_ai_strength_triplet_for_variant(strength, variant);
            let label = format!(
                "{} (depth={}, ply={}, time={}, nodes={})",
                Self::chess_ai_strength_label(strength),
                depth,
                depth,
                Self::chess_time_budget_seconds_label(time_ms),
                nodes
            );
            let button = gtk::CheckButton::with_label(&label);
            if let Some(anchor) = group_anchor.as_ref() {
                button.set_group(Some(anchor));
            } else {
                group_anchor = Some(button.clone());
            }
            button.set_active(strength == current);
            let on_pick = on_pick.clone();
            button.connect_toggled(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |btn| {
                    if btn.is_active() {
                        on_pick(&window, strength);
                    }
                }
            ));
            options_box.append(&button);
        }
        root.append(&options_box);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);

        let close = gtk::Button::with_label("Close");
        close.add_css_class("suggested-action");
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions.append(&close);
        root.append(&actions);

        dialog.set_default_widget(Some(&close));
        let _ = close.grab_focus();
        dialog.set_child(Some(&root));
        dialog.present();
    }

    pub(in crate::window) fn show_chess_ai_strength_dialog(&self) {
        self.show_chess_strength_dialog_with(
            "Auto-Response AI Strength",
            "Auto-Response AI Strength",
            "Select search strength for automatic response moves in chess modes.",
            self.chess_ai_strength_setting(),
            |window, strength| {
                window.set_chess_auto_response_ai_strength_setting(strength, true, true);
            },
        );
    }

    pub(in crate::window) fn show_chess_w_question_ai_strength_dialog(&self) {
        self.show_chess_strength_dialog_with(
            "W? AI Strength",
            "W? AI Strength",
            "Select search strength used by W? chess analysis.",
            self.chess_w_question_ai_strength_setting(),
            |window, strength| {
                window.set_chess_w_question_ai_strength_setting(strength, true, true);
            },
        );
    }

    pub(in crate::window) fn show_chess_wand_ai_strength_dialog(&self) {
        self.show_chess_strength_dialog_with(
            "Your Wand AI Strength",
            "Your Wand AI Strength",
            "Select search strength when you click Wand Wave in chess modes.",
            self.chess_wand_ai_strength_setting(),
            |window, strength| {
                window.set_chess_wand_ai_strength_setting(strength, true, true);
            },
        );
    }

    pub(in crate::window) fn show_chess_robot_white_ai_strength_dialog(&self) {
        self.show_chess_strength_dialog_with(
            "Robot White AI Strength",
            "Robot White AI Strength",
            "Select search strength when Robot controls White in chess modes.",
            self.chess_robot_white_ai_strength_setting(),
            |window, strength| {
                window.set_chess_robot_white_ai_strength_setting(strength, true, true);
            },
        );
    }

    pub(in crate::window) fn show_chess_robot_black_ai_strength_dialog(&self) {
        self.show_chess_strength_dialog_with(
            "Robot Black AI Strength",
            "Robot Black AI Strength",
            "Select search strength when Robot controls Black in chess modes.",
            self.chess_robot_black_ai_strength_setting(),
            |window, strength| {
                window.set_chess_robot_black_ai_strength_setting(strength, true, true);
            },
        );
    }
}
