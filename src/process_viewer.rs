#![crate_type = "bin"]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{WindowTrait, ContainerTrait, WidgetTrait, TreeSortableTrait, ButtonSignals, BoxTrait};
use gtk::{Orientation};
use sysinfo::*;
use gtk::signal::Inhibit;
use gtk::signal::{WidgetSignals, TreeViewSignals};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::{RefCell};

use std::cmp::{Ord, Ordering};
use std::str::FromStr;

struct NoteBook {
    notebook: gtk::NoteBook,
    tabs: Vec<gtk::Box>
}

impl NoteBook {
    fn new() -> NoteBook {
        NoteBook {
            notebook: gtk::NoteBook::new().unwrap(),
            tabs: Vec::new()
        }
    }

    fn create_tab<'a, Widget: gtk::WidgetTrait>(&mut self, title: &'a str, widget: &Widget) -> Option<u32> {
        let label = gtk::Label::new(title).unwrap();
        let tab = gtk::Box::new(Orientation::Horizontal, 0).unwrap();

        tab.pack_start(&label, false, false, 0);
        tab.show_all();

        let index = match self.notebook.append_page(widget, Some(&tab)) {
            Some(index) => index,
            _ => return None
        };

        self.tabs.push(tab);

        Some(index)
    }
}

struct Procs {
    left_tree: gtk::TreeView,
    scroll: gtk::ScrolledWindow,
    current_pid: Rc<RefCell<Option<i64>>>,
    kill_button: Rc<RefCell<gtk::Button>>,
    vertical_layout: gtk::Box,
    list_store: gtk::ListStore
}

impl Procs {
    pub fn new<'a>(proc_list: &HashMap<usize, Process>) -> Procs {
        let left_tree = gtk::TreeView::new().unwrap();
        let scroll = gtk::ScrolledWindow::new(None, None).unwrap();
        let current_pid = Rc::new(RefCell::new(None));
        let kill_button = Rc::new(RefCell::new(gtk::Button::new_with_label("End task").unwrap()));
        let current_pid1 = current_pid.clone();
        let current_pid2 = current_pid.clone();
        let kill_button1 = kill_button.clone();

        scroll.set_min_content_height(800);
        scroll.set_min_content_width(600);

        let mut columns : Vec<gtk::TreeViewColumn> = Vec::new();

        append_column("pid", &mut columns);
        append_column("process name", &mut columns);
        append_column("cpu usage", &mut columns);
        append_column("memory usage (in kB)", &mut columns);

        for i in columns {
            left_tree.append_column(&i);
        }

        let mut list_store = gtk::ListStore::new(&[glib::Type::ISize, glib::Type::String, glib::Type::String, glib::Type::USize]).unwrap();
        for (_, pro) in proc_list {
            create_and_fill_model(&mut list_store, pro.pid, &pro.cmd, &pro.name, pro.cpu_usage, pro.memory);
        }

        left_tree.set_model(&list_store.get_model().unwrap());
        left_tree.set_headers_visible(true);
        scroll.add(&left_tree);
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap();

        left_tree.connect_cursor_changed(move |tree_view| {
            match tree_view.get_selection() {
                Some(selection) => {
                    let model = tree_view.get_model().unwrap();
                    let mut iter = gtk::TreeIter::new();

                    if selection.get_selected(&model, &mut iter) {
                        let pid = Some(model.get_value(&iter, 0).get_long());
                        let mut tmp = current_pid1.borrow_mut();
                        *tmp = pid;
                    }
                    (*kill_button1.borrow_mut()).set_sensitive((*current_pid.borrow()).is_some());
                }
                None => {
                    let mut tmp = current_pid1.borrow_mut();
                    *tmp = None;
                }
            }
        });
        (*kill_button.borrow_mut()).set_sensitive(false);

        vertical_layout.add(&scroll);
        vertical_layout.add(&(*kill_button.borrow_mut()));
        let sort = left_tree.get_model().unwrap().get_tree_sortable();

        sort.set_sort_func(0, Box::new(|model, iter1, iter2| {
            println!("sort pids");
            (model.get_value(iter1, 0).get_long() - model.get_value(iter2, 0).get_long()) as i32
        }));
        sort.set_sort_func(1, Box::new(|model, iter1, iter2| {
            println!("sort names");
            match (model.get_value(iter1, 1).get_string(), model.get_value(iter2, 1).get_string()) {
                (Some(s1), Some(s2)) => {
                    match s1.cmp(&s2) {
                        Ordering::Less => -1,
                        Ordering::Greater => 1,
                        _ => 0
                    }
                }
                (_, _) => 0
            }
        }));
        sort.set_sort_func(2, Box::new(|model, iter1, iter2| {
            println!("sort %");
            match (model.get_value(iter1, 2).get_string(), model.get_value(iter2, 2).get_string()) {
                (Some(s1), Some(s2)) => {
                    let pourcent_1 = f64::from_str(&s1).unwrap();
                    let pourcent_2 = f64::from_str(&s2).unwrap();

                    if pourcent_1 > pourcent_2 {
                        1
                    } else if pourcent_1 < pourcent_2 {
                        -1
                    } else {
                        0
                    }
                }
                (_, _) => {
                    0
                }
            }
        }));
        sort.set_sort_func(3, Box::new(|model, iter1, iter2| {
            println!("sort memory");
            (model.get_value(iter1, 3).get_long() - model.get_value(iter2, 3).get_long()) as i32
        }));
        sort.set_sort_column_id(1, gtk::SortType::Ascending);
        Procs {
            left_tree: left_tree,
            scroll: scroll,
            current_pid: current_pid2.clone(),
            kill_button: kill_button,
            vertical_layout: vertical_layout,
            list_store: list_store
        }
    }
}

fn append_column(title: &str, v: &mut Vec<gtk::TreeViewColumn>) {
    let l = v.len();
    let renderer = gtk::CellRendererText::new().unwrap();

    v.push(gtk::TreeViewColumn::new().unwrap());
    let tmp = v.get_mut(l).unwrap();

    tmp.set_title(title);
    tmp.set_resizable(true);
    tmp.pack_start(&renderer, true);
    tmp.add_attribute(&renderer, "text", l as i32);
}

fn create_and_fill_model(list_store: &mut gtk::ListStore, pid: i64, cmdline: &str, name: &str, cpu: f32, memory: u64) {
    if cmdline.len() < 1 {
        return;
    }
    let mut top_level = gtk::TreeIter::new();

    let mut val1 = glib::Value::new();
    val1.init(glib::Type::ISize);
    val1.set_long(pid);
    let mut val2 = glib::Value::new();
    val2.init(glib::Type::USize);
    val2.set_ulong(memory);
    list_store.append(&mut top_level);
    list_store.set_value(&top_level, 0, &val1);
    list_store.set_string(&top_level, 1, name);
    list_store.set_string(&top_level, 2, &format!("{:.1}", cpu));
    list_store.set_value(&top_level, 3, &val2);
}

fn update_window(list: &mut gtk::ListStore, system: Rc<RefCell<sysinfo::System>>, info: Rc<RefCell<DisplaySysInfo>>) {
    let model = list.get_model().unwrap();
    let mut iter = gtk::TreeIter::new();

    (*system.borrow_mut()).refresh_all();
    let mut entries : HashMap<usize, Process> = ((*system.borrow()).get_process_list()).clone();
    let mut nb = model.iter_n_children(None);

    let sys = system.clone();
    (*info.borrow_mut()).update_ram_display(sys.clone());
    (*info.borrow_mut()).update_process_display(sys);

    let mut i = 0;
    while i < nb {
        if model.iter_nth_child(&mut iter, None, i) {
            let pid : i64 = model.get_value(&iter, 0).get_long();
            let mut to_delete = false;

            match entries.get(&(pid as usize)) {
                Some(p) => {
                    let mut val2 = glib::Value::new();
                    val2.init(glib::Type::USize);
                    val2.set_ulong(p.memory);
                    list.set_string(&iter, 2, &format!("{:.1}", p.cpu_usage));
                    list.set_value(&iter, 3, &val2);
                    to_delete = true;
                }
                None => {
                    list.remove(&iter);
                }
            }
            if to_delete {
                entries.remove(&(pid as usize));
                nb = model.iter_n_children(None);
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

struct DisplaySysInfo {
    procs : Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram : Rc<RefCell<gtk::ProgressBar>>,
    swap : Rc<RefCell<gtk::ProgressBar>>,
    vertical_layout : gtk::Box
}

impl DisplaySysInfo {
    pub fn new(sys1: Rc<RefCell<sysinfo::System>>) -> DisplaySysInfo {
        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(Vec::new())),
            ram: Rc::new(RefCell::new(gtk::ProgressBar::new().unwrap())),
            swap: Rc::new(RefCell::new(gtk::ProgressBar::new().unwrap())),
            vertical_layout: gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap()
        };

        let sys2 = sys1.clone();
        let v = tmp.procs.clone();

        (*tmp.ram.borrow_mut()).set_show_text(true);
        (*tmp.swap.borrow_mut()).set_show_text(true);

        tmp.vertical_layout.set_spacing(5);
        let mut i = 0;
        let mut total = false;

        tmp.vertical_layout.pack_start(&gtk::Label::new("Memory usage").unwrap(), false, false, 15);
        tmp.vertical_layout.add(&(*tmp.ram.borrow()));
        tmp.vertical_layout.pack_start(&gtk::Label::new("Swap usage").unwrap(), false, false, 15);
        tmp.vertical_layout.add(&(*tmp.swap.borrow()));
        tmp.vertical_layout.pack_start(&gtk::Label::new("Total CPU usage").unwrap(), false, false, 15);
        for pro in (*sys1.borrow()).get_processor_list() {
            if total {
                (*v.borrow_mut()).push(gtk::ProgressBar::new().unwrap());
                let p : &gtk::ProgressBar = &(*v.borrow())[i];
                let l = gtk::Label::new(&format!("{}", i)).unwrap();
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0).unwrap();

                p.set_text(&format!("{:.2} %", pro.get_cpu_usage() * 100.));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);
                horizontal_layout.pack_start(&l, false, false, 5);
                horizontal_layout.pack_start(p, true, true, 5);
                tmp.vertical_layout.add(&horizontal_layout);
            } else {
                (*v.borrow_mut()).push(gtk::ProgressBar::new().unwrap());
                let p : &gtk::ProgressBar = &(*v.borrow())[i];

                p.set_text(&format!("{:.2} %", pro.get_cpu_usage() * 100.));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);

                tmp.vertical_layout.add(p);
                tmp.vertical_layout.pack_start(&gtk::Label::new("Process usage").unwrap(), false, false, 15);
                total = true;
            }
            i += 1;
        }
        tmp.update_ram_display(sys2);
        tmp
    }

    pub fn update_ram_display(&mut self, sys: Rc<RefCell<sysinfo::System>>) {
        let total = (*sys.borrow()).get_total_memory();
        let used = (*sys.borrow()).get_used_memory();
        let disp = if total < 100000 {
            format!("{} / {}kB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        (*self.ram.borrow_mut()).set_text(&disp);
        (*self.ram.borrow_mut()).set_fraction(used as f64 / total as f64);

        let total = (*sys.borrow()).get_total_swap();
        let used = total - (*sys.borrow()).get_used_swap();
        let disp = if total < 100000 {
            format!("{} / {}kB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        (*self.swap.borrow_mut()).set_text(&disp);
        (*self.swap.borrow_mut()).set_fraction(used as f64 / total as f64);
    }

    pub fn update_process_display(&mut self, sys: Rc<RefCell<sysinfo::System>>) {
        let v = &*self.procs.borrow_mut();
        let mut i = 0;

        for pro in (*sys.borrow()).get_processor_list() {
            v[i].set_text(&format!("{:.1} %", pro.get_cpu_usage() * 100.));
            v[i].set_show_text(true);
            v[i].set_fraction(pro.get_cpu_usage() as f64);
            i += 1;
        }
    }
}

fn main() {
    gtk::init();

    let window = gtk::Window::new(gtk::WindowType::Toplevel).unwrap();
    let sys : Rc<RefCell<sysinfo::System>> = Rc::new(RefCell::new(sysinfo::System::new()));
    let mut note = NoteBook::new();
    let mut procs = Procs::new((*sys.borrow()).get_process_list());
    let current_pid2 = procs.current_pid.clone();
    let sys1 = sys.clone();
    let sys2 = sys.clone();

    window.set_title("Process viewer");
    window.set_window_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    (*sys.borrow_mut()).refresh_all();
    (*procs.kill_button.borrow_mut()).connect_clicked(move |_| {
        let tmp = (*current_pid2.borrow_mut()).is_some() ;

        if tmp {
            let s = (*current_pid2.borrow()).clone();
            match (*sys.borrow_mut()).get_process(s.unwrap()) {
                Some(p) => {
                    p.kill(Signal::Kill);
                },
                None => {}
            };
        }
    });

    let display_tab = Rc::new(RefCell::new(DisplaySysInfo::new(sys2)));

    //window.add(&vertical_layout);
    note.create_tab("Process list", &procs.vertical_layout);
    //let t = gtk::Button::new_with_label("test").unwrap();
    note.create_tab("System usage", &(*display_tab.borrow()).vertical_layout);

    glib::timeout_add(1500, move || {
        update_window(&mut procs.list_store, sys1.clone(), display_tab.clone());
        glib::Continue(true)
    });

    window.add(&note.notebook);
    window.show_all();
    gtk::main();
}
