/* application.rs
 *
 * Copyright 2026 emviolet
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::startup_trace;
use crate::CardthropicWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct CardthropicApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for CardthropicApplication {
        const NAME: &'static str = "CardthropicApplication";
        type Type = super::CardthropicApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for CardthropicApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.new-window", &["<primary>n"]);
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
            obj.set_accels_for_action("win.command-search", &["slash", "KP_Divide"]);
            obj.set_accels_for_action("win.help", &["F1"]);
            obj.set_accels_for_action("win.random-seed", &["<primary>r"]);
            obj.set_accels_for_action("win.winnable-seed", &["<primary><shift>r"]);
            obj.set_accels_for_action("win.seed-picker", &["<primary>l"]);
            obj.set_accels_for_action("win.repeat-seed", &["<primary><shift>g"]);
            obj.set_accels_for_action("win.check-seed-winnable", &["F7"]);
            obj.set_accels_for_action("win.draw", &["<primary>d"]);
            obj.set_accels_for_action("win.undo", &["<primary>z"]);
            obj.set_accels_for_action("win.redo", &["<primary>y"]);
            obj.set_accels_for_action("win.toggle-fullscreen", &["F11"]);
            obj.set_accels_for_action("win.play-hint-move", &["<primary>space"]);
            obj.set_accels_for_action("win.rapid-wand", &["<primary><shift>space"]);
            obj.set_accels_for_action("win.peek", &["F3"]);
            obj.set_accels_for_action("win.robot-mode", &["F6"]);
            obj.set_accels_for_action("win.forever-mode", &["backslash"]);
            obj.set_accels_for_action("win.robot-auto-new-game-on-loss", &["F9"]);
            obj.set_accels_for_action("win.robot-strict-debug-invariants", &["F10"]);
            obj.set_accels_for_action("win.ludicrous-speed", &["equal"]);
            obj.set_accels_for_action("win.copy-benchmark-snapshot", &["<primary><shift>b"]);
            obj.set_accels_for_action("win.cyclone-shuffle", &["F5"]);
            obj.set_accels_for_action("win.enable-hud", &["grave"]);
            obj.set_accels_for_action("win.copy-game-state", &["<primary><shift>c"]);
            obj.set_accels_for_action("win.paste-game-state", &["<primary><shift>v"]);
            obj.set_accels_for_action("win.insert-note", &["<primary><shift>n"]);
            obj.set_accels_for_action("win.mode-klondike-deal-1", &["<shift>1"]);
            obj.set_accels_for_action("win.mode-klondike-deal-2", &["<shift>2"]);
            obj.set_accels_for_action("win.mode-klondike-deal-3", &["<shift>3"]);
            obj.set_accels_for_action("win.mode-klondike-deal-4", &["<shift>4"]);
            obj.set_accels_for_action("win.mode-klondike-deal-5", &["<shift>5"]);
            obj.set_accels_for_action("win.mode-spider-suit-1", &["<primary>1"]);
            obj.set_accels_for_action("win.mode-spider-suit-2", &["<primary>2"]);
            obj.set_accels_for_action("win.mode-spider-suit-3", &["<primary>3"]);
            obj.set_accels_for_action("win.mode-spider-suit-4", &["<primary>4"]);
            obj.set_accels_for_action("win.mode-freecell-card-26", &["<primary><shift>1"]);
            obj.set_accels_for_action("win.mode-freecell-card-39", &["<primary><shift>2"]);
            obj.set_accels_for_action(
                "win.mode-freecell-card-52",
                &["<primary><shift>3", "<primary><shift>4"],
            );
            obj.set_accels_for_action("win.mode-chess-standard", &["<primary><alt>1"]);
            obj.set_accels_for_action("win.mode-chess-960", &["<primary><alt>2"]);
            obj.set_accels_for_action("win.mode-chess-atomic", &["<primary><alt>3"]);
            obj.set_accels_for_action("win.chess-flip-board", &["<primary><alt>f"]);
            obj.set_accels_for_action("win.chess-auto-flip-board-each-move", &["<primary><alt>a"]);
            obj.set_accels_for_action("win.chess-show-board-coordinates", &["<primary><alt>c"]);
            obj.set_accels_for_action("win.chess-system-sounds-enabled", &["<primary><alt>m"]);
            obj.set_accels_for_action("win.chess-rotate-board-dialog", &["<primary><alt>o"]);
            obj.set_accels_for_action("win.chess-ai-strength-dialog", &["<primary><alt>e"]);
            obj.set_accels_for_action(
                "win.chess-w-question-ai-strength-dialog",
                &["<primary><alt>q"],
            );
            obj.set_accels_for_action("win.chess-wand-ai-strength-dialog", &["<primary><alt>w"]);
            obj.set_accels_for_action(
                "win.chess-robot-white-ai-strength-dialog",
                &["<primary><alt>i"],
            );
            obj.set_accels_for_action(
                "win.chess-robot-black-ai-strength-dialog",
                &["<primary><alt>b"],
            );
            obj.set_accels_for_action(
                "win.chess-wand-ai-opponent-auto-response",
                &["<primary><alt>p"],
            );
            obj.set_accels_for_action("win.chess-auto-response-plays-white", &["<primary><alt>l"]);
            obj.set_accels_for_action("win.smart-move-double-click", &["<primary><alt>d"]);
            obj.set_accels_for_action("win.smart-move-single-click", &["<primary><alt>s"]);
            obj.set_accels_for_action("win.smart-move-right-click", &["<primary><alt>r"]);
            obj.set_accels_for_action("win.smart-move-disabled", &["<primary><alt>0"]);
            obj.set_accels_for_action("win.robot-debug-toggle", &["F8"]);
            obj.set_accels_for_action("win.open-theme-presets", &["<primary><shift>t"]);
            obj.set_accels_for_action("win.open-custom-css", &["<primary><alt>u"]);
            obj.set_accels_for_action("win.status-history", &["<primary><shift>h"]);
            obj.set_accels_for_action("win.apm-graph", &["<primary><shift>a"]);
            obj.set_accels_for_action("app.about", &["<primary>i"]);
        }
    }

    impl ApplicationImpl for CardthropicApplication {
        fn activate(&self) {
            startup_trace::mark("app:activate-enter");
            let application = self.obj();
            let windows = application.windows();
            let game_window = windows
                .iter()
                .find_map(|w| w.clone().downcast::<CardthropicWindow>().ok());

            for window in windows {
                if window.is::<CardthropicWindow>() {
                    continue;
                }
                // Guard startup against stray top-level helper windows taking focus.
                window.close();
            }

            if let Some(window) = game_window {
                startup_trace::mark("app:activate-present-existing");
                window.present();
            } else {
                startup_trace::mark("app:activate-create-window");
                let window = CardthropicWindow::new(&*application);
                window.present();
            }
            startup_trace::mark("app:activate-exit");
        }
    }

    impl GtkApplicationImpl for CardthropicApplication {}
    impl AdwApplicationImpl for CardthropicApplication {}
}

glib::wrapper! {
    pub struct CardthropicApplication(ObjectSubclass<imp::CardthropicApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl CardthropicApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("resource-base-path", "/io/codeberg/emviolet/cardthropic")
            .build()
    }

    fn setup_gactions(&self) {
        let new_window_action = gio::ActionEntry::builder("new-window")
            .activate(move |app: &Self, _, _| app.open_new_window())
            .build();
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([new_window_action, quit_action, about_action]);
    }

    fn open_new_window(&self) {
        let window = CardthropicWindow::new(self);
        window.present();
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Cardthropic")
            .application_icon("io.codeberg.emviolet.cardthropic")
            .developer_name("emviolet")
            .version(VERSION)
            .license_type(gtk::License::Gpl30)
            .developers(vec!["emviolet"])
            .translator_credits(gettext("translator-credits"))
            .copyright("© 2026 emviolet")
            .build();

        about.present(Some(&window));
    }
}
