use super::*;

impl CardthropicWindow {
    pub(super) fn play_hint_animation(&self, source: HintNode, target: HintNode) {
        self.clear_hint_effects();
        self.set_hint_node_active(source, true);

        let id_1 = glib::timeout_add_local(
            Duration::from_millis(1000),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.set_hint_node_active(source, false);
                    window.set_hint_node_active(target, true);
                    glib::ControlFlow::Break
                }
            ),
        );

        let id_2 = glib::timeout_add_local(
            Duration::from_millis(2000),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.set_hint_node_active(target, false);
                    glib::ControlFlow::Break
                }
            ),
        );

        let imp = self.imp();
        imp.hint_timeouts.borrow_mut().push(id_1);
        imp.hint_timeouts.borrow_mut().push(id_2);
    }

    pub(super) fn clear_hint_effects(&self) {
        let imp = self.imp();
        for timeout_id in imp.hint_timeouts.borrow_mut().drain(..) {
            Self::remove_source_if_present(timeout_id);
        }
        for widget in imp.hint_widgets.borrow_mut().drain(..) {
            widget.remove_css_class("hint-invert");
        }
    }

    pub(super) fn set_hint_node_active(&self, node: HintNode, active: bool) {
        if let Some(widget) = self.widget_for_hint_node(node) {
            if active {
                widget.add_css_class("hint-invert");
                self.imp().hint_widgets.borrow_mut().push(widget);
            } else {
                widget.remove_css_class("hint-invert");
            }
        }
    }

    pub(super) fn widget_for_hint_node(&self, node: HintNode) -> Option<gtk::Widget> {
        let imp = self.imp();
        match node {
            HintNode::Stock => Some(imp.stock_picture.get().upcast()),
            HintNode::Waste => Some(imp.waste_picture.get().upcast()),
            HintNode::Foundation(index) => self
                .foundation_pictures()
                .get(index)
                .map(|picture| picture.clone().upcast()),
            HintNode::Tableau { col, index } => {
                if let Some(card_index) = index {
                    imp.tableau_card_pictures
                        .borrow()
                        .get(col)?
                        .get(card_index)
                        .map(|picture| picture.clone().upcast())
                } else {
                    self.tableau_stacks()
                        .get(col)
                        .map(|stack| stack.clone().upcast())
                }
            }
        }
    }
}
