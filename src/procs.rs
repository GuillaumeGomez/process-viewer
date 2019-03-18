use glib::object::Cast;
use gtk::{self, Type};
use gtk::{
    BoxExt, CellLayoutExt, CellRendererExt, ContainerExt, EntryExt, GridExt,
    GtkListStoreExtManual, OverlayExt, SearchBarExt, TreeModelExt, TreeModelFilterExt,
    TreeSelectionExt, TreeViewColumnExt, TreeViewExt, WidgetExt,
};

use sysinfo::*;

use notebook::NoteBook;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(dead_code)]
pub struct Procs {
    pub left_tree: gtk::TreeView,
    pub scroll: gtk::ScrolledWindow,
    pub current_pid: Rc<Cell<Option<Pid>>>,
    pub kill_button: gtk::Button,
    pub info_button: gtk::Button,
    pub vertical_layout: gtk::Box,
    pub list_store: gtk::ListStore,
    pub columns: Vec<gtk::TreeViewColumn>,
    pub filter_entry: gtk::Entry,
    pub search_bar: gtk::SearchBar,
}

impl Procs {
    pub fn new(proc_list: &HashMap<Pid, Process>, note: &mut NoteBook) -> Procs {
        let left_tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let current_pid = Rc::new(Cell::new(None));
        let kill_button = gtk::Button::new_with_label("End task");
        let info_button = gtk::Button::new_with_label("More information");

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

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            Type::U32,       // pid
            Type::String,    // name
            Type::String,    // CPU
            Type::U64,       // mem
            // These two will serve as keys when sorting by process name and CPU usage.
            Type::String,    // name_lowercase
            Type::F32,       // CPU_f32
        ]);

        append_column("pid", &mut columns, &left_tree, None);
        append_column("process name", &mut columns, &left_tree, Some(200));
        append_column("cpu usage", &mut columns, &left_tree, None);
        append_column("memory usage (in kB)", &mut columns, &left_tree, None);

        // When we click the "name" column the order is defined by the
        // "name_lowercase" effectively making the built-in comparator ignore case.
        columns[1].set_sort_column_id(4);
        // Likewise clicking the "CPU" column sorts by the "CPU_f32" one because
        // we want the order to be numerical not lexicographical.
        columns[2].set_sort_column_id(5);

        for pro in proc_list.values() {
            create_and_fill_model(&list_store, pro.pid().as_u32(), &format!("{:?}", &pro.cmd()),
                                  &pro.name(), pro.cpu_usage(), pro.memory());
        }

        left_tree.set_headers_visible(true);
        //let filter = gtk::TreeModelFilter::new(&list_store, None);
        scroll.add(&left_tree);
        overlay.add(&scroll);
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let horizontal_layout = gtk::Grid::new();
        /*let list_store1 = list_store.clone();

        filter.set_modify_func(|_, iter, value| {
            list_store1.get_value(&iter, 4).get::<String>()
                                           .expect("get_value from list_store failed")
                       .contains(value.get::<String>().expect("contains.get failed").to_lowercase())
        });*/

        left_tree.connect_cursor_changed(
            clone!(current_pid, kill_button, info_button => move |tree_view| {
                let selection = tree_view.get_selection();
                let (pid, ret) = if let Some((model, iter)) = selection.get_selected() {
                    if let Some(x) = model.get_value(&iter, 0).get::<u32>().map(|x| x as Pid) {
                        (Some(x), true)
                    } else {
                        (None, false)
                    }
                } else {
                    (None, false)
                };
                current_pid.set(pid);
                kill_button.set_sensitive(ret);
                info_button.set_sensitive(ret);
            }
        ));
        kill_button.set_sensitive(false);
        info_button.set_sensitive(false);

        vertical_layout.pack_start(&overlay, true, true, 0);
        horizontal_layout.attach(&info_button, 0, 0, 2, 1);
        horizontal_layout.attach_next_to(&kill_button, Some(&info_button),
                                         gtk::PositionType::Right, 2, 1);
        horizontal_layout.set_column_homogeneous(true);
        vertical_layout.pack_start(&horizontal_layout, false, true, 0);

        // The filter part.
        let filter_model = gtk::TreeModelFilter::new(&list_store, None);
        filter_model.set_visible_func(clone!(filter_entry => move |model, iter| {
            if !filter_entry.get_visible() || filter_entry.get_text_length() < 1 {
                return true;
            }
            if let Some(text) = filter_entry.get_text() {
                if text.is_empty() {
                    return true;
                }
                let text: &str = text.as_ref();
                // TODO: Maybe add an option to make searches case sensitive?
                let pid = model.get_value(iter, 0)
                               .get::<u32>()
                               .map(|p| p.to_string())
                               .unwrap_or_else(String::new);
                let name = model.get_value(iter, 1)
                                .get::<String>()
                                .map(|s| s.to_lowercase())
                                .unwrap_or_else(String::new);
                pid.contains(text) ||
                text.contains(&pid) ||
                name.contains(text) ||
                text.contains(&name)
            } else {
                true
            }
        }));
        // For the filtering to be taken into account, we need to add it directly into the
        // "global" model.
        left_tree.set_model(Some(&filter_model));

        filter_entry.connect_property_text_length_notify(move |_| {
            filter_model.refilter();
        });

        note.create_tab("Process list", &vertical_layout);

        Procs {
            left_tree,
            scroll,
            current_pid,
            kill_button,
            info_button,
            vertical_layout: vertical_layout.downcast::<gtk::Box>().expect("downcast failed"),
            list_store,
            columns,
            filter_entry,
            search_bar,
        }
    }
}

fn append_column(title: &str, v: &mut Vec<gtk::TreeViewColumn>, left_tree: &gtk::TreeView,
                 max_width: Option<i32>) {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    if title != "process name" {
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
    left_tree.append_column(&column);
    v.push(column);
}

pub fn create_and_fill_model(list_store: &gtk::ListStore, pid: u32, cmdline: &str, name: &str,
                             cpu: f32, memory: u64) {
    if cmdline.is_empty() || name.is_empty() {
        return;
    }
    list_store.insert_with_values(None,
                                  &[0, 1, 2, 3, 4, 5],
                                  &[&pid,
                                    &name,
                                    &format!("{:.1}", cpu),
                                    &memory,
                                    &name.to_lowercase(),
                                    &cpu
                                   ]);
}
