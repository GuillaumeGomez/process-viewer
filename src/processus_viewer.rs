#![crate_type = "bin"]
#![feature(collections)]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{WindowTrait, ContainerTrait, WidgetTrait, ButtonSignals, BoxTrait};
use gtk::{IconSize, Orientation, ReliefStyle};
use glib::{Type, timeout};
use sysinfo::*;
use gtk::signal::Inhibit;
use gtk::signal::{WidgetSignals, TreeViewSignals};
use std::collections::VecMap;
use std::str::FromStr;
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::ops::DerefMut;

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
    current_pid: Rc<RefCell<Option<String>>>,
    kill_button: Rc<RefCell<gtk::Button>>,
    vertical_layout: gtk::Box,
    list_store: gtk::ListStore
}

impl Procs {
    pub fn new<'a>(proc_list: &VecMap<Processus>) -> Procs {
        let mut left_tree = gtk::TreeView::new().unwrap();
        let mut scroll = gtk::ScrolledWindow::new(None, None).unwrap();
        let mut current_pid = Rc::new(RefCell::new(None));
        let mut kill_button = Rc::new(RefCell::new(gtk::Button::new_with_label("End task").unwrap()));
        let mut current_pid1 = current_pid.clone();
        let mut current_pid2 = current_pid.clone();
        let mut kill_button1 = kill_button.clone();

        scroll.set_min_content_height(800);
        scroll.set_min_content_width(600);

        let mut columns : Vec<gtk::TreeViewColumn> = Vec::new();

        append_column("pid", &mut columns);
        append_column("process name", &mut columns);
        append_column("cpu usage", &mut columns);
        append_column("memory usage", &mut columns);

        for i in columns {
            left_tree.append_column(&i);
        }

        let mut list_store = gtk::ListStore::new(&[Type::String, Type::String, Type::String, Type::String]).unwrap();
        for (_, pro) in proc_list {
            create_and_fill_model(&mut list_store, pro.pid, &pro.name, pro.cpu_usage, pro.memory);
        }

        left_tree.set_model(&list_store.get_model().unwrap());
        left_tree.set_headers_visible(true);
        scroll.add(&left_tree);
        let mut vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap();

        left_tree.connect_cursor_changed(move |tree_view| {
            match tree_view.get_selection() {
                Some(selection) => {
                    let model = tree_view.get_model().unwrap();
                    let mut iter = gtk::TreeIter::new();

                    if selection.get_selected(&model, &mut iter) {
                        let pid = model.get_value(&iter, 0).get_string();
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
    tmp.pack_start(&renderer, true);
    tmp.add_attribute(&renderer, "text", l as i32);
}

fn create_and_fill_model(list_store: &mut gtk::ListStore, pid: i64, name: &str, cpu: f32, memory: u64) {
    if name.len() < 1 {
        return;
    }
    let mut top_level = gtk::TreeIter::new();

    list_store.append(&mut top_level);
    list_store.set_string(&top_level, 0, &format!("{}", pid));
    list_store.set_string(&top_level, 1, name);
    list_store.set_string(&top_level, 2, &format!("{}", cpu));
    list_store.set_string(&top_level, 3, &format!("{}", memory));
}

fn update_window(w: &mut (&mut gtk::ListStore, Rc<RefCell<sysinfo::System>>, Rc<RefCell<Vec<gtk::ProgressBar>>>)) -> i32 {
    let mut model = w.0.get_model().unwrap();
    let mut iter = gtk::TreeIter::new();

    (*w.1.borrow_mut()).refresh();
    let mut entries : VecMap<Processus> = ((*w.1.borrow()).get_processus_list()).clone();
    let mut nb = model.iter_n_children(None);
    let mut i = 0;

    let v = &*w.2.borrow_mut();
    for pro in (*w.1.borrow()).get_process_list() {
        v[i].set_text(&format!("{:.2} %", pro.get_cpu_usage() * 100.));
        v[i].set_show_text(true);
        v[i].set_fraction(pro.get_cpu_usage() as f64);
        i += 1;
    }

    let mut i = 0;
    while i < nb {
        if model.iter_nth_child(&mut iter, None, i) {
            let pid : i64 = i64::from_str(&model.get_value(&iter, 0).get_string().unwrap()).unwrap();
            let mut to_delete = false;

            match entries.get(&(pid as usize)) {
                Some(p) => {
                    //w.0.set_string(&iter, 1, &p.name);
                    w.0.set_string(&iter, 2, &format!("{}", p.cpu_usage));
                    w.0.set_string(&iter, 3, &format!("{}", p.memory));
                    to_delete = true;
                }
                None => {
                    w.0.remove(&iter);
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
        create_and_fill_model(&mut w.0, pro.pid, &pro.name, pro.cpu_usage, pro.memory);
    }
    1
}

fn main() {
    gtk::init();

    let mut window = gtk::Window::new(gtk::WindowType::TopLevel).unwrap();
    let mut sys : Rc<RefCell<sysinfo::System>> = Rc::new(RefCell::new(sysinfo::System::new()));
    let mut sys1 = sys.clone();
    let mut note = NoteBook::new();
    (*sys.borrow_mut()).refresh();
    let mut procs = Procs::new((*sys.borrow()).get_processus_list());
    let mut current_pid2 = procs.current_pid.clone();
    let mut sys1 = sys.clone();

    window.set_title("Processus viewer");
    window.set_window_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    (*procs.kill_button.borrow_mut()).connect_clicked(move |_| {
        let tmp = (*current_pid2.borrow_mut()).is_some() ;

        if tmp {
            let s = (*current_pid2.borrow()).clone();
            match (*sys.borrow_mut()).get_processus(i64::from_str(&s.unwrap()).unwrap()) {
                Some(p) => {
                    p.kill(Signal::Kill);
                },
                None => {}
            };
        }
    });

    let mut vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap();
    //let mut i = 0;

    let mut v = Vec::new();
    for pro in (*sys1.borrow()).get_process_list() {
        v.push(gtk::ProgressBar::new().unwrap());
        let p : &gtk::ProgressBar = &v[v.len() - 1];

        p.set_text(&format!("{:.2} %", pro.get_cpu_usage() * 100.));
        p.set_show_text(true);
        p.set_fraction(pro.get_cpu_usage() as f64);
        vertical_layout.add(p);
    }
    let mut v_procs = Rc::new(RefCell::new(v));

    timeout::add(1500, update_window, &mut (&mut procs.list_store, sys1.clone(), v_procs.clone()));
    //window.add(&vertical_layout);
    note.create_tab("Processus list", &procs.vertical_layout);
    //let t = gtk::Button::new_with_label("test").unwrap();
    note.create_tab("System usage", &vertical_layout);
    window.add(&note.notebook);
    window.show_all();
    gtk::main();
}
