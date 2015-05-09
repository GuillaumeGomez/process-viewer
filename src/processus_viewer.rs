#![crate_type = "bin"]
#![feature(collections)]

extern crate gtk;
extern crate glib;
extern crate sysinfo;

use gtk::{WindowTrait, ContainerTrait, WidgetTrait};
use glib::{Type, timeout};
use sysinfo::*;
use gtk::signal::Inhibit;
use gtk::signal::WidgetSignals;
use std::collections::VecMap;
use std::str::FromStr;

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

fn update_window(w: &mut (&mut gtk::ListStore, &mut sysinfo::System)) -> i32 {
    let mut model = w.0.get_model().unwrap();
    let mut iter = gtk::TreeIter::new();

    w.1.refresh();
    let mut entries : VecMap<Processus> = (*w.1.get_processus_list()).clone();
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
    let mut sys = sysinfo::System::new();

    window.set_title("TreeView Sample");
    window.set_window_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });

    let mut left_tree = gtk::TreeView::new().unwrap();
    let mut scroll = gtk::ScrolledWindow::new(None, None).unwrap();

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
    sys.refresh();
    for (_, pro) in sys.get_processus_list() {
        create_and_fill_model(&mut list_store, pro.pid, &pro.name, pro.cpu_usage, pro.memory);
    }

    left_tree.set_model(&list_store.get_model().unwrap());
    left_tree.set_headers_visible(true);
    scroll.add(&left_tree);

    /*for _ in 0..10 {
        let mut iter = gtk::TreeIter::new().unwrap();
        left_store.append(&mut iter);
        left_store.set_string(&iter, 0, "I'm in a list");
    }*/

    // display the panes

    //let mut split_pane = gtk::Box::new(gtk::Orientation::Horizontal, 10).unwrap();

    //split_pane.set_size_request(-1, -1);
    //split_pane.add(&left_tree);

    timeout::add(1500, update_window, &mut (&mut list_store, &mut sys));

    window.add(&scroll);
    window.show_all();
    gtk::main();
}
