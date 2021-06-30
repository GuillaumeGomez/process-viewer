use gtk::prelude::{
    AdjustmentExt, BoxExt, ButtonExt, CellLayoutExt, CellRendererExt, CellRendererTextExt,
    ContainerExt, GtkListStoreExtManual, GtkWindowExt, Inhibit, LabelExt, ScrolledWindowExt,
    TreeModelExt, TreeViewColumnExt, TreeViewExt, WidgetExt,
};
use gtk::{self, glib};

use sysinfo::{self, NetworkExt};

use crate::graph::{Connecter, Graph};
use crate::notebook::NoteBook;
use crate::utils::{
    connect_graph, format_number, format_number_full, get_main_window, graph_label,
    graph_label_units, RotateVec,
};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

pub struct NetworkDialog {
    pub name: String,
    popup: gtk::Window,
    packets_errors_history: Rc<RefCell<Graph>>,
    in_out_history: Rc<RefCell<Graph>>,
    received_peak: Rc<RefCell<u64>>,
    transmitted_peak: Rc<RefCell<u64>>,
    packets_received_peak: Rc<RefCell<u64>>,
    packets_transmitted_peak: Rc<RefCell<u64>>,
    errors_on_received_peak: Rc<RefCell<u64>>,
    errors_on_transmitted_peak: Rc<RefCell<u64>>,
    to_be_removed: Rc<RefCell<bool>>,
    list_store: gtk::ListStore,
}

macro_rules! update_graph {
    ($this:expr, $t:expr, $pos:expr, $value:expr, $total_value:expr, $peak:ident, $list_pos:expr, $formatter:ident) => {{
        $t.data[$pos].move_start();
        *$t.data[$pos].get_mut(0).expect("cannot get data 0") = $value as f64;
        let mut x = $this.$peak.borrow_mut();
        if *x < $value {
            *x = $value;
            if let Some(iter) = $this.list_store.iter_nth_child(None, $list_pos - 1) {
                $this.list_store.set(&iter, &[(1, &$formatter($value))]);
            }
        }
        if let Some(iter) = $this.list_store.iter_nth_child(None, $list_pos - 2) {
            $this.list_store.set(&iter, &[(1, &$formatter($value))]);
        }
        if let Some(iter) = $this.list_store.iter_nth_child(None, $list_pos) {
            $this
                .list_store
                .set(&iter, &[(1, &$formatter($total_value))]);
        }
    }};
}

impl NetworkDialog {
    #[allow(clippy::cognitive_complexity)]
    pub fn update(&self, network: &sysinfo::NetworkData) {
        if self.need_remove() {
            return;
        }
        fn formatter(value: u64) -> String {
            format_number_full(value, false)
        }

        let mut t = self.packets_errors_history.borrow_mut();
        update_graph!(
            self,
            t,
            0,
            network.packets_received(),
            network.total_packets_received(),
            packets_received_peak,
            8,
            formatter
        );
        update_graph!(
            self,
            t,
            1,
            network.packets_transmitted(),
            network.total_packets_transmitted(),
            packets_transmitted_peak,
            11,
            formatter
        );
        update_graph!(
            self,
            t,
            2,
            network.errors_on_received(),
            network.total_errors_on_received(),
            errors_on_received_peak,
            14,
            formatter
        );
        update_graph!(
            self,
            t,
            3,
            network.errors_on_transmitted(),
            network.total_errors_on_transmitted(),
            errors_on_transmitted_peak,
            17,
            formatter
        );
        t.invalidate();

        let mut t = self.in_out_history.borrow_mut();
        update_graph!(
            self,
            t,
            0,
            network.received(),
            network.total_received(),
            received_peak,
            2,
            format_number
        );
        update_graph!(
            self,
            t,
            1,
            network.transmitted(),
            network.total_transmitted(),
            transmitted_peak,
            5,
            format_number
        );
        t.invalidate();
    }

    pub fn show(&self) {
        self.popup.present();
    }

    pub fn need_remove(&self) -> bool {
        *self.to_be_removed.borrow()
    }
}

fn append_text_column(tree: &gtk::TreeView, title: &str, pos: i32, right_align: bool) {
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();

    if right_align {
        cell.set_xalign(1.0);
    }
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", pos);
    if pos == 1 {
        cell.set_wrap_mode(gtk::pango::WrapMode::Char);
        column.set_expand(true);
    }
    column.set_title(title);
    column.set_resizable(true);
    tree.append_column(&column);
}

pub fn create_network_dialog(
    network: &sysinfo::NetworkData,
    interface_name: &str,
) -> NetworkDialog {
    let mut notebook = NoteBook::new();

    let popup = gtk::Window::new(gtk::WindowType::Toplevel);

    popup.set_title(&format!("Information about network {}", interface_name));
    popup.set_transient_for(get_main_window().as_ref());
    popup.set_destroy_with_parent(true);

    let close_button = gtk::Button::with_label("Close");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

    vertical_layout.pack_start(&notebook.notebook, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);
    popup.add(&vertical_layout);

    //
    // GRAPH TAB
    //
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vertical_layout.set_spacing(5);
    vertical_layout.set_margin_top(10);
    vertical_layout.set_margin_bottom(10);
    vertical_layout.set_margin_start(5);
    vertical_layout.set_margin_end(5);
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    let mut in_out_history = Graph::new(Some(1.), false);

    in_out_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "received",
        None,
    );
    in_out_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "transmitted",
        None,
    );
    in_out_history.set_label_callbacks(Some(Box::new(graph_label_units)));
    let label = gtk::Label::new(None);
    label.set_markup("<b>Network usage</b>");
    vertical_layout.add(&label);
    in_out_history.attach_to(&vertical_layout);
    in_out_history.invalidate();
    in_out_history.set_labels_width(120);
    let in_out_history = connect_graph(in_out_history);

    let mut packets_errors_history = Graph::new(Some(1.), false);

    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "received packets",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "transmitted packets",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "errors on received",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "errors on transmitted",
        None,
    );
    packets_errors_history.set_label_callbacks(Some(Box::new(graph_label)));
    packets_errors_history.set_labels_width(120);
    let label = gtk::Label::new(None);
    label.set_markup("<b>Extra data</b>");
    vertical_layout.add(&label);
    packets_errors_history.attach_to(&vertical_layout);
    packets_errors_history.invalidate();
    let packets_errors_history = connect_graph(packets_errors_history);

    scroll.add(&vertical_layout);
    scroll.connect_show(
        glib::clone!(@weak packets_errors_history, @weak in_out_history => move |_| {
            packets_errors_history.borrow().show_all();
            in_out_history.borrow().show_all();
        }),
    );
    notebook.create_tab("Graphics", &scroll);

    //
    // NETWORK INFO TAB
    //
    let tree = gtk::TreeView::new();
    let list_store = gtk::ListStore::new(&[glib::Type::STRING, glib::Type::STRING]);

    tree.set_headers_visible(true);
    tree.set_model(Some(&list_store));

    append_text_column(&tree, "property", 0, false);
    append_text_column(&tree, "value", 1, true);

    list_store.insert_with_values(
        None,
        &[(0, &"received"), (1, &format_number(network.received()))],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"received peak"),
            (1, &format_number(network.received())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"total received"),
            (1, &format_number(network.total_received())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"transmitted"),
            (1, &format_number(network.transmitted())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"transmitted peak"),
            (1, &format_number(network.transmitted())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"total transmitted"),
            (1, &format_number(network.total_transmitted())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"packets received"),
            (1, &format_number_full(network.packets_received(), false)),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"packets received peak"),
            (1, &format_number(network.packets_received())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"total packets received"),
            (
                1,
                &format_number_full(network.total_packets_received(), false),
            ),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"packets transmitted"),
            (1, &format_number_full(network.packets_transmitted(), false)),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"packets transmitted peak"),
            (1, &format_number(network.packets_transmitted())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"total packets transmitted"),
            (
                1,
                &format_number_full(network.total_packets_transmitted(), false),
            ),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"errors on received"),
            (1, &format_number_full(network.errors_on_received(), false)),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"errors on received peak"),
            (1, &format_number(network.errors_on_received()).as_str()),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"total errors on received"),
            (
                1,
                &format_number_full(network.total_errors_on_received(), false),
            ),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"errors on transmitted"),
            (
                1,
                &format_number_full(network.errors_on_transmitted(), false),
            ),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"errors on transmitted peak"),
            (1, &format_number(network.errors_on_transmitted())),
        ],
    );
    list_store.insert_with_values(
        None,
        &[
            (0, &"total errors on transmitted"),
            (
                1,
                &format_number_full(network.total_errors_on_transmitted(), false),
            ),
        ],
    );

    notebook.create_tab("Information", &tree);

    // To silence the annoying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    popup.preferred_width();
    popup.set_size_request(700, 540);

    close_button.connect_clicked(glib::clone!(@weak popup => move |_| {
        popup.close();
    }));
    let to_be_removed = Rc::new(RefCell::new(false));
    popup.connect_destroy(glib::clone!(@weak to_be_removed => move |_| {
        *to_be_removed.borrow_mut() = true;
    }));
    popup.connect_key_press_event(|win, key| {
        if key.keyval() == gtk::gdk::keys::constants::Escape {
            win.close();
        }
        Inhibit(false)
    });
    popup.set_resizable(true);
    popup.show_all();

    let adjust = scroll.vadjustment();
    adjust.set_value(0.);
    scroll.set_vadjustment(Some(&adjust));

    packets_errors_history.connect_to_window_events();
    in_out_history.connect_to_window_events();

    NetworkDialog {
        name: interface_name.to_owned(),
        popup,
        packets_errors_history,
        in_out_history,
        received_peak: Rc::new(RefCell::new(network.received())),
        transmitted_peak: Rc::new(RefCell::new(network.transmitted())),
        packets_received_peak: Rc::new(RefCell::new(network.packets_received())),
        packets_transmitted_peak: Rc::new(RefCell::new(network.packets_transmitted())),
        errors_on_received_peak: Rc::new(RefCell::new(network.errors_on_received())),
        errors_on_transmitted_peak: Rc::new(RefCell::new(network.errors_on_transmitted())),
        to_be_removed,
        list_store,
    }
}
