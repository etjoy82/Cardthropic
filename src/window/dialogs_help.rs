use super::*;

impl CardthropicWindow {
    fn manual_pages() -> [(&'static str, &'static str); 5] {
        [
            (
                "Welcome",
                r#"# Welcome to Cardthropic

Thanks for trying Cardthropic.

This first-launch intro opens after the main game window so you can see the board first, then jump into the manual.

## Quick Start
1. Pick a mode from the game menu (Klondike, Spider, FreeCell, or Chess).
2. Play a few moves manually.
3. Use Undo/Redo and seed controls to replay ideas.
4. Open automation tools when you want help exploring lines.

## What This Manual Covers
- Philosophy and design goals
- Practical play workflow
- Automation and solver behavior
- Keypad/gamepad mapping

Close this window any time with `Esc`.
"#,
            ),
            (
                "Philosophy",
                r#"# Cardthropic Manual

## What You Can Do Here
- Play Klondike, Spider, and FreeCell with mode-specific constraints
- Use seeds to replay, compare, and practice the same deal repeatedly
- Use automation to explore candidate lines and tactical branches
- Recover and iterate quickly with robust state history and reseeding workflows

## Why Cardthropic Exists
Cardthropic exists to make solitaire feel alive, inspectable, and playful while still respecting the strategy depth that makes the genre worth mastering.

This project treats solitaire as:
- A strategy sandbox, not just a time filler
- A systems game where decisions have measurable effects
- A place where humans and automation can collaborate

## Design Values
- **Transparent state**: runs, seeds, and outcomes should be understandable.
- **Fast iteration**: switch modes quickly, replay ideas quickly, and test hypotheses quickly.
- **Intentional imperfection**: automation is useful but not omniscient.

## What “Good” Looks Like
A good Cardthropic session is one where you can:
1. Select a mode and constraints intentionally.
2. Explore a deal with confidence that input latency and rendering stay out of the way.
3. Use automation as an assistant, not a replacement for your thinking.
4. Learn from both wins and losses.
"#,
            ),
            (
                "Playing & Workflow",
                r#"## Core Workflow
1. Pick a game family and mode constraints.
2. Start from a random or seed-driven deal.
3. Use manual play first to establish structure.
4. Bring in hint/automation tools when the position gets tactical.
5. Replay or reseed to compare approaches.

## Seed-Centric Practice
Seeds make strategy reproducible.

Use seed workflows to:
- Re-test a difficult line.
- Compare two different opening plans on identical initial states.
- Validate if a difficult position was execution-limited or structurally bad.

## Performance and Keyboard
Cardthropic tracks pace and rhythm so you can improve execution:
- Move economy matters more than speed spikes.
- Avoid reversible churn when it does not improve board potential.
- Preserve flexibility in free spaces and temporary storage.

## Keyboard-First Strategy
Cardthropic supports keyboard-driven play and menu access so the game remains fully operable without pointer-only interactions.

This makes repeated testing, reseeding, and automation control much faster over long sessions.
"#,
            ),
            (
                "Automation & Solver",
                r#"## Automation Model
Automation in Cardthropic is pragmatic:
- It can solve many deals.
- It can fail on solvable deals.
- It can win through persistence and controlled retries.

This is intentional. The system is built to be robust and observable, not magical.

## How to Use Automation Well
1. Let automation handle mechanical branches and repetitive candidate testing.
2. Intervene manually when structure or tempo looks wrong.
3. Re-run from seed to evaluate whether failure is heuristic, not impossibility.

## Solver Nuances
The planner/solver stack is heuristic-driven and bounded:
- Prioritizes progress signals (foundation movement, mobility, flexibility).
- Penalizes repeated-state churn and obvious cycles.
- Applies loss guards (for example, progress drought windows) to avoid endless loops.

Normal automation is policy-limited. Forever Mode is intentionally unbounded exploration.

Because of this, results are best interpreted as:
- **Found a practical line under current heuristics**, or
- **Did not find one within policy limits**

not absolute proof of global impossibility.

## Forever Mode and Ludicrous Speed
- **Forever Mode** keeps automation running across outcomes and reseeds by design.
- **Ludicrous Speed** removes pacing delay so actions execute as fast as policy allows.

Use them intentionally:
1. Turn on Forever Mode for long unattended exploration runs.
2. Turn on Ludicrous Speed when you want throughput over readability.
3. Turn off Ludicrous Speed when you want to inspect behavior move-by-move.

These toggles are strongest when used with seed replay and benchmark snapshots.

## Quick Recipes
### If You Are Stuck
1. Run Smart Move once.
2. If progress stalls, run automation for 30 to 60 seconds.
3. Undo several moves.
4. Try a different structural plan from that branch.

### If You Are Practicing
1. Pick a seed and play manually for two minutes.
2. Run robot automation on the same deal.
3. Replay the seed and compare your line to the robot line.

## Human + Robot Collaboration Pattern
- Human opens structure.
- Robot explores tactical branches.
- Human resolves strategic forks.
- Robot finishes conversion where execution density is high.

This hybrid style typically outperforms either side used alone.
"#,
            ),
            (
                "Gamepad Keypad Mapping",
                r#"## Goal
Map a gamepad to keypad-driven controls so solitaire stays fully playable from controller input without distro-specific dependencies.

## Works on Any Linux Distro
Use any remapper that can emit keyboard keys:
- Steam Input
- Input Remapper
- AntiMicroX
- Other controller-to-keyboard tools

The workflow is the same regardless of distro: map gamepad buttons to keyboard outputs, then keep that profile active while Cardthropic is focused.

## Cardthropic Keypad Targets
Cardthropic now supports these keypad controls:
- `KP_7`: undo
- `KP_9`: redo
- `KP_1`: robot toggle (start/stop)
- `KP_3`: peek (temporary reveal)
- `KP_5`: activate focused target
- `KP_Decimal` (`KP_Delete`): draw card
- `KP_Multiply`: play wand move
- `KP_Subtract`: undo
- `KP_Divide`: open command palette

## Recommended Controller Layout
Use this baseline profile first:
1. D-pad Up -> `KP_7` (undo)
2. D-pad Right -> `KP_9` (redo)
3. D-pad Left -> `KP_1` (robot toggle)
4. D-pad Down -> `KP_3` (peek)
5. South / A button -> `KP_5` (activate)
6. East / B button -> `KP_Subtract` (undo fallback)
7. North / Y button -> `KP_Multiply` (wand)
8. West / X button -> `KP_Decimal` (draw)
9. Start / Menu button -> `KP_Divide` (command palette)

If your remapper supports layers, keep these in a dedicated `Cardthropic` layer/profile.

## Setup Steps (Distro-Agnostic)
1. Create a new profile in your remapper named `Cardthropic`.
2. Bind each gamepad control to the keypad key targets above.
3. Set profile scope to app/window focus if your tool supports per-app profiles.
4. Disable remapper turbo/autofire for gameplay buttons to prevent accidental repeats.
5. Keep `Num Lock` enabled unless your remapper explicitly emits keypad keysyms independent of lock state.
6. Save and activate the profile.

## In-App Validation
1. Start any solitaire mode.
2. Press your mapped `KP_7/9` controls and confirm undo/redo behavior.
3. Press your mapped `KP_1` to toggle Robot Mode and `KP_3` to trigger Peek.
4. Press mapped `KP_5` and confirm it activates the focused pile/slot/card.
5. Press mapped draw/undo/wand buttons and verify expected behavior.
6. Press mapped `KP_Divide` and confirm command palette opens.

## Reliability Tips
- Use press-once bindings, not hold-repeat, for `KP_1`, `KP_3`, `KP_5`, `KP_Subtract`, and `KP_Divide`.
- If input feels doubled, disable duplicate mappings in desktop/game launcher overlays.
- If a mapping appears ignored, re-check whether your remapper sent a top-row key (`1`, `/`, `-`) instead of keypad key (`KP_1`, `KP_Divide`, `KP_Subtract`).
- Keep one active profile per controller to avoid key conflicts.

## Optional Enhancements
1. Add shoulder buttons mapped to Arrow keys for fine directional movement.
2. Add a secondary modifier layer for non-solitaire shortcuts (fullscreen/help) if desired.
3. Export your remapper profile so it can be reused across machines and distros.
"#,
            ),
        ]
    }

    fn render_inline_markdown(raw: &str) -> String {
        let mut out = String::new();
        let mut plain_start = 0usize;
        let mut i = 0usize;
        while i < raw.len() {
            let rest = &raw[i..];
            if rest.starts_with("**") {
                if let Some(rel_end) = raw[i + 2..].find("**") {
                    let end = i + 2 + rel_end;
                    out.push_str(&glib::markup_escape_text(&raw[plain_start..i]));
                    let content = glib::markup_escape_text(&raw[i + 2..end]);
                    out.push_str(&format!("<b>{content}</b>"));
                    i = end + 2;
                    plain_start = i;
                    continue;
                }
            } else if rest.starts_with('`') {
                if let Some(rel_end) = raw[i + 1..].find('`') {
                    let end = i + 1 + rel_end;
                    out.push_str(&glib::markup_escape_text(&raw[plain_start..i]));
                    let content = glib::markup_escape_text(&raw[i + 1..end]);
                    out.push_str(&format!("<tt>{content}</tt>"));
                    i = end + 1;
                    plain_start = i;
                    continue;
                }
            } else if rest.starts_with('*') {
                if let Some(rel_end) = raw[i + 1..].find('*') {
                    let end = i + 1 + rel_end;
                    out.push_str(&glib::markup_escape_text(&raw[plain_start..i]));
                    let content = glib::markup_escape_text(&raw[i + 1..end]);
                    out.push_str(&format!("<i>{content}</i>"));
                    i = end + 1;
                    plain_start = i;
                    continue;
                }
            }
            if let Some(ch) = rest.chars().next() {
                i += ch.len_utf8();
            } else {
                break;
            }
        }
        out.push_str(&glib::markup_escape_text(&raw[plain_start..]));
        out
    }

    fn markdown_to_pango_markup(markdown: &str) -> String {
        let mut out = String::new();
        let mut in_code = false;
        for raw_line in markdown.lines() {
            let line = raw_line.trim_end();
            if line.starts_with("```") {
                in_code = !in_code;
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                continue;
            }
            if in_code {
                let text = glib::markup_escape_text(line);
                out.push_str(&format!("<tt>{text}</tt>\n"));
                continue;
            }
            if let Some(rest) = line.strip_prefix("# ") {
                out.push_str(&format!(
                    "<span size=\"xx-large\" weight=\"bold\">{}</span>\n",
                    Self::render_inline_markdown(rest)
                ));
            } else if let Some(rest) = line.strip_prefix("## ") {
                out.push_str(&format!(
                    "<span size=\"x-large\" weight=\"bold\">{}</span>\n",
                    Self::render_inline_markdown(rest)
                ));
            } else if let Some(rest) = line.strip_prefix("### ") {
                out.push_str(&format!(
                    "<span size=\"large\" weight=\"bold\">{}</span>\n",
                    Self::render_inline_markdown(rest)
                ));
            } else if let Some(rest) = line.strip_prefix("- ") {
                out.push_str(&format!("• {}\n", Self::render_inline_markdown(rest)));
            } else if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str(&format!("{}\n", Self::render_inline_markdown(line)));
            }
        }
        out
    }

    fn markdown_page_view(title: &str, markdown: &str) -> gtk::ScrolledWindow {
        let label = gtk::Label::new(None);
        label.set_use_markup(true);
        label.set_markup(&Self::markdown_to_pango_markup(markdown));
        label.set_selectable(true);
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        label.set_xalign(0.0);
        label.set_yalign(0.0);
        label.set_margin_top(10);
        label.set_margin_bottom(10);
        label.set_margin_start(12);
        label.set_margin_end(12);
        label.set_tooltip_text(Some(title));

        let clamp = adw::Clamp::new();
        clamp.set_child(Some(&label));
        clamp.set_maximum_size(860);
        clamp.set_tightening_threshold(620);

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        scroller.set_child(Some(&clamp));
        scroller
    }

    fn help_window_size(&self) -> (i32, i32) {
        const DEFAULT_WIDTH: i32 = 760;
        const DEFAULT_HEIGHT: i32 = 640;
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 280;

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        let Some(schema) = settings.settings_schema() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        if !schema.has_key(SETTINGS_KEY_HELP_WIDTH) || !schema.has_key(SETTINGS_KEY_HELP_HEIGHT) {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        }

        (
            settings.int(SETTINGS_KEY_HELP_WIDTH).max(MIN_WIDTH),
            settings.int(SETTINGS_KEY_HELP_HEIGHT).max(MIN_HEIGHT),
        )
    }

    fn help_window_maximized(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return false;
        };
        let Some(schema) = settings.settings_schema() else {
            return false;
        };
        if !schema.has_key(SETTINGS_KEY_HELP_MAXIMIZED) {
            return false;
        }
        settings.boolean(SETTINGS_KEY_HELP_MAXIMIZED)
    }

    fn persist_help_window_maximized(&self, maximized: bool) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_HELP_MAXIMIZED) {
            return;
        }
        if settings.boolean(SETTINGS_KEY_HELP_MAXIMIZED) != maximized {
            let _ = settings.set_boolean(SETTINGS_KEY_HELP_MAXIMIZED, maximized);
        }
    }

    fn persist_help_window_size(&self, dialog: &gtk::Window) {
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 280;

        if dialog.is_maximized() {
            return;
        }

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_HELP_WIDTH) || !schema.has_key(SETTINGS_KEY_HELP_HEIGHT) {
            return;
        }

        let width = dialog.width().max(MIN_WIDTH);
        let height = dialog.height().max(MIN_HEIGHT);
        if settings.int(SETTINGS_KEY_HELP_WIDTH) != width {
            let _ = settings.set_int(SETTINGS_KEY_HELP_WIDTH, width);
        }
        if settings.int(SETTINGS_KEY_HELP_HEIGHT) != height {
            let _ = settings.set_int(SETTINGS_KEY_HELP_HEIGHT, height);
        }
    }

    pub(super) fn schedule_first_launch_intro_if_needed(&self) {
        if !self.should_persist_shared_state() {
            return;
        }
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_FIRST_LAUNCH_INTRO_SHOWN) {
            return;
        }
        if settings.boolean(SETTINGS_KEY_FIRST_LAUNCH_INTRO_SHOWN) {
            return;
        }

        glib::timeout_add_seconds_local(
            3,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if !window.is_visible() {
                        return glib::ControlFlow::Break;
                    }
                    let settings = window.imp().settings.borrow().clone();
                    let Some(settings) = settings.as_ref() else {
                        return glib::ControlFlow::Break;
                    };
                    let Some(schema) = settings.settings_schema() else {
                        return glib::ControlFlow::Break;
                    };
                    if !schema.has_key(SETTINGS_KEY_FIRST_LAUNCH_INTRO_SHOWN)
                        || settings.boolean(SETTINGS_KEY_FIRST_LAUNCH_INTRO_SHOWN)
                    {
                        return glib::ControlFlow::Break;
                    }
                    let _ = settings.set_boolean(SETTINGS_KEY_FIRST_LAUNCH_INTRO_SHOWN, true);
                    window.show_help_dialog();
                    glib::ControlFlow::Break
                }
            ),
        );
    }

    pub(super) fn show_help_dialog(&self) {
        if let Some(existing) = self.imp().help_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let (saved_width, saved_height) = self.help_window_size();
        let saved_maximized = self.help_window_maximized();
        let dialog = gtk::Window::builder()
            .title("Cardthropic Manual")
            .modal(false)
            .default_width(saved_width)
            .default_height(saved_height)
            .build();
        dialog.set_transient_for(Some(self));
        dialog.set_resizable(true);
        dialog.set_deletable(true);
        dialog.set_hide_on_close(false);
        dialog.set_destroy_with_parent(true);
        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |dialog| {
                let maximized = dialog.is_maximized();
                window.persist_help_window_maximized(maximized);
                window.persist_help_window_size(dialog);
                *window.imp().help_dialog.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        if saved_maximized {
            dialog.maximize();
        }

        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(glib::clone!(
            #[weak]
            dialog,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    dialog.close();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        dialog.add_controller(key_controller);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let title = gtk::Label::new(Some("Manual"));
        title.set_xalign(0.0);
        title.add_css_class("title-4");
        root.append(&title);

        let stack = adw::ViewStack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        let pages = Self::manual_pages();
        for (name, markdown) in pages.iter() {
            let page = Self::markdown_page_view(name, markdown);
            stack.add_titled(&page, Some(name), name);
        }
        if let Some((first_name, _)) = pages.first() {
            stack.set_visible_child_name(first_name);
        }

        let tabs = gtk::Box::new(gtk::Orientation::Vertical, 6);
        tabs.set_halign(gtk::Align::Start);
        let tabs_row_top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        tabs_row_top.set_halign(gtk::Align::Start);
        let tabs_row_bottom = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        tabs_row_bottom.set_halign(gtk::Align::Start);
        let split_index = (pages.len() + 1) / 2;
        let mut group_anchor: Option<gtk::CheckButton> = None;
        for (idx, (name, _)) in pages.iter().enumerate() {
            let page_name = *name;
            let tab = gtk::CheckButton::with_label(page_name);
            if let Some(anchor) = group_anchor.as_ref() {
                tab.set_group(Some(anchor));
            } else {
                group_anchor = Some(tab.clone());
            }
            if idx == 0 {
                tab.set_active(true);
            }
            tab.connect_toggled(glib::clone!(
                #[weak]
                stack,
                #[strong]
                page_name,
                move |btn| {
                    if btn.is_active() {
                        stack.set_visible_child_name(page_name);
                    }
                }
            ));
            if idx < split_index {
                tabs_row_top.append(&tab);
            } else {
                tabs_row_bottom.append(&tab);
            }
        }
        tabs.append(&tabs_row_top);
        tabs.append(&tabs_row_bottom);
        root.append(&tabs);
        root.append(&stack);

        let close = gtk::Button::with_label("Close");
        close.set_halign(gtk::Align::End);
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        root.append(&close);

        dialog.set_child(Some(&root));
        *self.imp().help_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }

    pub(super) fn toggle_fullscreen_mode(&self) {
        if self.is_fullscreen() {
            self.unfullscreen();
            *self.imp().status_override.borrow_mut() = Some("Exited fullscreen.".to_string());
        } else {
            self.fullscreen();
            *self.imp().status_override.borrow_mut() = Some("Entered fullscreen.".to_string());
        }
        self.render();
    }
}
