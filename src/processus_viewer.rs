#![crate_type = "bin"]
#![feature(collections)]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{WindowTrait, ContainerTrait, WidgetTrait, ButtonSignals};
use glib::{Type, timeout};
use sysinfo::*;
use gtk::signal::Inhibit;
use gtk::signal::{WidgetSignals, TreeViewSignals};
use std::collections::VecMap;
use std::str::FromStr;
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::ops::DerefMut;

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

fn update_window(w: &mut (&mut gtk::ListStore, Rc<RefCell<sysinfo::System>>)) -> i32 {
    let mut model = w.0.get_model().unwrap();
    let mut iter = gtk::TreeIter::new();

    (*w.1.borrow_mut()).refresh();
    let mut entries : VecMap<Processus> = ((*w.1.borrow()).get_processus_list()).clone();
    let mut nb = model.iter_n_children(None);
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

    window.set_title("Processus viewer");
    window.set_window_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    let mut left_tree = gtk::TreeView::new().unwrap();
    let mut scroll = gtk::ScrolledWindow::new(None, None).unwrap();
    let mut current_pid : Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let mut current_pid1 = current_pid.clone();
    let mut current_pid2 = current_pid.clone();
    let mut kill_button : Rc<RefCell<gtk::Button>> = Rc::new(RefCell::new(gtk::Button::new_with_label("End task").unwrap()));
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
    (*sys.borrow_mut()).refresh();
    for (_, pro) in (*sys.borrow()).get_processus_list() {
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
        /*let mut path = gtk::TreePath::new().unwrap();
        println!("{:?}", path.unwrap_pointer());
        tree_view.get_cursor(Some(&mut path), None);
        println!("{:?}", path.unwrap_pointer());
        let model = tree_view.get_model().unwrap();

        if model.get_iter(&mut iter, &path) {
            let mut tmp = current_pid1.borrow_mut();
            *tmp = Some(model.get_value(&iter, 0).get_string().unwrap());
        }
        println!("current pid : {:?}", current_pid);*/
    });
    (*kill_button.borrow_mut()).set_sensitive(false);
    (*kill_button.borrow_mut()).connect_clicked(move |_| {
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
    timeout::add(1500, update_window, &mut (&mut list_store, sys1.clone()));

    vertical_layout.add(&scroll);
    vertical_layout.add(&(*kill_button.borrow_mut()));
    window.add(&vertical_layout);
    window.show_all();
    gtk::main();
}
