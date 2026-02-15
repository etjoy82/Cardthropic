use super::*;

impl CardthropicWindow {
    pub(super) fn normalize_board_color(color: &str) -> gdk::RGBA {
        gdk::RGBA::parse(color)
            .or_else(|_| gdk::RGBA::parse(DEFAULT_BOARD_COLOR))
            .unwrap_or_else(|_| gdk::RGBA::new(0.12, 0.14, 0.17, 1.0))
    }

    pub(super) fn set_board_color(&self, color: &str, persist: bool) {
        let imp = self.imp();
        let normalized = Self::normalize_board_color(color);
        let stored_color = normalized.to_string();
        let css_color = normalized.to_string();

        *imp.board_color_hex.borrow_mut() = stored_color.clone();
        let preview = imp.board_color_preview.borrow().clone();
        if let Some(preview) = preview.as_ref() {
            preview.queue_draw();
        }

        if persist {
            let settings = imp.settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_BOARD_COLOR, &stored_color);
            }
        }

        if self.is_system_userstyle_active() {
            self.clear_board_color_override();
            return;
        }

        let existing_provider = imp.board_color_provider.borrow().clone();
        let provider = if let Some(provider) = existing_provider {
            provider
        } else {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &self.display(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
            *imp.board_color_provider.borrow_mut() = Some(provider.clone());
            provider
        };

        provider.load_from_string(&format!(
            ".board-background {{ background-color: {}; border-radius: 12px; transition: background-color 180ms ease-in-out; }}",
            css_color
        ));
    }

    pub(super) fn clear_board_color_override(&self) {
        if let Some(provider) = self.imp().board_color_provider.borrow().as_ref() {
            provider.load_from_string("");
        }
    }
}
