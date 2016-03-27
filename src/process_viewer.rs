#![crate_type = "bin"]

extern crate cairo;
extern crate gtk;
extern crate glib;
extern crate sysinfo;
extern crate gdk;

use gtk::prelude::*;

use sysinfo::*;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use display_sysinfo::DisplaySysInfo;
use notebook::NoteBook;
use procs::{create_and_fill_model, Procs};

mod color;
mod display_sysinfo;
mod notebook;
mod procs;
mod utils;

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

    let mut display_tab = DisplaySysInfo::new(sys.clone(), &mut note, &window);

    window.add(&note.notebook);
    window.show_all();

    gtk::timeout_add(1000, move || {
        // first part, deactivate sorting
        let sorted = procs.list_store.get_sort_column_id();
        procs.list_store.set_unsorted();

        // we update the tree view
        update_window(&procs.list_store, &sys, &mut display_tab);

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            procs.list_store.set_sort_column_id(col, order);
        }
        window.queue_draw();
        glib::Continue(true)
    });

    gtk::main();
}
