#![crate_type = "bin"]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{Orientation, SortColumn, TreeModel, TreeSortable, Widget};
use gtk::prelude::*;

use sysinfo::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::cmp::Ordering;

macro_rules! set_sort {
    ($id:expr, $columns:expr) => {{
        $columns.get($id).unwrap().set_sort_column_id($id);
    }};
    ($id:expr, $left_tree:expr, $val:ty, $columns:expr) => {{
        let model = $left_tree.get_model().unwrap()
                              .downcast::<TreeSortable>().unwrap();
        model.set_sort_func(SortColumn::Index($id), |m, it1, it2| {
            let it1 : $val = m.clone().upcast::<TreeModel>().get_value(it1, $id).get().unwrap_or(String::new());
            let it2 : $val = m.clone().upcast::<TreeModel>().get_value(it2, $id).get().unwrap_or(String::new());
            it1.to_lowercase().cmp(&it2.to_uppercase())
        });
        $columns.get($id).unwrap().set_sort_column_id($id);
    }};
    // ultra super ugly
    ($id:expr, $left_tree:expr, $val:ty, $columns:expr, $fill:expr) => {{
        let model = $left_tree.get_model().unwrap()
                              .downcast::<TreeSortable>().unwrap();
        model.set_sort_func(SortColumn::Index($id), |m, it1, it2| {
            let it1 : $val = m.clone().upcast::<TreeModel>().get_value(it1, $id).get().unwrap_or("0".to_owned());
            let it2 : $val = m.clone().upcast::<TreeModel>().get_value(it2, $id).get().unwrap_or("0".to_owned());
            let it1 : f32 = it1.parse().unwrap();
            let it2 : f32 = it2.parse().unwrap();
            if it1.lt(&it2) {
                Ordering::Less
            } else if it1.gt(&it2) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        $columns.get($id).unwrap().set_sort_column_id($id);
    }};
}

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

    fn create_tab<'a>(&mut self, title: &'a str, widget: &Widget) -> Option<u32> {
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
    current_pid: Rc<RefCell<Option<i64>>>,
    kill_button: Rc<RefCell<gtk::Button>>,
    vertical_layout: gtk::Box,
    list_store: gtk::ListStore,
    columns: Vec<gtk::TreeViewColumn>,
}

impl Procs {
    pub fn new<'a>(proc_list: &HashMap<usize, Process>, note: &mut NoteBook) -> Procs {
        let left_tree = gtk::TreeView::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let current_pid = Rc::new(RefCell::new(None));
        let kill_button = Rc::new(RefCell::new(gtk::Button::new_with_label("End task")));
        let current_pid1 = current_pid.clone();
        let current_pid2 = current_pid.clone();
        let kill_button1 = kill_button.clone();

        scroll.set_min_content_height(800);
        scroll.set_min_content_width(600);

        let mut columns : Vec<gtk::TreeViewColumn> = Vec::new();

        append_column("pid", &mut columns, &left_tree);
        append_column("process name", &mut columns, &left_tree);
        append_column("cpu usage", &mut columns, &left_tree);
        append_column("memory usage (in kB)", &mut columns, &left_tree);

        let mut list_store = gtk::ListStore::new(&[glib::Type::I64, glib::Type::String,
                                                   glib::Type::String, glib::Type::U32]);
        for (_, pro) in proc_list {
            create_and_fill_model(&mut list_store, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage,
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
                *(current_pid1.borrow_mut()) = pid;
                (*kill_button1.borrow()).set_sensitive(true);
            } else {
                *(current_pid1.borrow_mut()) = None;
                (*kill_button1.borrow()).set_sensitive(false);
            }
        });
        set_sort!(0, columns);
        set_sort!(3, columns);
        set_sort!(1, left_tree, String, columns);
        set_sort!(2, left_tree, String, columns, 1);
        (*kill_button.borrow()).set_sensitive(false);

        vertical_layout.add(&scroll);
        vertical_layout.add(&(*kill_button.borrow()));
        let vertical_layout : Widget = vertical_layout.upcast();

        note.create_tab("Process list", &vertical_layout);
        Procs {
            left_tree: left_tree,
            scroll: scroll,
            current_pid: current_pid2.clone(),
            kill_button: kill_button.clone(),
            vertical_layout: vertical_layout.downcast::<gtk::Box>().unwrap(),
            list_store: list_store,
            columns: columns,
        }
    }
}

fn append_column(title: &str, v: &mut Vec<gtk::TreeViewColumn>, left_tree: &gtk::TreeView) {
    let l = v.len();
    let renderer = gtk::CellRendererText::new();

    v.push(gtk::TreeViewColumn::new());
    let tmp = v.get_mut(l).unwrap();

    tmp.set_title(title);
    tmp.set_resizable(true);
    tmp.pack_start(&renderer, true);
    tmp.add_attribute(&renderer, "text", l as i32);
    tmp.set_clickable(true);
    left_tree.append_column(&tmp);
}

fn create_and_fill_model(list_store: &mut gtk::ListStore, pid: i64, cmdline: &str, name: &str,
                         cpu: f32, memory: u64) {
    if cmdline.len() < 1 {
        return;
    }

    let val1 = pid.to_value();
    let val2 = memory.to_value();
    list_store.insert_with_values(None,
                                  &[0, 1, 2, 3],
                                  &[&val1,
                                    &name.to_value(),
                                    &format!("{:.1}", cpu).to_value(),
                                    &val2]);
}

fn update_window(list: &mut gtk::ListStore, system: &Rc<RefCell<sysinfo::System>>,
                 info: &mut DisplaySysInfo) {
    system.borrow_mut().refresh_all();
    let mut entries : HashMap<usize, Process> = system.borrow().get_process_list().clone();
    let mut nb = list.iter_n_children(None);

    info.update_ram_display(&system.borrow());
    info.update_process_display(&system.borrow());

    let mut i = 0;
    while i < nb {
        if let Some(mut iter) = list.iter_nth_child(None, i) {
            if let Some(pid) = list.get_value(&iter, 0).get::<i64>() {
                match entries.get(&(pid as usize)) {
                    Some(p) => {
                        let val2 = p.memory.to_value();
                        list.set(&iter,
                                 &[2, 3],
                                 &[&format!("{:.1}", p.cpu_usage).to_value(), &val2]);
                    }
                    None => {
                        list.remove(&mut iter);
                        nb = list.iter_n_children(None);
                        continue
                    }
                }
                entries.remove(&(pid as usize));
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    for (_, pro) in entries {
        create_and_fill_model(list, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage, pro.memory);
    }
}

#[allow(dead_code)]
struct DisplaySysInfo {
    procs : Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram : Rc<RefCell<gtk::ProgressBar>>,
    swap : Rc<RefCell<gtk::ProgressBar>>,
    vertical_layout : Rc<RefCell<gtk::Box>>,
}

impl DisplaySysInfo {
    pub fn new(sys1: Rc<RefCell<sysinfo::System>>, note: &mut NoteBook) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let ram = gtk::ProgressBar::new();
        let swap = gtk::ProgressBar::new();

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

        let vertical_layout : Widget = vertical_layout.upcast();
        note.create_tab("System usage", &vertical_layout);
        let vertical_layout : gtk::Box = vertical_layout.downcast::<gtk::Box>().unwrap();

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: Rc::new(RefCell::new(ram)),
            swap: Rc::new(RefCell::new(swap)),
            vertical_layout: Rc::new(RefCell::new(vertical_layout)),
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

        (*self.ram.borrow()).set_text(Some(&disp));
        (*self.ram.borrow()).set_fraction(used as f64 / total as f64);

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

        (*self.swap.borrow()).set_text(Some(&disp));
        (*self.swap.borrow()).set_fraction(used as f64 / total as f64);
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
    let mut procs = Procs::new((*sys.borrow()).get_process_list(), &mut note);
    let current_pid2 = procs.current_pid.clone();
    let sys1 = sys.clone();
    let sys2 = sys.clone();

    window.set_title("Process viewer");
    window.set_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    sys.borrow_mut().refresh_all();
    (*procs.kill_button.borrow()).connect_clicked(move |_| {
        if (*current_pid2.borrow()).is_some() {
            let s = (*current_pid2.borrow()).clone();
            match sys1.borrow().get_process(s.unwrap()) {
                Some(p) => {
                    p.kill(Signal::Kill);
                },
                None => {}
            };
        }
    });

    let display_tab = DisplaySysInfo::new(sys2.clone(), &mut note);
    let m_display_tab = Rc::new(RefCell::new(display_tab));

    gtk::timeout_add(1500, move || {
        // first part, deactivate sorting
        let model = procs.left_tree.get_model().unwrap()
                                   .downcast::<TreeSortable>().unwrap();
        let sorted = model.get_sort_column_id();
        model.set_unsorted();

        // we update the tree view
        update_window(&mut procs.list_store, &sys2, &mut m_display_tab.borrow_mut());

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            model.set_sort_column_id(col, order);
        }
        glib::Continue(true)
    });

    window.add(&note.notebook);
    window.show_all();
    gtk::main();
}
