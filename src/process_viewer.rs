#![crate_type = "bin"]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{Orientation, Type, Widget};
use gtk::prelude::*;

use sysinfo::*;

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

struct NoteBook {
    notebook: gtk::Notebook,
    tabs: Vec<gtk::Box>,
}

impl NoteBook {
    fn new() -> NoteBook {
        NoteBook {
            notebook: gtk::Notebook::new(),
            tabs: Vec::new(),
        }
    }

    fn create_tab(&mut self, title: &str, widget: &Widget) -> Option<u32> {
        let label = gtk::Label::new(Some(title));
        let tab = gtk::Box::new(Orientation::Horizontal, 0);

        tab.pack_start(&label, false, false, 0);
        tab.show_all();

        let index = self.notebook.append_page(widget, Some(&tab));

        self.tabs.push(tab);

        Some(index)
    }
}

#[allow(dead_code)]
struct Procs {
    left_tree: gtk::TreeView,
    scroll: gtk::ScrolledWindow,
    current_pid: Rc<Cell<Option<i64>>>,
    kill_button: gtk::Button,
    vertical_layout: gtk::Box,
    list_store: gtk::ListStore,
    columns: Vec<gtk::TreeViewColumn>,
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

fn create_and_fill_model(list_store: &gtk::ListStore, pid: i64, cmdline: &str, name: &str,
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

fn update_window(list: &gtk::ListStore, system: &Rc<RefCell<sysinfo::System>>,
                 info: &mut DisplaySysInfo) {
    let mut system = system.borrow_mut();
    system.refresh_all();
    info.update_ram_display(&system);
    info.update_process_display(&system);
    let entries: &HashMap<usize, Process> = system.get_process_list();
    let mut seen: HashSet<usize> = HashSet::new();

    if let Some(mut iter) = list.get_iter_first() {
        let mut valid = true;
        while valid {
            let pid = list.get_value(&iter, 0).get::<i64>().unwrap() as usize;
            if let Some(p) = entries.get(&(pid)) {
                list.set(&iter,
                         &[2, 3, 5],
                         &[&format!("{:.1}", p.cpu_usage), &p.memory, &p.cpu_usage]);
                valid = list.iter_next(&mut iter);
                seen.insert(pid);
            } else {
                valid = list.remove(&mut iter);
            }
        }
    }

    for (pid, pro) in entries.iter() {
        if !seen.contains(pid) {
            create_and_fill_model(list, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage, pro.memory);
        }
    }
}

#[allow(dead_code)]
struct DisplaySysInfo {
    procs : Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram : gtk::ProgressBar,
    swap : gtk::ProgressBar,
    vertical_layout : gtk::Box,
    components: Vec<gtk::Label>,
}

impl DisplaySysInfo {
    pub fn new(sys1: Rc<RefCell<sysinfo::System>>, note: &mut NoteBook) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let ram = gtk::ProgressBar::new();
        let swap = gtk::ProgressBar::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let mut components = vec!();

        ram.set_show_text(true);
        swap.set_show_text(true);
        vertical_layout.set_spacing(5);

        let mut i = 0;
        let mut total = false;

        vertical_layout.pack_start(&gtk::Label::new(Some("Memory usage")), false, false, 15);
        vertical_layout.add(&ram);
        vertical_layout.pack_start(&gtk::Label::new(Some("Swap usage")), false, false, 15);
        vertical_layout.add(&swap);
        vertical_layout.pack_start(&gtk::Label::new(Some("Total CPU usage")), false, false, 15);
        for pro in sys1.borrow().get_processor_list() {
            if total {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];
                let l = gtk::Label::new(Some(&format!("{}", i)));
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);
                horizontal_layout.pack_start(&l, false, false, 5);
                horizontal_layout.pack_start(p, true, true, 5);
                vertical_layout.add(&horizontal_layout);
            } else {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);

                vertical_layout.add(p);
                vertical_layout.pack_start(&gtk::Label::new(Some("Process usage")), false,
                                           false, 15);
                total = true;
            }
            i += 1;
        }
        if sys1.borrow().get_components_list().len() > 0 {
            vertical_layout.pack_start(&gtk::Label::new(Some("Components' temperature")), false, false, 15);
            for component in sys1.borrow().get_components_list() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let temp = gtk::Label::new(Some(&format!("{:.1} °C", component.temperature)));
                horizontal_layout.pack_start(&gtk::Label::new(Some(&component.label)), true, false, 0);
                horizontal_layout.pack_start(&temp, true, false, 0);
                horizontal_layout.set_homogeneous(true);
                vertical_layout.add(&horizontal_layout);
                components.push(temp);
            }
        }

        scroll.add(&vertical_layout);
        let scroll : Widget = scroll.upcast();
        note.create_tab("System usage", &scroll);
        let vertical_layout : gtk::Box = vertical_layout.downcast::<gtk::Box>().unwrap();

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: ram,
            swap: swap,
            vertical_layout: vertical_layout,
            components: components,
        };
        tmp.update_ram_display(&sys1.borrow());
        tmp
    }

    pub fn update_ram_display(&mut self, sys: &sysinfo::System) {
        let total = sys.get_total_memory();
        let used = sys.get_used_memory();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        self.ram.set_text(Some(&disp));
        self.ram.set_fraction(used as f64 / total as f64);

        let total = sys.get_total_swap();
        let used = total - sys.get_used_swap();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        self.swap.set_text(Some(&disp));
        self.swap.set_fraction(used as f64 / total as f64);

        for (component, label) in sys.get_components_list().iter().zip(self.components.iter()) {
            label.set_text(&format!("{:.1} °C", component.temperature));
        }
    }

    pub fn update_process_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let mut i = 0;

        for pro in sys.get_processor_list() {
            v[i].set_text(Some(&format!("{:.1} %", pro.get_cpu_usage() * 100.)));
            v[i].set_show_text(true);
            v[i].set_fraction(pro.get_cpu_usage() as f64);
            i += 1;
        }
    }
}

fn main() {
    gtk::init().expect("GTK couldn't start normally");

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let sys = Rc::new(RefCell::new(sysinfo::System::new()));
    let mut note = NoteBook::new();
    let procs = Procs::new(sys.borrow().get_process_list(), &mut note);
    let current_pid = procs.current_pid.clone();
    let sys1 = sys.clone();

    window.set_title("Process viewer");
    window.set_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    sys.borrow_mut().refresh_all();
    procs.kill_button.connect_clicked(move |_| {
        let sys = sys1.borrow();
        if let Some(process) = current_pid.get().and_then(|pid| sys.get_process(pid)) {
            process.kill(Signal::Kill);
        }
    });

    let mut display_tab = DisplaySysInfo::new(sys.clone(), &mut note);

    gtk::timeout_add(1500, move || {
        // first part, deactivate sorting
        let sorted = procs.list_store.get_sort_column_id();
        procs.list_store.set_unsorted();

        // we update the tree view
        update_window(&procs.list_store, &sys, &mut display_tab);

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            procs.list_store.set_sort_column_id(col, order);
        }
        glib::Continue(true)
    });

    window.add(&note.notebook);
    window.show_all();
    gtk::main();
}
