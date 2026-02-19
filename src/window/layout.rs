use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl CardthropicWindow {
    pub(super) fn update_tableau_metrics(&self) {
        let columns = match self.active_game_mode() {
            GameMode::Spider => 10,
            GameMode::Freecell => 8,
            _ => 7,
        };
        let mobile_phone_mode = self.imp().mobile_phone_mode.get();
        let mut profile = self.workspace_layout_profile();

        let imp = self.imp();
        imp.tableau_row.set_spacing(profile.gap);
        let window_width = self.width().max(MIN_WINDOW_WIDTH);
        let window_height = self.height().max(MIN_WINDOW_HEIGHT);
        let column_gap = imp.tableau_row.spacing().max(0);
        let scroller_width = imp.tableau_scroller.width();
        if mobile_phone_mode && scroller_width > 0 {
            imp.observed_scroller_width.set(scroller_width);
        }

        // Compute available_width before the hash so the proportional max_card_width
        // override is included in the hash key (preventing stale cache on resize).
        // On mobile-phone mode, derive from the scroller width to avoid slight
        // overflow that causes horizontal scrolling at tiny sizes.
        let window_budget = (window_width - profile.side_padding * 2).max(0);
        let mut available_width = if mobile_phone_mode {
            let scroller_budget = if scroller_width > 0 {
                scroller_width
            } else {
                let observed = imp.observed_scroller_width.get();
                if observed > 0 {
                    observed
                } else {
                    window_budget
                }
            };
            // Clamp to current window budget so stale observed widths cannot cause overflow.
            scroller_budget.clamp(0, window_budget)
        } else {
            window_budget
        };
        if mobile_phone_mode {
            // Keep only a minimal safety margin; mobile should prioritize
            // filling the viewport instead of leaving visible slack.
            let safety = 2;
            available_width = (available_width - safety).max(0);
        }

        // Allow cards to scale proportionally with screen width beyond preset limits.
        // Without this, any display taller than 1080p is capped at the Qhd1440 max
        // (~240px), producing postage-stamp cards on 4K and 8K screens.
        let proportional_max = (available_width / columns.max(1)).min(900);
        if proportional_max > profile.max_card_width {
            profile.max_card_width = proportional_max;
        }

        let mut metrics_hasher = DefaultHasher::new();
        window_width.hash(&mut metrics_hasher);
        window_height.hash(&mut metrics_hasher);
        self.active_game_mode().hash(&mut metrics_hasher);
        columns.hash(&mut metrics_hasher);
        self.is_maximized().hash(&mut metrics_hasher);
        profile.side_padding.hash(&mut metrics_hasher);
        profile.tableau_vertical_padding.hash(&mut metrics_hasher);
        column_gap.hash(&mut metrics_hasher);
        profile.assumed_depth.hash(&mut metrics_hasher);
        profile.min_card_width.hash(&mut metrics_hasher);
        profile.max_card_width.hash(&mut metrics_hasher);
        profile.min_card_height.hash(&mut metrics_hasher);
        mobile_phone_mode.hash(&mut metrics_hasher);
        imp.observed_scroller_width.get().hash(&mut metrics_hasher);
        TABLEAU_FACE_UP_STEP_PX.hash(&mut metrics_hasher);
        TABLEAU_FACE_DOWN_STEP_PX.hash(&mut metrics_hasher);
        let metrics_key = metrics_hasher.finish();
        if metrics_key == imp.last_metrics_key.get() {
            return;
        }
        imp.last_metrics_key.set(metrics_key);

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

        let mut card_width = width_limited_by_columns
            .min(width_limited_by_top_row)
            .min(width_limited_by_window_height)
            .clamp(profile.min_card_width, profile.max_card_width);
        if mobile_phone_mode {
            // Fill-bias for tiny layouts: use the full computed fit budget
            // instead of conservative rounding that leaves visible slack.
            let mobile_max_fit = width_limited_by_columns
                .min(width_limited_by_top_row)
                .min(width_limited_by_window_height)
                .clamp(profile.min_card_width, profile.max_card_width);
            card_width = card_width.max(mobile_max_fit);
        } else {
            // Keep even desktop widths for crispness and consistency.
            card_width = (card_width - (card_width % 2)).max(profile.min_card_width);
        }
        let card_height = (card_width * 108 / 70).max(profile.min_card_height);
        let (face_up_step, face_down_step) = if mobile_phone_mode {
            Self::tableau_steps_for_mobile_card_height(card_height)
        } else {
            Self::tableau_steps_for_card_height(card_height)
        };
        let used_tableau_width = card_width * columns + column_gap * (columns - 1);
        let overflow_delta = used_tableau_width - available_width;

        let stock_live_w = imp.stock_picture.width();
        let stock_live_h = imp.stock_picture.height();
        let waste_live_w = imp.waste_picture.width();
        let waste_live_h = imp.waste_picture.height();
        let waste_overlay_live_w = imp.waste_overlay.width();
        let waste_overlay_live_h = imp.waste_overlay.height();
        let stock_req_w = imp.stock_picture.width_request();
        let stock_req_h = imp.stock_picture.height_request();
        let waste_req_w = imp.waste_picture.width_request();
        let waste_req_h = imp.waste_picture.height_request();
        let waste_overlay_req_w = imp.waste_overlay.width_request();
        let waste_overlay_req_h = imp.waste_overlay.height_request();
        let visible_foundation_slots = self
            .foundation_pictures()
            .iter()
            .filter(|picture| picture.is_visible())
            .count();
        let expected_foundation_slots = match self.active_game_mode() {
            GameMode::Spider => 8usize,
            GameMode::Klondike | GameMode::Freecell => 4usize,
        };
        let foundation_slots_ok = visible_foundation_slots == expected_foundation_slots;
        let mode_label = self.active_game_mode().label();

        imp.card_width.set(card_width);
        imp.card_height.set(card_height);
        imp.face_up_step.set(face_up_step);
        imp.face_down_step.set(face_down_step);

        self.append_layout_debug_history_line(&format!(
            "mode={} mobile={} win={}x{} scroll_live={} scroll_obs={} avail_w={} used_w={} overflow_delta={} cols={} gap={} card={}x{} topcap={} colcap={} hcap={} stock_live={}x{} stock_req={}x{} waste_live={}x{} waste_req={}x{} waste_overlay_live={}x{} waste_overlay_req={}x{} fslots_vis={} fslots_exp={} fslots_ok={}",
            mode_label,
            mobile_phone_mode,
            window_width,
            window_height,
            scroller_width,
            imp.observed_scroller_width.get(),
            available_width,
            used_tableau_width,
            overflow_delta,
            columns,
            column_gap,
            card_width,
            card_height,
            width_limited_by_top_row,
            width_limited_by_columns,
            width_limited_by_window_height,
            stock_live_w,
            stock_live_h,
            stock_req_w,
            stock_req_h,
            waste_live_w,
            waste_live_h,
            waste_req_w,
            waste_req_h,
            waste_overlay_live_w,
            waste_overlay_live_h,
            waste_overlay_req_w,
            waste_overlay_req_h,
            visible_foundation_slots,
            expected_foundation_slots,
            foundation_slots_ok
        ));
    }

    fn workspace_layout_profile(&self) -> WorkspaceLayoutProfile {
        let window_width = self.width().max(1);
        let window_height = self.height().max(1);
        if self.imp().mobile_phone_mode.get() {
            return WorkspaceLayoutProfile {
                side_padding: 0,
                tableau_vertical_padding: 1,
                gap: 1,
                assumed_depth: 6,
                min_card_width: 18,
                max_card_width: 64,
                min_card_height: 28,
            };
        }
        let preset = if window_height <= 600 {
            WorkspacePreset::Compact600
        } else if window_height <= 720 {
            WorkspacePreset::Hd720
        } else if window_height <= 1080 {
            WorkspacePreset::Fhd1080
        } else if window_height <= 1440 {
            WorkspacePreset::Qhd1440
        } else if window_height <= 2160 {
            WorkspacePreset::FourK2160
        } else {
            WorkspacePreset::EightK4320
        };

        let mut profile = match preset {
            WorkspacePreset::Compact600 => WorkspaceLayoutProfile {
                side_padding: 8,
                tableau_vertical_padding: 6,
                gap: 2,
                // 5: initial Klondike deal peaks at 5 visible cards on small screens.
                // The tableau scroller handles overflow for deeper game states.
                assumed_depth: 5,
                min_card_width: 14,
                max_card_width: 96,
                min_card_height: 24,
            },
            WorkspacePreset::Hd720 => WorkspaceLayoutProfile {
                side_padding: 12,
                tableau_vertical_padding: 8,
                gap: 3,
                assumed_depth: 6,
                min_card_width: 16,
                max_card_width: 118,
                min_card_height: 28,
            },
            WorkspacePreset::Fhd1080 => WorkspaceLayoutProfile {
                side_padding: 18,
                tableau_vertical_padding: 10,
                gap: 4,
                // 7: fits the full initial Klondike deal (7 columns, deepest has 7 cards)
                // without scrolling.  Deeper mid-game stacks use the vertical scroller.
                assumed_depth: 7,
                min_card_width: 18,
                max_card_width: 164,
                min_card_height: 32,
            },
            WorkspacePreset::Qhd1440 => WorkspaceLayoutProfile {
                side_padding: 24,
                tableau_vertical_padding: 14,
                gap: 6,
                assumed_depth: 8,
                min_card_width: 20,
                max_card_width: 240,
                min_card_height: 36,
            },
            // 4K logical resolution (e.g. 3840x2160 at 1x DPI or 5120x2880 at 1.33x).
            // Padding and gap scale proportionally; max_card_width is overridden in
            // update_tableau_metrics by the proportional formula.
            WorkspacePreset::FourK2160 => WorkspaceLayoutProfile {
                side_padding: 36,
                tableau_vertical_padding: 20,
                gap: 9,
                assumed_depth: 14,
                min_card_width: 28,
                max_card_width: 400,
                min_card_height: 52,
            },
            // 8K logical resolution (e.g. 7680x4320 at 1x DPI).
            // max_card_width is overridden in update_tableau_metrics.
            WorkspacePreset::EightK4320 => WorkspaceLayoutProfile {
                side_padding: 64,
                tableau_vertical_padding: 32,
                gap: 16,
                assumed_depth: 14,
                min_card_width: 48,
                max_card_width: 600,
                min_card_height: 88,
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
        }

        if window_width < window_height {
            profile.side_padding = (profile.side_padding / 2).max(6);
            profile.gap = (profile.gap - 1).max(2);
            profile.max_card_width = profile.max_card_width.saturating_sub(6);
        }

        profile
    }

    fn vertical_layout_reserve(&self, window_height: i32) -> i32 {
        // When the HUD is hidden the toolbar_box is not visible, so its height
        // is no longer part of the layout.  Return a small constant to account
        // only for the window chrome (title bar, etc.).
        if !self.imp().hud_enabled.get() {
            return 60;
        }
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
            let (face_up_step, _) = Self::tableau_steps_for_card_height(card_height);
            let available_tableau_height =
                usable_window_height.saturating_sub(card_height + tableau_overhead);
            let tallest = self.tallest_tableau_height_with_steps(
                profile.assumed_depth,
                card_height,
                face_up_step,
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
        // Derive the binary search ceiling from available width rather than a fixed 320px.
        // The old ceiling caused the top-row constraint to cap cards at 320px on 4K/8K
        // displays even when the top row had more than enough space for larger cards.
        let mut hi = (usable / 4).min(900);
        let mode = self.active_game_mode();
        let spider_mode = mode == GameMode::Spider;
        let freecell_mode = mode == GameMode::Freecell;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let waste_step = (mid / 6).clamp(8, 22);
            let top_row_width = if spider_mode {
                // Spider hides foundations; only stock + waste need to fit.
                (2 * mid) + (4 * waste_step) + 32
            } else if freecell_mode {
                // FreeCell top row is free-cell strip (4 cards) + foundation strip (4 cards).
                // The old estimate reused Klondike's waste fan math and could undercount
                // width by a large margin, allowing oversized cards on resize.
                (8 * mid) + 96
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

    fn tableau_steps_for_card_height(card_height: i32) -> (i32, i32) {
        // Steps scale proportionally with card height.  The old 44px / 24px upper caps
        // caused two problems at 4K/8K: the rendered peeking was far too tight relative
        // to the card size, and the height binary search underestimated column height
        // (step too small → more room than actually exists → allowed cards that would
        // overflow in practice).  Removing the caps lets the height constraint correctly
        // reject oversized cards and keeps the visual peek ratio consistent at all DPIs.
        let face_up_step = (card_height * 24 / 108).max(TABLEAU_FACE_UP_STEP_PX);
        let face_down_step = (face_up_step * 11 / 24).max(TABLEAU_FACE_DOWN_STEP_PX);
        (face_up_step, face_down_step)
    }

    fn tableau_steps_for_mobile_card_height(card_height: i32) -> (i32, i32) {
        // Mobile mode needs denser stacks for readability in small windows.
        // Keep peeking visible, but reduce vertical spread relative to desktop.
        let face_up_step = (card_height * 16 / 108).clamp(5, 12);
        let face_down_step = (face_up_step * 7 / 16).clamp(3, 8);
        (face_up_step, face_down_step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desktop_columns(mode: GameMode) -> i32 {
        match mode {
            GameMode::Spider => 10,
            GameMode::Freecell => 8,
            _ => 7,
        }
    }

    fn desktop_vertical_layout_reserve(window_height: i32) -> i32 {
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

    fn desktop_workspace_profile(
        window_width: i32,
        window_height: i32,
        is_maximized: bool,
    ) -> WorkspaceLayoutProfile {
        let preset = if window_height <= 600 {
            WorkspacePreset::Compact600
        } else if window_height <= 720 {
            WorkspacePreset::Hd720
        } else if window_height <= 1080 {
            WorkspacePreset::Fhd1080
        } else if window_height <= 1440 {
            WorkspacePreset::Qhd1440
        } else if window_height <= 2160 {
            WorkspacePreset::FourK2160
        } else {
            WorkspacePreset::EightK4320
        };

        let mut profile = match preset {
            WorkspacePreset::Compact600 => WorkspaceLayoutProfile {
                side_padding: 8,
                tableau_vertical_padding: 6,
                gap: 2,
                assumed_depth: 5,
                min_card_width: 14,
                max_card_width: 96,
                min_card_height: 24,
            },
            WorkspacePreset::Hd720 => WorkspaceLayoutProfile {
                side_padding: 12,
                tableau_vertical_padding: 8,
                gap: 3,
                assumed_depth: 6,
                min_card_width: 16,
                max_card_width: 118,
                min_card_height: 28,
            },
            WorkspacePreset::Fhd1080 => WorkspaceLayoutProfile {
                side_padding: 18,
                tableau_vertical_padding: 10,
                gap: 4,
                assumed_depth: 7,
                min_card_width: 18,
                max_card_width: 164,
                min_card_height: 32,
            },
            WorkspacePreset::Qhd1440 => WorkspaceLayoutProfile {
                side_padding: 24,
                tableau_vertical_padding: 14,
                gap: 6,
                assumed_depth: 8,
                min_card_width: 20,
                max_card_width: 240,
                min_card_height: 36,
            },
            WorkspacePreset::FourK2160 => WorkspaceLayoutProfile {
                side_padding: 36,
                tableau_vertical_padding: 20,
                gap: 9,
                assumed_depth: 14,
                min_card_width: 28,
                max_card_width: 400,
                min_card_height: 52,
            },
            WorkspacePreset::EightK4320 => WorkspaceLayoutProfile {
                side_padding: 64,
                tableau_vertical_padding: 32,
                gap: 16,
                assumed_depth: 14,
                min_card_width: 48,
                max_card_width: 600,
                min_card_height: 88,
            },
        };

        if is_maximized {
            if window_height > 1080 {
                profile.max_card_width = profile.max_card_width.saturating_add(18);
            } else {
                profile.max_card_width = profile.max_card_width.saturating_add(6);
            }
        }

        if window_height <= 1080 && window_width >= 1600 {
            profile.max_card_width = profile.max_card_width.saturating_sub(16);
        }

        if window_width < window_height {
            profile.side_padding = (profile.side_padding / 2).max(6);
            profile.gap = (profile.gap - 1).max(2);
            profile.max_card_width = profile.max_card_width.saturating_sub(6);
        }

        profile
    }

    fn top_row_width(mode: GameMode, card_width: i32) -> i32 {
        let waste_step = (card_width / 6).clamp(8, 22);
        match mode {
            GameMode::Spider => (2 * card_width) + (4 * waste_step) + 32,
            GameMode::Freecell => (8 * card_width) + 96,
            _ => (6 * card_width) + (4 * waste_step) + 56,
        }
    }

    fn max_card_width_for_top_row_fit(mode: GameMode, available_width: i32) -> i32 {
        let usable = available_width.saturating_sub(24).max(120);
        let mut best = 18;
        let mut lo = 18;
        let mut hi = (usable / 4).min(900);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            if top_row_width(mode, mid) <= usable {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        best
    }

    fn max_card_width_for_window_height_fit(
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
            let (face_up_step, _) = CardthropicWindow::tableau_steps_for_card_height(card_height);
            let available_tableau_height =
                usable_window_height.saturating_sub(card_height + tableau_overhead);
            let depth = profile.assumed_depth.max(1);
            let tallest = card_height + (depth - 1) * face_up_step.max(1);

            if available_tableau_height > 0 && tallest <= available_tableau_height {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        best
    }

    fn compute_desktop_metrics(
        mode: GameMode,
        window_width: i32,
        window_height: i32,
        is_maximized: bool,
    ) -> (i32, i32, i32, i32) {
        let columns = desktop_columns(mode);
        let mut profile = desktop_workspace_profile(window_width, window_height, is_maximized);
        let available_width = (window_width - profile.side_padding * 2).max(0);
        let proportional_max = (available_width / columns.max(1)).min(900);
        if proportional_max > profile.max_card_width {
            profile.max_card_width = proportional_max;
        }

        let column_gap = profile.gap.max(0);
        let slots = (available_width - column_gap * (columns - 1)).max(0);
        let width_limited_by_columns = if slots > 0 { slots / columns } else { 70 };
        let width_limited_by_top_row = max_card_width_for_top_row_fit(mode, available_width);
        let reserve = desktop_vertical_layout_reserve(window_height);
        let usable_window_height = (window_height - reserve).max(220);
        let tableau_overhead = profile.tableau_vertical_padding + 12;
        let width_limited_by_window_height =
            max_card_width_for_window_height_fit(usable_window_height, profile, tableau_overhead);

        let mut card_width = width_limited_by_columns
            .min(width_limited_by_top_row)
            .min(width_limited_by_window_height)
            .clamp(profile.min_card_width, profile.max_card_width);
        card_width = (card_width - (card_width % 2)).max(profile.min_card_width);

        let used_tableau_width = card_width * columns + column_gap * (columns - 1);
        (card_width, used_tableau_width, available_width, column_gap)
    }

    #[test]
    fn desktop_layout_no_horizontal_tableau_overflow() {
        let cases = [
            (800, 600, false),
            (1280, 720, false),
            (1920, 1080, false),
            (1920, 1080, true),
            (2560, 1440, true),
        ];
        let modes = [GameMode::Klondike, GameMode::Spider, GameMode::Freecell];

        for mode in modes {
            for (w, h, maximized) in cases {
                let (card_width, used, available, gap) =
                    compute_desktop_metrics(mode, w, h, maximized);
                assert!(
                    used <= available,
                    "desktop overflow: mode={mode:?} win={}x{} maximized={} card_width={} gap={} used={} available={}",
                    w,
                    h,
                    maximized,
                    card_width,
                    gap,
                    used,
                    available
                );
            }
        }
    }

    #[test]
    fn desktop_layout_top_row_fits_for_all_modes() {
        let cases = [
            (800, 600, false),
            (1280, 720, false),
            (1920, 1080, false),
            (2560, 1440, true),
        ];
        let modes = [GameMode::Klondike, GameMode::Spider, GameMode::Freecell];

        for mode in modes {
            for (w, h, maximized) in cases {
                let (card_width, _, available, _) = compute_desktop_metrics(mode, w, h, maximized);
                let usable = available.saturating_sub(24).max(120);
                let needed = top_row_width(mode, card_width);
                assert!(
                    needed <= usable,
                    "top-row overflow: mode={mode:?} win={}x{} maximized={} card_width={} needed={} usable={}",
                    w,
                    h,
                    maximized,
                    card_width,
                    needed,
                    usable
                );
            }
        }
    }
}
