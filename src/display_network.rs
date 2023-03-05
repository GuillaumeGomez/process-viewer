use crate::network_dialog::{self, NetworkDialog};

use crate::utils::{format_number, format_number_full};
use gtk::glib;
use gtk::prelude::*;
use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn append_column(
    title: &str,
    v: &mut Vec<gtk::TreeViewColumn>,
    tree: &gtk::TreeView,
    max_width: Option<i32>,
) {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    if title != "name" {
        renderer.set_xalign(1.0);
    }

    let column = gtk::TreeViewColumn::builder()
        .title(title)
        .resizable(true)
        .clickable(true)
        .min_width(10)
        .build();
    if let Some(max_width) = max_width {
        column.set_max_width(max_width);
        column.set_expand(true);
    }
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    tree.append_column(&column);
    v.push(column);
}

pub struct Network {
    list_store: gtk::ListStore,
    pub filter_entry: gtk::SearchEntry,
    pub search_bar: gtk::SearchBar,
    dialogs: Rc<RefCell<Vec<NetworkDialog>>>,
}

impl Network {
    pub fn new(stack: &gtk::Stack, sys: &Arc<Mutex<System>>) -> Network {
        let tree = gtk::TreeView::builder().headers_visible(true).build();
        let scroll = gtk::ScrolledWindow::builder().child(&tree).build();
        let info_button = gtk::Button::builder()
            .label("More information")
            .hexpand(true)
            .sensitive(false)
            .css_classes(vec!["button-with-margin".to_owned()])
            .build();
        let current_network = Rc::new(RefCell::new(None));

        // We put the filter entry at the right bottom.
        let filter_entry = gtk::SearchEntry::new();
        let search_bar = gtk::SearchBar::builder()
            .halign(gtk::Align::End)
            .valign(gtk::Align::End)
            .child(&filter_entry)
            .show_close_button(true)
            .build();

        let overlay = gtk::Overlay::builder()
            .child(&scroll)
            .hexpand(true)
            .vexpand(true)
            .build();
        overlay.add_overlay(&search_bar);

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            glib::Type::STRING, // name
            glib::Type::STRING, // received data
            glib::Type::STRING, // transmitted data
            glib::Type::STRING, // received packets
            glib::Type::STRING, // transmitted packets
            glib::Type::STRING, // errors on received
            glib::Type::STRING, // errors on transmitted
            // These two will serve as keys when sorting by interface name and other numerical
            // things.
            glib::Type::STRING, // name_lowercase
            glib::Type::U64,    // received data
            glib::Type::U64,    // transmitted data
            glib::Type::U64,    // received packets
            glib::Type::U64,    // transmitted packets
            glib::Type::U64,    // errors on received
            glib::Type::U64,    // errors on transmitted
        ]);

        // The filter model
        let filter_model = gtk::TreeModelFilter::new(&list_store, None);
        filter_model.set_visible_func(
            glib::clone!(@weak filter_entry => @default-return false, move |model, iter| {
                if !WidgetExt::is_visible(&filter_entry) {
                    return true;
                }
                let text = filter_entry.text();
                if text.is_empty() {
                    return true;
                }
                let text: &str = text.as_ref();
                // TODO: Maybe add an option to make searches case sensitive?
                let name = model.get_value(iter, 0)
                                .get::<String>()
                                .map(|s| s.to_lowercase())
                                .ok()
                                .unwrap_or_default();
                name.contains(text)
            }),
        );

        // For the filtering to be taken into account, we need to add it directly into the
        // "global" model.
        let sort_model = gtk::TreeModelSort::with_model(&filter_model);
        tree.set_model(Some(&sort_model));
        tree.set_search_entry(Some(&filter_entry));

        append_column("name", &mut columns, &tree, Some(200));
        append_column("received data", &mut columns, &tree, None);
        append_column("transmitted data", &mut columns, &tree, None);
        append_column("received packets", &mut columns, &tree, None);
        append_column("transmitted packets", &mut columns, &tree, None);
        append_column("errors on received", &mut columns, &tree, None);
        append_column("errors on transmitted", &mut columns, &tree, None);

        let columns_len = columns.len();
        for (pos, column) in columns.iter().enumerate() {
            column.set_sort_column_id(pos as i32 + columns_len as i32);
        }

        // Sort by network name by default.
        sort_model.set_sort_column_id(
            gtk::SortColumn::Index(columns_len as _),
            gtk::SortType::Ascending,
        );

        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

        vertical_layout.append(&overlay);
        vertical_layout.append(&info_button);

        filter_entry.connect_search_changed(move |_| {
            filter_model.refilter();
        });

        stack.add_titled(&vertical_layout, Some("Networks"), "Networks");

        tree.connect_cursor_changed(
            glib::clone!(@weak current_network, @weak info_button => move |tree_view| {
                let selection = tree_view.selection();
                let (name, ret) = if let Some((model, iter)) = selection.selected() {
                    if let Ok(x) = model.get_value(&iter, 0).get::<String>() {
                        (Some(x), true)
                    } else {
                        (None, false)
                    }
                } else {
                    (None, false)
                };
                *current_network.borrow_mut() = name;
                info_button.set_sensitive(ret);
            }),
        );

        let dialogs = Rc::new(RefCell::new(Vec::new()));

        info_button.connect_clicked(glib::clone!(@weak dialogs, @weak sys => move |_| {
            let current_network = current_network.borrow();
            if let Some(ref interface_name) = *current_network {
                create_network_dialog(
                    &mut dialogs.borrow_mut(),
                    interface_name,
                    &sys.lock().expect("failed to lock for new network dialog"),
                );
            }
        }));

        tree.connect_row_activated(
            glib::clone!(@weak sys, @weak dialogs => move |tree_view, path, _| {
                let model = tree_view.model().expect("couldn't get model");
                let iter = model.iter(path).expect("couldn't get iter");
                let interface_name = model.get_value(&iter, 0)
                    .get::<String>()
                    .expect("Model::get failed");
                create_network_dialog(
                    &mut dialogs.borrow_mut(),
                    &interface_name,
                    &sys.lock().expect("failed to lock for new network dialog (from tree)"),
                );
            }),
        );

        Network {
            list_store,
            filter_entry,
            search_bar,
            dialogs,
        }
    }

    pub fn update_networks(&mut self, sys: &System) {
        // first part, deactivate sorting
        let sorted = TreeSortableExtManual::sort_column_id(&self.list_store);
        self.list_store.set_unsorted();

        let mut seen: HashSet<String> = HashSet::new();
        let networks = sys.networks();

        if let Some(iter) = self.list_store.iter_first() {
            let mut valid = true;
            while valid {
                let name = match self.list_store.get_value(&iter, 0).get::<glib::GString>() {
                    Ok(n) => n,
                    _ => {
                        valid = self.list_store.iter_next(&iter);
                        continue;
                    }
                };
                if let Some((_, data)) = networks
                    .iter()
                    .find(|(interface_name, _)| interface_name.as_str() == name)
                {
                    self.list_store.set(
                        &iter,
                        &[
                            (1, &format_number(data.received())),
                            (2, &format_number(data.transmitted())),
                            (3, &format_number_full(data.packets_received(), false)),
                            (4, &format_number_full(data.packets_transmitted(), false)),
                            (5, &format_number_full(data.errors_on_received(), false)),
                            (6, &format_number_full(data.errors_on_transmitted(), false)),
                            (8, &data.received()),
                            (9, &data.transmitted()),
                            (10, &data.packets_received()),
                            (11, &data.packets_transmitted()),
                            (12, &data.errors_on_received()),
                            (13, &data.errors_on_transmitted()),
                        ],
                    );
                    valid = self.list_store.iter_next(&iter);
                    seen.insert(name.to_string());
                } else {
                    valid = self.list_store.remove(&iter);
                }
            }
        }

        for (interface_name, data) in networks.iter() {
            if !seen.contains(interface_name.as_str()) {
                create_and_fill_model(
                    &self.list_store,
                    interface_name,
                    data.received(),
                    data.transmitted(),
                    data.packets_received(),
                    data.packets_transmitted(),
                    data.errors_on_received(),
                    data.errors_on_transmitted(),
                );
            }
            if let Some(dialog) = self
                .dialogs
                .borrow()
                .iter()
                .find(|x| x.name == *interface_name)
            {
                dialog.update(data);
            }
        }

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            self.list_store.set_sort_column_id(col, order);
        }

        self.dialogs.borrow_mut().retain(|x| !x.need_remove());
    }
}

#[allow(clippy::too_many_arguments)]
fn create_and_fill_model(
    list_store: &gtk::ListStore,
    interface_name: &str,
    in_usage: u64,
    out_usage: u64,
    incoming_packets: u64,
    outgoing_packets: u64,
    incoming_errors: u64,
    outgoing_errors: u64,
) {
    list_store.insert_with_values(
        None,
        &[
            (0, &interface_name),
            (1, &format_number(in_usage)),
            (2, &format_number(out_usage)),
            (3, &format_number_full(incoming_packets, false)),
            (4, &format_number_full(outgoing_packets, false)),
            (5, &format_number_full(incoming_errors, false)),
            (6, &format_number_full(outgoing_errors, false)),
            // sort part
            (7, &interface_name.to_lowercase()),
            (8, &in_usage),
            (9, &out_usage),
            (10, &incoming_packets),
            (11, &outgoing_packets),
            (12, &incoming_errors),
            (13, &outgoing_errors),
        ],
    );
}

fn create_network_dialog(dialogs: &mut Vec<NetworkDialog>, interface_name: &str, sys: &System) {
    for dialog in dialogs.iter() {
        if dialog.name == interface_name {
            dialog.show();
            return;
        }
    }
    if let Some((_, data)) = sys
        .networks()
        .iter()
        .find(|(name, _)| name.as_str() == interface_name)
    {
        dialogs.push(network_dialog::create_network_dialog(data, interface_name));
    } else {
        eprintln!("couldn't find {}...", interface_name);
    }
}
