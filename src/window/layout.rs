use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl CardthropicWindow {
    pub(super) fn update_tableau_metrics(&self) {
        let columns = match self.active_game_mode() {
            GameMode::Spider => 10,
            _ => 7,
        };
        let profile = self.workspace_layout_profile();

        let imp = self.imp();
        imp.tableau_row.set_spacing(profile.gap);
        let window_width = self.width().max(MIN_WINDOW_WIDTH);
        let window_height = self.height().max(MIN_WINDOW_HEIGHT);
        let column_gap = imp.tableau_row.spacing().max(0);
        let mut metrics_hasher = DefaultHasher::new();
        window_width.hash(&mut metrics_hasher);
        window_height.hash(&mut metrics_hasher);
        self.is_maximized().hash(&mut metrics_hasher);
        profile.side_padding.hash(&mut metrics_hasher);
        profile.tableau_vertical_padding.hash(&mut metrics_hasher);
        column_gap.hash(&mut metrics_hasher);
        profile.assumed_depth.hash(&mut metrics_hasher);
        profile.min_card_width.hash(&mut metrics_hasher);
        profile.max_card_width.hash(&mut metrics_hasher);
        profile.min_card_height.hash(&mut metrics_hasher);
        TABLEAU_FACE_UP_STEP_PX.hash(&mut metrics_hasher);
        TABLEAU_FACE_DOWN_STEP_PX.hash(&mut metrics_hasher);
        let metrics_key = metrics_hasher.finish();
        if metrics_key == imp.last_metrics_key.get() {
            return;
        }
        imp.last_metrics_key.set(metrics_key);

        let scroller_width = imp.tableau_scroller.width();
        let available_width = if scroller_width > 0 {
            (scroller_width - profile.side_padding).max(0)
        } else {
            (window_width - profile.side_padding * 2).max(0)
        };
        let slots = (available_width - column_gap * (columns - 1)).max(0);
        let width_limited_by_columns = if slots > 0 { slots / columns } else { 70 };
        let width_limited_by_top_row = self.max_card_width_for_top_row_fit(available_width);
        let reserve = self.vertical_layout_reserve(window_height);
        let usable_window_height = (window_height - reserve).max(220);
        let tableau_overhead = profile.tableau_vertical_padding + 12;
        let width_limited_by_window_height = self.max_card_width_for_window_height_fit(
            usable_window_height,
            profile,
            tableau_overhead,
        );

        let card_width = width_limited_by_columns
            .min(width_limited_by_top_row)
            .min(width_limited_by_window_height)
            .clamp(profile.min_card_width, profile.max_card_width);
        let card_height = (card_width * 108 / 70).max(profile.min_card_height);

        imp.card_width.set(card_width);
        imp.card_height.set(card_height);
        imp.face_up_step.set(TABLEAU_FACE_UP_STEP_PX);
        imp.face_down_step.set(TABLEAU_FACE_DOWN_STEP_PX);
    }

    fn workspace_layout_profile(&self) -> WorkspaceLayoutProfile {
        let window_width = self.width().max(1);
        let window_height = self.height().max(1);
        let preset = if window_height <= 600 {
            WorkspacePreset::Compact600
        } else if window_height <= 720 {
            WorkspacePreset::Hd720
        } else if window_height <= 1080 {
            WorkspacePreset::Fhd1080
        } else {
            WorkspacePreset::Qhd1440
        };

        let mut profile = match preset {
            WorkspacePreset::Compact600 => WorkspaceLayoutProfile {
                side_padding: 8,
                tableau_vertical_padding: 6,
                gap: 2,
                assumed_depth: 11,
                min_card_width: 14,
                max_card_width: 96,
                min_card_height: 24,
            },
            WorkspacePreset::Hd720 => WorkspaceLayoutProfile {
                side_padding: 12,
                tableau_vertical_padding: 8,
                gap: 3,
                assumed_depth: 12,
                min_card_width: 16,
                max_card_width: 118,
                min_card_height: 28,
            },
            WorkspacePreset::Fhd1080 => WorkspaceLayoutProfile {
                side_padding: 18,
                tableau_vertical_padding: 10,
                gap: 4,
                assumed_depth: 14,
                min_card_width: 18,
                max_card_width: 164,
                min_card_height: 32,
            },
            WorkspacePreset::Qhd1440 => WorkspaceLayoutProfile {
                side_padding: 24,
                tableau_vertical_padding: 14,
                gap: 6,
                assumed_depth: 14,
                min_card_width: 20,
                max_card_width: 240,
                min_card_height: 36,
            },
        };

        if self.is_maximized() {
            if window_height > 1080 {
                profile.max_card_width = profile.max_card_width.saturating_add(18);
            } else {
                profile.max_card_width = profile.max_card_width.saturating_add(6);
            }
        }

        if window_height <= 1080 && window_width >= 1600 {
            profile.max_card_width = profile.max_card_width.saturating_sub(16);
            profile.assumed_depth = profile.assumed_depth.saturating_add(1);
        }

        if window_width < window_height {
            profile.side_padding = (profile.side_padding / 2).max(6);
            profile.gap = (profile.gap - 1).max(2);
            profile.max_card_width = profile.max_card_width.saturating_sub(6);
        }

        profile
    }

    fn vertical_layout_reserve(&self, window_height: i32) -> i32 {
        if window_height <= 600 {
            196
        } else if window_height <= 720 {
            208
        } else if window_height <= 1080 {
            228
        } else {
            258
        }
    }

    fn max_card_width_for_window_height_fit(
        &self,
        usable_window_height: i32,
        profile: WorkspaceLayoutProfile,
        tableau_overhead: i32,
    ) -> i32 {
        let mut best = profile.min_card_width;
        let mut lo = profile.min_card_width;
        let mut hi = profile.max_card_width;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let card_height = (mid * 108 / 70).max(profile.min_card_height);
            let available_tableau_height =
                usable_window_height.saturating_sub(card_height + tableau_overhead);
            let tallest = self.tallest_tableau_height_with_steps(
                profile.assumed_depth,
                card_height,
                TABLEAU_FACE_UP_STEP_PX,
            );

            if available_tableau_height > 0 && tallest <= available_tableau_height {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        best
    }

    fn max_card_width_for_top_row_fit(&self, available_width: i32) -> i32 {
        let usable = available_width.saturating_sub(24).max(120);
        let mut best = 18;
        let mut lo = 18;
        let mut hi = 320;
        let spider_mode = self.active_game_mode() == GameMode::Spider;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let waste_step = (mid / 6).clamp(8, 22);
            let top_row_width = if spider_mode {
                // Spider hides foundations; only stock + waste need to fit.
                (2 * mid) + (4 * waste_step) + 32
            } else {
                (6 * mid) + (4 * waste_step) + 56
            };
            if top_row_width <= usable {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        best
    }

    fn tallest_tableau_height_with_steps(
        &self,
        assumed_depth: i32,
        card_height: i32,
        face_up_step: i32,
    ) -> i32 {
        let depth = assumed_depth.max(1);
        card_height + (depth - 1) * face_up_step.max(1)
    }
}
