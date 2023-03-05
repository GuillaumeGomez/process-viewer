use gtk::prelude::*;
use gtk::{glib, EventControllerKey, Inhibit};

use sysinfo::NetworkExt;

use crate::graph::GraphWidget;
use crate::notebook::NoteBook;
use crate::utils::{
    format_number, format_number_full, get_main_window, graph_label, graph_label_units, RotateVec,
};

use std::cell::{Cell, RefCell};
use std::iter;
use std::rc::Rc;

pub struct NetworkDialog {
    pub name: String,
    popup: gtk::Window,
    packets_errors_history: Rc<RefCell<GraphWidget>>,
    in_out_history: Rc<RefCell<GraphWidget>>,
    received_peak: Rc<RefCell<u64>>,
    transmitted_peak: Rc<RefCell<u64>>,
    packets_received_peak: Rc<RefCell<u64>>,
    packets_transmitted_peak: Rc<RefCell<u64>>,
    errors_on_received_peak: Rc<RefCell<u64>>,
    errors_on_transmitted_peak: Rc<RefCell<u64>>,
    to_be_removed: Rc<Cell<bool>>,
    list_store: gtk::ListStore,
}

macro_rules! update_graph {
    ($this:expr, $t:expr, $pos:expr, $value:expr, $total_value:expr, $peak:ident, $list_pos:expr, $formatter:ident) => {{
        $t.data($pos, |d| {
            d.move_start();
            *d.get_mut(0).expect("cannot get data 0") = $value as f32;
        });
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

        let t = self.packets_errors_history.borrow_mut();
        update_graph!(
            self,
            t,
            0,
            network.packets_received(),
            network.total_packets_received(),
            packets_received_peak,
            9,
            formatter
        );
        update_graph!(
            self,
            t,
            1,
            network.packets_transmitted(),
            network.total_packets_transmitted(),
            packets_transmitted_peak,
            12,
            formatter
        );
        update_graph!(
            self,
            t,
            2,
            network.errors_on_received(),
            network.total_errors_on_received(),
            errors_on_received_peak,
            15,
            formatter
        );
        update_graph!(
            self,
            t,
            3,
            network.errors_on_transmitted(),
            network.total_errors_on_transmitted(),
            errors_on_transmitted_peak,
            18,
            formatter
        );
        t.queue_draw();

        let t = self.in_out_history.borrow_mut();
        update_graph!(
            self,
            t,
            0,
            network.received(),
            network.total_received(),
            received_peak,
            3,
            format_number
        );
        update_graph!(
            self,
            t,
            1,
            network.transmitted(),
            network.total_transmitted(),
            transmitted_peak,
            6,
            format_number
        );
        t.queue_draw();
    }

    pub fn show(&self) {
        self.popup.present();
    }

    pub fn need_remove(&self) -> bool {
        self.to_be_removed.get()
    }
}

fn append_text_column(tree: &gtk::TreeView, title: &str, pos: i32, right_align: bool) {
    let column = gtk::TreeViewColumn::builder()
        .title(title)
        .resizable(true)
        .build();
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
    tree.append_column(&column);
}

pub fn create_network_dialog(
    network: &sysinfo::NetworkData,
    interface_name: &str,
) -> NetworkDialog {
    let mut notebook = NoteBook::new();

    let popup = gtk::Window::new();

    popup.set_title(Some(&format!(
        "Information about network {}",
        interface_name
    )));
    popup.set_transient_for(get_main_window().as_ref());
    popup.set_destroy_with_parent(true);

    let close_button = gtk::Button::with_label("Close");
    close_button.add_css_class("button-with-margin");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

    notebook.notebook.set_hexpand(true);
    notebook.notebook.set_vexpand(true);
    vertical_layout.append(&notebook.notebook);
    vertical_layout.append(&close_button);
    popup.set_child(Some(&vertical_layout));

    //
    // GRAPH TAB
    //
    let vertical_layout = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(5)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(5)
        .margin_end(5)
        .build();
    let scroll = gtk::ScrolledWindow::new();
    let in_out_history = GraphWidget::new(Some(1.), false);

    in_out_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "received",
        None,
    );
    in_out_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "transmitted",
        None,
    );
    in_out_history.set_labels_callback(Some(Box::new(graph_label_units)));
    let label = gtk::Label::new(None);
    label.set_markup("<b>Network usage</b>");
    vertical_layout.append(&label);
    vertical_layout.append(&in_out_history);
    in_out_history.queue_draw();
    // let in_out_history = connect_graph(in_out_history);
    let in_out_history = Rc::new(RefCell::new(in_out_history));

    let packets_errors_history = GraphWidget::new(Some(1.), false);

    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "received packets",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "transmitted packets",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "errors on received",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "errors on transmitted",
        None,
    );
    packets_errors_history.set_labels_callback(Some(Box::new(graph_label)));
    let label = gtk::Label::new(None);
    label.set_markup("<b>Extra data</b>");
    vertical_layout.append(&label);
    vertical_layout.append(&packets_errors_history);
    packets_errors_history.queue_draw();
    // let packets_errors_history = connect_graph(packets_errors_history);
    let packets_errors_history = Rc::new(RefCell::new(packets_errors_history));

    scroll.set_child(Some(&vertical_layout));
    scroll.connect_show(
        glib::clone!(@weak packets_errors_history, @weak in_out_history => move |_| {
            packets_errors_history.borrow().show();
            in_out_history.borrow().show();
        }),
    );
    notebook.create_tab("Graphics", &scroll);

    //
    // NETWORK INFO TAB
    //
    let list_store = gtk::ListStore::new(&[glib::Type::STRING, glib::Type::STRING]);
    let tree = gtk::TreeView::builder()
        .headers_visible(true)
        .model(&list_store)
        .build();

    append_text_column(&tree, "property", 0, false);
    append_text_column(&tree, "value", 1, true);

    list_store.insert_with_values(
        None,
        &[(0, &"MAC address"), (1, &network.mac_address().to_string())],
    );
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

    popup.set_size_request(700, 540);

    let to_be_removed = Rc::new(Cell::new(false));
    popup.connect_destroy(glib::clone!(@weak to_be_removed => move |_| {
        to_be_removed.set(true);
    }));
    close_button.connect_clicked(glib::clone!(@weak popup, @weak to_be_removed => move |_| {
        popup.close();
    }));
    popup.connect_close_request(
        glib::clone!(@weak to_be_removed => @default-return Inhibit(false), move |_| {
            to_be_removed.set(true);
            Inhibit(false)
        }),
    );
    let event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(glib::clone!(
        @weak popup,
        @weak to_be_removed
        => @default-return Inhibit(false), move |_, key, _, _modifier| {
            if key == gtk::gdk::Key::Escape {
                popup.close();
                to_be_removed.set(true);
            }
            Inhibit(false)
        }
    ));
    popup.add_controller(event_controller);
    popup.set_resizable(true);
    popup.show();

    let adjust = scroll.vadjustment();
    adjust.set_value(0.);
    scroll.set_vadjustment(Some(&adjust));

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
