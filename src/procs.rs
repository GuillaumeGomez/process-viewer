use glib::object::Cast;
use gtk::{self, Type, Widget};
use gtk::prelude::{BoxExt, ContainerExt, ScrolledWindowExt, TreeModelExt};
use gtk::prelude::{TreeViewSignals, WidgetExt};

use sysinfo::*;

use notebook::NoteBook;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

#[allow(dead_code)]
pub struct Procs {
    pub left_tree: gtk::TreeView,
    pub scroll: gtk::ScrolledWindow,
    pub current_pid: Rc<Cell<Option<i64>>>,
    pub kill_button: gtk::Button,
    pub vertical_layout: gtk::Box,
    pub list_store: gtk::ListStore,
    pub columns: Vec<gtk::TreeViewColumn>,
}

impl Procs {
    pub fn new(proc_list: &HashMap<usize, Process>, note: &mut NoteBook) -> Procs {
        let left_tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let current_pid = Rc::new(Cell::new(None));
        let kill_button = gtk::Button::new_with_label("End task");
        let current_pid1 = current_pid.clone();
        let kill_button1 = kill_button.clone();

        scroll.set_min_content_height(800);
        scroll.set_min_content_width(600);

        let mut columns : Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            Type::I64,       // pid
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
            create_and_fill_model(&list_store, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage,
                                  pro.memory);
        }

        left_tree.set_model(Some(&list_store));
        left_tree.set_headers_visible(true);
        scroll.add(&left_tree);
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

        left_tree.connect_cursor_changed(move |tree_view| {
            let selection = tree_view.get_selection();
            if let Some((model, iter)) = selection.get_selected() {
                let pid = Some(model.get_value(&iter, 0).get().unwrap());
                current_pid1.set(pid);
                kill_button1.set_sensitive(true);
            } else {
                current_pid1.set(None);
                kill_button1.set_sensitive(false);
            }
        });
        kill_button.set_sensitive(false);

        vertical_layout.pack_start(&scroll, true, true, 0);
        vertical_layout.pack_start(&kill_button, false, true, 0);
        let vertical_layout : Widget = vertical_layout.upcast();

        note.create_tab("Process list", &vertical_layout);
        Procs {
            left_tree: left_tree,
            scroll: scroll,
            current_pid: current_pid,
            kill_button: kill_button.clone(),
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

pub fn create_and_fill_model(list_store: &gtk::ListStore, pid: i64, cmdline: &str, name: &str,
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
