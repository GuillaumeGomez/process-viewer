use network_dialog::{self, NetworkDialog};

use gtk;
use gtk::prelude::{
    BoxExt, ButtonExt, CellLayoutExt, CellRendererExt, ContainerExt, EntryExt, GridExt,
    GtkListStoreExt, GtkListStoreExtManual, GtkWindowExt, OverlayExt, SearchBarExt, TreeModelExt,
    TreeModelFilterExt, TreeSelectionExt, TreeSortableExtManual, TreeViewColumnExt, TreeViewExt,
    WidgetExt,
};
use notebook::NoteBook;
use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
use utils::{create_button_with_image, format_number, format_number_full};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

fn append_column(
    title: &str,
    v: &mut Vec<gtk::TreeViewColumn>,
    tree: &gtk::TreeView,
    max_width: Option<i32>,
) {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    if title != "name" {
        renderer.set_property_xalign(1.0);
    }

    let column = gtk::TreeViewColumn::new();
    column.set_title(title);
    column.set_resizable(true);
    if let Some(max_width) = max_width {
        column.set_max_width(max_width);
        column.set_expand(true);
    }
    column.set_min_width(10);
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    column.set_clickable(true);
    column.set_sort_column_id(id);
    tree.append_column(&column);
    v.push(column);
}

pub struct Network {
    list_store: gtk::ListStore,
    pub filter_entry: gtk::Entry,
    pub search_bar: gtk::SearchBar,
    dialogs: Rc<RefCell<Vec<NetworkDialog>>>,
}

impl Network {
    pub fn new(
        note: &mut NoteBook,
        window: &gtk::ApplicationWindow,
        sys: &Rc<RefCell<System>>,
    ) -> Network {
        let tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let info_button = gtk::Button::new_with_label("More information");
        let current_network = Rc::new(RefCell::new(None));

        let filter_button =
            create_button_with_image(include_bytes!("../assets/magnifier.png"), "Filter");

        // TODO: maybe add an 'X' button to close search as well?
        let overlay = gtk::Overlay::new();
        let filter_entry = gtk::Entry::new();
        let search_bar = gtk::SearchBar::new();

        // We put the filter entry at the right bottom.
        filter_entry.set_halign(gtk::Align::End);
        filter_entry.set_valign(gtk::Align::End);
        filter_entry.hide(); // By default, we don't show it.
        search_bar.connect_entry(&filter_entry);
        search_bar.set_show_close_button(true);

        overlay.add_overlay(&filter_entry);

        tree.set_headers_visible(true);
        scroll.add(&tree);
        overlay.add(&scroll);

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            glib::Type::String, // name
            glib::Type::String, // in usage
            glib::Type::String, // out usage
            glib::Type::String, // incoming packets
            glib::Type::String, // outgoing packets
            glib::Type::String, // incoming errors
            glib::Type::String, // outgoing errors
            // These two will serve as keys when sorting by interface name and other numerical
            // things.
            glib::Type::String, // name_lowercase
            glib::Type::U64,    // in usage
            glib::Type::U64,    // out usage
            glib::Type::U64,    // incoming packets
            glib::Type::U64,    // outgoing packets
            glib::Type::U64,    // incoming errors
            glib::Type::U64,    // outgoing errors
        ]);

        // The filter model
        let filter_model = gtk::TreeModelFilter::new(&list_store, None);
        filter_model.set_visible_func(
            clone!(@weak filter_entry => @default-return false, move |model, iter| {
                if !filter_entry.get_visible() || filter_entry.get_text_length() < 1 {
                    return true;
                }
                if let Some(text) = filter_entry.get_text() {
                    if text.is_empty() {
                        return true;
                    }
                    let text: &str = text.as_ref();
                    // TODO: Maybe add an option to make searches case sensitive?
                    let name = model.get_value(iter, 0)
                                    .get::<String>()
                                    .unwrap_or_else(|_| None)
                                    .map(|s| s.to_lowercase())
                                    .unwrap_or_else(String::new);
                    name.contains(text)
                } else {
                    true
                }
            }),
        );

        // For the filtering to be taken into account, we need to add it directly into the
        // "global" model.
        let sort_model = gtk::TreeModelSort::new(&filter_model);
        tree.set_model(Some(&sort_model));

        append_column("name", &mut columns, &tree, Some(200));
        append_column("in usage", &mut columns, &tree, None);
        append_column("out usage", &mut columns, &tree, None);
        append_column("incoming packets", &mut columns, &tree, None);
        append_column("outgoing packets", &mut columns, &tree, None);
        append_column("incoming errors", &mut columns, &tree, None);
        append_column("outgoing errors", &mut columns, &tree, None);

        let columns_len = columns.len();
        for (pos, column) in columns.iter().enumerate() {
            column.set_sort_column_id(pos as i32 + columns_len as i32);
        }

        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let horizontal_layout = gtk::Grid::new();

        horizontal_layout.attach(&info_button, 0, 0, 4, 1);
        horizontal_layout.attach_next_to(
            &filter_button,
            Some(&info_button),
            gtk::PositionType::Right,
            1,
            1,
        );
        horizontal_layout.set_column_homogeneous(true);

        vertical_layout.pack_start(&overlay, true, true, 0);
        vertical_layout.pack_start(&horizontal_layout, false, true, 0);

        filter_entry.connect_property_text_length_notify(move |_| {
            filter_model.refilter();
        });

        note.create_tab("Networks", &vertical_layout);

        tree.connect_cursor_changed(
            clone!(@weak current_network, @weak info_button => move |tree_view| {
                let selection = tree_view.get_selection();
                let (name, ret) = if let Some((model, iter)) = selection.get_selected() {
                    if let Ok(Some(x)) = model.get_value(&iter, 0).get::<String>() {
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
        info_button.set_sensitive(false);

        filter_button.connect_clicked(clone!(@weak filter_entry, @weak window => move |_| {
            if filter_entry.get_visible() {
                filter_entry.hide();
            } else {
                filter_entry.show_all();
                window.set_focus(Some(&filter_entry));
            }
        }));

        let dialogs = Rc::new(RefCell::new(Vec::new()));

        info_button.connect_clicked(clone!(@weak dialogs, @weak sys => move |_| {
            let current_network = current_network.borrow();
            if let Some(ref interface_name) = *current_network {
                println!("create network dialog for {}", interface_name);
                create_network_dialog(&mut *dialogs.borrow_mut(), interface_name, &*sys.borrow());
            }
        }));

        tree.connect_row_activated(
            clone!(@weak sys, @weak dialogs => move |tree_view, path, _| {
                let model = tree_view.get_model().expect("couldn't get model");
                let iter = model.get_iter(path).expect("couldn't get iter");
                let interface_name = model.get_value(&iter, 0)
                                            .get::<String>()
                                            .expect("Model::get failed")
                                            .expect("failed to get value from model");
                create_network_dialog(&mut *dialogs.borrow_mut(), &interface_name, &*sys.borrow());
            }),
        );

        Network {
            list_store,
            filter_entry,
            search_bar,
            dialogs,
        }
    }

    pub fn hide_filter(&self) {
        self.filter_entry.hide();
        self.filter_entry.set_text("");
        self.search_bar.set_search_mode(false);
    }

    pub fn update_networks(&mut self, sys: &System) {
        // first part, deactivate sorting
        let sorted = TreeSortableExtManual::get_sort_column_id(&self.list_store);
        self.list_store.set_unsorted();

        let mut seen: HashSet<String> = HashSet::new();
        let networks = sys.get_networks();

        if let Some(iter) = self.list_store.get_iter_first() {
            let mut valid = true;
            while valid {
                if let Some(name) = match self.list_store.get_value(&iter, 0).get::<glib::GString>()
                {
                    Ok(n) => n,
                    _ => {
                        valid = self.list_store.iter_next(&iter);
                        continue;
                    }
                } {
                    if let Some((_, data)) = networks
                        .iter()
                        .find(|(interface_name, _)| interface_name.as_str() == name)
                    {
                        self.list_store.set(
                            &iter,
                            &[1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13],
                            &[
                                &format_number(data.get_income()),
                                &format_number(data.get_outcome()),
                                &format_number_full(data.get_packets_income(), false),
                                &format_number_full(data.get_packets_outcome(), false),
                                &format_number_full(data.get_errors_income(), false),
                                &format_number_full(data.get_errors_outcome(), false),
                                &data.get_income(),
                                &data.get_outcome(),
                                &data.get_packets_income(),
                                &data.get_packets_outcome(),
                                &data.get_errors_income(),
                                &data.get_errors_outcome(),
                            ],
                        );
                        valid = self.list_store.iter_next(&iter);
                        seen.insert(name.to_string());
                    } else {
                        valid = self.list_store.remove(&iter);
                    }
                }
            }
        }

        for (interface_name, data) in networks.iter() {
            if !seen.contains(interface_name.as_str()) {
                create_and_fill_model(
                    &self.list_store,
                    interface_name,
                    data.get_income(),
                    data.get_outcome(),
                    data.get_packets_income(),
                    data.get_packets_outcome(),
                    data.get_errors_income(),
                    data.get_errors_outcome(),
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
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
        &[
            &interface_name,
            &format_number(in_usage),
            &format_number(out_usage),
            &format_number_full(incoming_packets, false),
            &format_number_full(outgoing_packets, false),
            &format_number_full(incoming_errors, false),
            &format_number_full(outgoing_errors, false),
            // sort part
            &interface_name.to_lowercase(),
            &in_usage,
            &out_usage,
            &incoming_packets,
            &outgoing_packets,
            &incoming_errors,
            &outgoing_errors,
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
        .get_networks()
        .iter()
        .find(|(name, _)| name.as_str() == interface_name)
    {
        dialogs.push(network_dialog::create_network_dialog(data, interface_name));
    } else {
        eprintln!("couldn't find {}...", interface_name);
    }
}
