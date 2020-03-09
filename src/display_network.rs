use gdk_pixbuf::Pixbuf;
use gio::MemoryInputStream;
use glib::{Bytes, Cast, IsA, ToVariant};
use gtk::prelude::{
    BoxExt, ButtonExt, CellLayoutExt, CellRendererExt, ContainerExt, EntryExt, GridExt,
    GtkListStoreExt, GtkListStoreExtManual, OverlayExt, SearchBarExt, TreeModelExt,
    TreeModelFilterExt, TreeSelectionExt, TreeSortableExtManual, TreeViewColumnExt, TreeViewExt,
    WidgetExt,
};
use gtk;
use notebook::NoteBook;
use sysinfo::{NetworkExt, NetworksExt, SystemExt};
use utils::{format_number, format_number_full};

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
    current_network: Rc<RefCell<Option<String>>>,
}

impl Network {
    pub fn new(note: &mut NoteBook) -> Network {
        let tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let info_button = gtk::Button::new_with_label("More information");
        let current_network = Rc::new(RefCell::new(None));

        let filter_button = gtk::Button::new();
        let memory_stream = MemoryInputStream::new_from_bytes(&Bytes::from_static(include_bytes!(
            "../assets/magnifier.png"
        )));
        let image = Pixbuf::new_from_stream_at_scale(
            &memory_stream,
            32,
            32,
            true,
            None::<&gio::Cancellable>,
        );
        if let Ok(image) = image {
            let image = gtk::Image::new_from_pixbuf(Some(&image));
            filter_button.set_image(Some(&image));
            filter_button.set_always_show_image(true);
        } else {
            filter_button.set_label("Filter");
        }

        tree.set_headers_visible(true);
        scroll.add(&tree);

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            glib::Type::String, // name
            glib::Type::String, // in usage
            glib::Type::String, // out usage
            glib::Type::String, // income packets
            glib::Type::String, // outcome packets
            glib::Type::String, // income errors
            glib::Type::String, // outcome errors
            // These two will serve as keys when sorting by interface name and other numerical
            // things.
            glib::Type::String, // name_lowercase
            glib::Type::U64,    // in usage
            glib::Type::U64,    // out usage
            glib::Type::U64,    // income packets
            glib::Type::U64,    // outcome packets
            glib::Type::U64,    // income errors
            glib::Type::U64,    // outcome errors
        ]);

        // The filter model
        let filter_model = gtk::TreeModelFilter::new(&list_store, None);
        filter_model.set_visible_func(|_, _| true);
        // filter_model.set_visible_func(
        //     clone!(@weak filter_entry => @default-return false, move |model, iter| {
        //         if !filter_entry.get_visible() || filter_entry.get_text_length() < 1 {
        //             return true;
        //         }
        //         if let Some(text) = filter_entry.get_text() {
        //             if text.is_empty() {
        //                 return true;
        //             }
        //             let text: &str = text.as_ref();
        //             let pid = model.get_value(iter, 0)
        //                            .get::<u32>()
        //                            .unwrap_or_else(|_| None)
        //                            .map(|p| p.to_string())
        //                            .unwrap_or_else(String::new);
        //             let name = model.get_value(iter, 1)
        //                             .get::<String>()
        //                             .unwrap_or_else(|_| None)
        //                             .map(|s| s.to_lowercase())
        //                             .unwrap_or_else(String::new);
        //             pid.contains(text) ||
        //             text.contains(&pid) ||
        //             name.contains(text) ||
        //             text.contains(&name)
        //         } else {
        //             true
        //         }
        //     }),
        // );

        // For the filtering to be taken into account, we need to add it directly into the
        // "global" model.
        let sort_model = gtk::TreeModelSort::new(&filter_model);
        tree.set_model(Some(&sort_model));

        append_column("name", &mut columns, &tree, Some(200));
        append_column("in usage", &mut columns, &tree, None);
        append_column("out usage", &mut columns, &tree, None);
        append_column("income packets", &mut columns, &tree, None);
        append_column("outcome packets", &mut columns, &tree, None);
        append_column("income errors", &mut columns, &tree, None);
        append_column("outcome errors", &mut columns, &tree, None);

        let columns_len = columns.len();
        for (pos, column) in columns.iter().enumerate() {
            column.set_sort_column_id(pos as i32 + columns_len as i32);
        }

        // filter_entry.connect_property_text_length_notify(move |_| {
        //     filter_model.refilter();
        // });

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

        vertical_layout.pack_start(&scroll, true, true, 0);
        vertical_layout.pack_start(&horizontal_layout, false, true, 0);

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
                }
            ),
        );
        info_button.set_sensitive(false);

        Network {
            list_store,
            current_network,
        }
    }

    pub fn update_networks(&mut self, sys: &sysinfo::System) {
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
        }

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            self.list_store.set_sort_column_id(col, order);
        }
    }
}

fn create_and_fill_model(
    list_store: &gtk::ListStore,
    interface_name: &str,
    in_usage: u64,
    out_usage: u64,
    income_packets: u64,
    outcome_packets: u64,
    income_errors: u64,
    outcome_errors: u64,
) {
    list_store.insert_with_values(
        None,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
        &[
            &interface_name,
            &format_number(in_usage),
            &format_number(out_usage),
            &format_number_full(income_packets, false),
            &format_number_full(outcome_packets, false),
            &format_number_full(income_errors, false),
            &format_number_full(outcome_errors, false),
            // sort part
            &interface_name.to_lowercase(),
            &in_usage,
            &out_usage,
            &income_packets,
            &outcome_packets,
            &income_errors,
            &outcome_errors,
        ],
    );
}
