#![crate_type = "bin"]

extern crate cairo;
extern crate pango_sys;
extern crate gdk;
extern crate glib;
extern crate gtk;
extern crate gtk_sys;
extern crate libc;
extern crate sysinfo;

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
mod process_dialog;
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
    let sys = sysinfo::System::new();
    let start_time = unsafe { sys.get_process(libc::getpid() as i64).unwrap().start_time };
    let running_since = Rc::new(RefCell::new(0));
    let sys = Rc::new(RefCell::new(sys));
    let mut note = NoteBook::new();
    let procs = Procs::new(sys.borrow().get_process_list(), &mut note);
    let current_pid = procs.current_pid.clone();
    let current_pid2 = procs.current_pid.clone();
    let sys1 = sys.clone();
    let sys2 = sys.clone();
    let sys3 = sys.clone();
    let info_button = procs.info_button.clone();

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
    let window1 = window.clone();
    let window2 = window.clone();

    let list_store = procs.list_store.clone();
    let run1 = running_since.clone();
    let run2 = running_since.clone();
    gtk::timeout_add(1000, move || {
        *run1.borrow_mut() += 1;
        // first part, deactivate sorting
        let sorted = list_store.get_sort_column_id();
        list_store.set_unsorted();

        // we update the tree view
        update_window(&list_store, &sys, &mut display_tab);

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            list_store.set_sort_column_id(col, order);
        }
        window1.queue_draw();
        glib::Continue(true)
    });
    info_button.connect_clicked(move |_| {
        let sys = sys2.borrow();
        if let Some(process) = current_pid2.get().and_then(|pid| sys.get_process(pid)) {
            process_dialog::create_process_dialog(&process, &window2, start_time,
                                                  run2.borrow().clone());
        }
    });
    procs.left_tree.connect_row_activated(move |tree_view, path, _| {
        let model = tree_view.get_model().unwrap();
        let iter = model.get_iter(path).unwrap();
        let sys = sys3.borrow();
        if let Some(process) = sys.get_process(model.get_value(&iter, 0).get().unwrap()) {
            process_dialog::create_process_dialog(&process, &window, start_time,
                                                  running_since.borrow().clone());
        }
    });

    gtk::main();
}
