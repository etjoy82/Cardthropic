use super::*;

impl CardthropicWindow {
    pub(super) fn update_tableau_overflow_hints(&self) {
        let imp = self.imp();
        let scroller = imp.tableau_scroller.get();
        let adj = scroller.hadjustment();

        let value = adj.value();
        let upper = adj.upper();
        let page_size = adj.page_size();
        let max_value = (upper - page_size).max(0.0);
        let overflow = max_value > 1.0;
        let show_left = overflow && value > 1.0;
        let show_right = overflow && value < (max_value - 1.0);

        if overflow {
            scroller.add_css_class("tableau-overflow");
        } else {
            scroller.remove_css_class("tableau-overflow");
        }
        if show_left {
            scroller.add_css_class("tableau-overflow-left");
        } else {
            scroller.remove_css_class("tableau-overflow-left");
        }
        if show_right {
            scroller.add_css_class("tableau-overflow-right");
        } else {
            scroller.remove_css_class("tableau-overflow-right");
        }
    }

    pub(super) fn setup_tableau_overflow_hints(&self) {
        let imp = self.imp();
        let scroller = imp.tableau_scroller.get();
        let adj = scroller.hadjustment();
        adj.connect_value_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.update_tableau_overflow_hints();
            }
        ));
        adj.connect_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.update_tableau_overflow_hints();
            }
        ));
        self.update_tableau_overflow_hints();
    }
}
