use glib::object::Cast;
use gtk::{self, Type, Widget};
use gtk::{BoxExt, CellLayoutExt, ContainerExt, ScrolledWindowExt, TreeModelExt, WidgetExt};

use sysinfo::*;

use notebook::NoteBook;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(dead_code)]
pub struct Procs {
    pub left_tree: gtk::TreeView,
    pub scroll: gtk::ScrolledWindow,
    pub current_pid: Rc<Cell<Option<i32>>>,
    pub kill_button: gtk::Button,
    pub info_button: gtk::Button,
    pub vertical_layout: gtk::Box,
    pub list_store: gtk::ListStore,
    pub columns: Vec<gtk::TreeViewColumn>,
}

impl Procs {
    pub fn new(proc_list: &HashMap<i32, Process>, note: &mut NoteBook) -> Procs {
        let left_tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let current_pid = Rc::new(Cell::new(None));
        let kill_button = gtk::Button::new_with_label("End task");
        let info_button = gtk::Button::new_with_label("More information");
        let current_pid1 = current_pid.clone();
        let kill_button1 = kill_button.clone();
        let info_button1 = info_button.clone();

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            Type::I32,       // pid
            Type::String,    // name
            Type::String,    // CPU
            Type::U32,       // mem
            // These two will serve as keys when sorting by process name and CPU usage.
            Type::String,    // name_lowercase
            Type::F32,       // CPU_f32
        ]);

        append_column("pid", &mut columns, &left_tree);
        append_column("process name", &mut columns, &left_tree);
        append_column("cpu usage", &mut columns, &left_tree);
        append_column("memory usage (in kB)", &mut columns, &left_tree);

        // When we click the "name" column the order is defined by the
        // "name_lowercase" effectively making the built-in comparator ignore case.
        columns[1].set_sort_column_id(4);
        // Likewise clicking the "CPU" column sorts by the "CPU_f32" one because
        // we want the order to be numerical not lexicographical.
        columns[2].set_sort_column_id(5);

        for (_, pro) in proc_list {
            create_and_fill_model(&list_store, pro.pid, &format!("{:?}", &pro.cmd), &pro.name,
                                  pro.cpu_usage, pro.memory);
        }

        left_tree.set_model(Some(&list_store));
        left_tree.set_headers_visible(true);
        //let filter = gtk::TreeModelFilter::new(&list_store, None);
        scroll.add(&left_tree);
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let horizontal_layout = gtk::Grid::new();
        /*let list_store1 = list_store.clone();

        filter.set_modify_func(|_, iter, value| {
            list_store1.get_value(&iter, 4).get::<String>().unwrap()
                       .contains(value.get::<String>().unwrap().to_lowercase())
        });*/

        left_tree.connect_cursor_changed(move |tree_view| {
            let selection = tree_view.get_selection();
            let (pid, ret) = if let Some((model, iter)) = selection.get_selected() {
                if let Some(x) = model.get_value(&iter, 0).get() {
                    (Some(x), true)
                } else {
                    (None, false)
                }
            } else {
                (None, false)
            };
            current_pid1.set(pid);
            kill_button1.set_sensitive(ret);
            info_button1.set_sensitive(ret);
        });
        kill_button.set_sensitive(false);
        info_button.set_sensitive(false);

        vertical_layout.pack_start(&scroll, true, true, 0);
        horizontal_layout.attach(&info_button, 0, 0, 2, 1);
        horizontal_layout.attach_next_to(&kill_button, Some(&info_button),
                                         gtk::PositionType::Right, 2, 1);
        horizontal_layout.set_column_homogeneous(true);
        vertical_layout.pack_start(&horizontal_layout, false, true, 0);

        let vertical_layout : Widget = vertical_layout.upcast();
        note.create_tab("Process list", &vertical_layout);

        Procs {
            left_tree: left_tree,
            scroll: scroll,
            current_pid: current_pid,
            kill_button: kill_button,
            info_button: info_button,
            vertical_layout: vertical_layout.downcast::<gtk::Box>().unwrap(),
            list_store: list_store,
            columns: columns,
        }
    }
}

fn append_column(title: &str, v: &mut Vec<gtk::TreeViewColumn>, left_tree: &gtk::TreeView) {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    let column = gtk::TreeViewColumn::new();
    column.set_title(title);
    column.set_resizable(true);
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    column.set_clickable(true);
    column.set_sort_column_id(id);
    left_tree.append_column(&column);
    v.push(column);
}

pub fn create_and_fill_model(list_store: &gtk::ListStore, pid: i32, cmdline: &str, name: &str,
                             cpu: f32, memory: u64) {
    if cmdline.len() < 1 {
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
