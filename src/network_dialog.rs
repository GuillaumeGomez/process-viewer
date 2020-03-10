use gtk::prelude::{
    CellLayoutExt, CellRendererTextExt, GtkListStoreExtManual, GtkWindowExt, GtkWindowExtManual,
    TreeViewColumnExt, TreeViewExt, WidgetExt,
};
use gtk::{self, AdjustmentExt, BoxExt, ButtonExt, ContainerExt, LabelExt, ScrolledWindowExt};

use sysinfo::{self, NetworkExt};

use graph::{Connecter, Graph};
use notebook::NoteBook;
use process_dialog::append_text_column;
use utils::{connect_graph, format_number, format_number_full, get_main_window, RotateVec};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

pub struct NetworkDialog {
    pub name: String,
    popup: gtk::Window,
    notebook: NoteBook,
    packets_errors_history: Rc<RefCell<Graph>>,
    in_out_history: Rc<RefCell<Graph>>,
    income_peak: Rc<RefCell<u64>>,
    outcome_peak: Rc<RefCell<u64>>,
    income_packets_peak: Rc<RefCell<u64>>,
    outcome_packets_peak: Rc<RefCell<u64>>,
    income_errors_peak: Rc<RefCell<u64>>,
    outcome_errors_peak: Rc<RefCell<u64>>,
    to_be_removed: Rc<RefCell<bool>>,
}

macro_rules! update_graph {
    ($this:expr, $t:expr, $pos:expr, $value:expr, $peak:ident) => {{
        $t.data[$pos].move_start();
        *$t.data[$pos].get_mut(0).expect("cannot get data 0") = $value as f64;
        let mut x = $this.$peak.borrow_mut();
        if *x < $value {
            *x = $value;
            // TODO: update associated value in the non-graph view
        }
    }};
}

impl NetworkDialog {
    pub fn update(&self, network: &sysinfo::NetworkData) {
        let mut t = self.packets_errors_history.borrow_mut();
        update_graph!(
            self,
            t,
            0,
            network.get_packets_income(),
            income_packets_peak
        );
        update_graph!(
            self,
            t,
            1,
            network.get_packets_outcome(),
            outcome_packets_peak
        );
        update_graph!(self, t, 2, network.get_errors_income(), income_errors_peak);
        update_graph!(
            self,
            t,
            3,
            network.get_errors_outcome(),
            outcome_errors_peak
        );
        t.invalidate();

        let mut t = self.in_out_history.borrow_mut();
        update_graph!(self, t, 0, network.get_income(), income_peak);
        update_graph!(self, t, 1, network.get_outcome(), outcome_peak);
        t.invalidate();
    }

    pub fn show(&self) {
        self.popup.present();
    }

    pub fn need_remove(&self) -> bool {
        *self.to_be_removed.borrow()
    }
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

    //
    // NETWORK INFO TAB
    //
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    let close_button = gtk::Button::new_with_label("Close");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

    let tree = gtk::TreeView::new();
    let list_store = gtk::ListStore::new(&[glib::Type::String, glib::Type::String]);

    tree.set_headers_visible(false);
    tree.set_model(Some(&list_store));

    append_text_column(&tree, 0);
    let column = append_text_column(&tree, 1);
    // column.set_property_xalign(1.0);

    list_store.insert_with_values(
        None,
        &[0, 1],
        &[&"income", &format_number(network.get_income())],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[&"total income", &format_number(network.get_total_income())],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[&"income peak", &format_number(network.get_income())],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[&"outcome", &format_number(network.get_outcome())],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"total outcome",
            &format_number(network.get_total_outcome()),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[&"outcome peak", &format_number(network.get_outcome())],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"packets in",
            &format_number_full(network.get_packets_income(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"total packets in",
            &format_number_full(network.get_total_packets_income(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"packets in peak",
            &format_number(network.get_packets_income()),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"packets out",
            &format_number_full(network.get_packets_outcome(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"total packets out",
            &format_number_full(network.get_total_packets_outcome(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"packets out peak",
            &format_number(network.get_packets_outcome()),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"errors in",
            &format_number_full(network.get_errors_income(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"total errors in",
            &format_number_full(network.get_total_errors_income(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"errors in peak",
            &format_number(network.get_errors_income()),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"errors out",
            &format_number_full(network.get_errors_outcome(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"total errors out",
            &format_number_full(network.get_total_errors_outcome(), false),
        ],
    );
    list_store.insert_with_values(
        None,
        &[0, 1],
        &[
            &"errors out peak",
            &format_number(network.get_errors_outcome()),
        ],
    );

    scroll.add(&tree);

    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);

    notebook.create_tab("Information", &vertical_layout);

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
    let mut in_out_history = Graph::new(Some(100.), false); // In case a process uses more than 100%
    in_out_history.set_minimum(Some(100.));

    let mut packets_errors_history = Graph::new(None, false);

    in_out_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "income",
        None,
    );
    in_out_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "outcome",
        None,
    );
    in_out_history.set_label_callbacks(Some(Box::new(|v| {
        if v < 100_000. {
            [
                v.to_string(),
                format!("{}", v / 2.),
                "0".to_string(),
                "KiB".to_string(),
            ]
        } else if v < 10_000_000. {
            [
                format!("{:.1}", v / 1_024f64),
                format!("{:.1}", v / 2_048f64),
                "0".to_string(),
                "MiB".to_string(),
            ]
        } else if v < 10_000_000_000. {
            [
                format!("{:.1}", v / 1_048_576f64),
                format!("{:.1}", v / 2_097_152f64),
                "0".to_string(),
                "GiB".to_string(),
            ]
        } else {
            [
                format!("{:.1}", v / 1_073_741_824f64),
                format!("{:.1}", v / 2_147_483_648f64),
                "0".to_string(),
                "TiB".to_string(),
            ]
        }
    })));
    vertical_layout.add(&gtk::Label::new(Some("Network usage")));
    in_out_history.attach_to(&vertical_layout);
    in_out_history.invalidate();
    let in_out_history = connect_graph(in_out_history);

    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "income packets",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "outcome packets",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "income errors",
        None,
    );
    packets_errors_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "outcome errors",
        None,
    );
    packets_errors_history.set_label_callbacks(Some(Box::new(|v| {
        if v < 100_000. {
            [
                v.to_string(),
                format!("{}", v / 2.),
                "0".to_string(),
                "Ki".to_string(),
            ]
        } else if v < 10_000_000. {
            [
                format!("{:.1}", v / 1_024f64),
                format!("{:.1}", v / 2_048f64),
                "0".to_string(),
                "Mi".to_string(),
            ]
        } else if v < 10_000_000_000. {
            [
                format!("{:.1}", v / 1_048_576f64),
                format!("{:.1}", v / 2_097_152f64),
                "0".to_string(),
                "Gi".to_string(),
            ]
        } else {
            [
                format!("{:.1}", v / 1_073_741_824f64),
                format!("{:.1}", v / 2_147_483_648f64),
                "0".to_string(),
                "Ti".to_string(),
            ]
        }
    })));
    vertical_layout.add(&gtk::Label::new(Some("Extra data")));
    packets_errors_history.attach_to(&vertical_layout);
    packets_errors_history.invalidate();
    let packets_errors_history = connect_graph(packets_errors_history);

    scroll.add(&vertical_layout);
    scroll.connect_show(
        clone!(@weak packets_errors_history, @weak in_out_history => move |_| {
            packets_errors_history.borrow().show_all();
            in_out_history.borrow().show_all();
        }),
    );
    notebook.create_tab("Graphics", &scroll);

    popup.add(&notebook.notebook);
    // To silence the annoying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    popup.get_preferred_width();
    popup.set_size_request(500, 600);

    close_button.connect_clicked(clone!(@weak popup => move |_| {
        popup.destroy();
    }));
    let to_be_removed = Rc::new(RefCell::new(false));
    popup.connect_destroy(clone!(@weak to_be_removed => move |_| {
        *to_be_removed.borrow_mut() = true;
    }));
    popup.set_resizable(true);
    popup.show_all();

    if let Some(adjust) = scroll.get_vadjustment() {
        adjust.set_value(0.);
        scroll.set_vadjustment(Some(&adjust));
    }
    packets_errors_history.connect_to_window_events();
    in_out_history.connect_to_window_events();

    NetworkDialog {
        name: interface_name.to_owned(),
        popup,
        notebook,
        packets_errors_history,
        in_out_history,
        income_peak: Rc::new(RefCell::new(network.get_income())),
        outcome_peak: Rc::new(RefCell::new(network.get_outcome())),
        income_packets_peak: Rc::new(RefCell::new(network.get_packets_income())),
        outcome_packets_peak: Rc::new(RefCell::new(network.get_packets_outcome())),
        income_errors_peak: Rc::new(RefCell::new(network.get_errors_income())),
        outcome_errors_peak: Rc::new(RefCell::new(network.get_errors_outcome())),
        to_be_removed,
    }
}
