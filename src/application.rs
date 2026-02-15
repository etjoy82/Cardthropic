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
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
            obj.set_accels_for_action("win.help", &["F1"]);
            obj.set_accels_for_action("win.random-seed", &["<primary>r"]);
            obj.set_accels_for_action("win.winnable-seed", &["<primary><shift>r"]);
            obj.set_accels_for_action("win.draw", &["space"]);
            obj.set_accels_for_action("win.undo", &["<primary>z"]);
            obj.set_accels_for_action("win.redo", &["<primary>y"]);
            obj.set_accels_for_action("win.toggle-fullscreen", &["F11"]);
            obj.set_accels_for_action("win.play-hint-move", &["<primary>space"]);
            obj.set_accels_for_action("win.rapid-wand", &["<primary><shift>space"]);
            obj.set_accels_for_action("win.peek", &["F3"]);
            obj.set_accels_for_action("win.robot-mode", &["F6"]);
            obj.set_accels_for_action("win.cyclone-shuffle", &["F5"]);
            obj.set_accels_for_action("win.enable-hud", &["grave"]);
        }
    }

    impl ApplicationImpl for CardthropicApplication {
        fn activate(&self) {
            let application = self.obj();
            let window = application.active_window().unwrap_or_else(|| {
                let window = CardthropicWindow::new(&*application);
                window.upcast()
            });
            window.present();
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
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, about_action]);
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
