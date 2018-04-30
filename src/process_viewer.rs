// 
// Process viewer
// 
// Copyright (c) 2017 Guillaume Gomez
//

#![crate_type = "bin"]

extern crate cairo;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;
extern crate libc;
extern crate pango;
extern crate sysinfo;

use sysinfo::*;

use gdk_pixbuf::Pixbuf;
use gio::{ActionExt, ActionMapExt, ApplicationExt, ApplicationExtManual, MemoryInputStream, MenuExt,
          SimpleActionExt};
use glib::{Bytes, IsA, ToVariant};
use gtk::{AboutDialog, Button, Dialog, EditableSignals, Entry, Inhibit, MessageDialog};
use gtk::{
    AboutDialogExt, BoxExt, ButtonExt, ContainerExt, DialogExt, EntryExt, GtkApplicationExt,
    ListStoreExt, ListStoreExtManual, ToggleButtonExt, TreeModelExt, TreeSortableExtManual,
    TreeViewExt, WidgetExt, GtkWindowExt, GtkWindowExtManual,
};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env::args;
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::rc::Rc;

use display_sysinfo::DisplaySysInfo;
use notebook::NoteBook;
use procs::{create_and_fill_model, Procs};

mod color;
mod display_sysinfo;
mod graph;
mod notebook;
mod process_dialog;
mod procs;
mod utils;

fn update_window(list: &gtk::ListStore, system: &Rc<RefCell<sysinfo::System>>,
                 info: &mut DisplaySysInfo, display_fahrenheit: bool) {
    let mut system = system.borrow_mut();
    system.refresh_all();
    info.update_ram_display(&system, display_fahrenheit);
    info.update_process_display(&system);
    let entries: &HashMap<Pid, Process> = system.get_process_list();
    let mut seen: HashSet<Pid> = HashSet::new();

    if let Some(iter) = list.get_iter_first() {
        let mut valid = true;
        while valid {
            if let Some(pid) = list.get_value(&iter, 0).get::<u32>().map(|x| x as Pid) {
                if let Some(p) = entries.get(&(pid)) {
                    list.set(&iter,
                             &[2, 3, 5],
                             &[&format!("{:.1}", p.cpu_usage()), &p.memory(), &p.cpu_usage()]);
                    valid = list.iter_next(&iter);
                    seen.insert(pid);
                } else {
                    valid = list.remove(&iter);
                }
            }
        }
    }

    for (pid, pro) in entries.iter() {
        if !seen.contains(pid) {
            create_and_fill_model(list, pro.pid().as_u32(), &format!("{:?}", &pro.cmd()), &pro.name(),
                                  pro.cpu_usage(), pro.memory());
        }
    }
}

fn parse_quote(line: &str, quote: char) -> Vec<String> {
    let args = line.split(quote).collect::<Vec<&str>>();
    let mut out_args = vec!();

    for (num, arg) in args.iter().enumerate() {
        if num != 1 {
            out_args.extend_from_slice(&parse_entry(arg));
        } else {
            out_args.push((*arg).to_owned());
        }
    }
    out_args
}

// super simple parsing
fn parse_entry(line: &str) -> Vec<String> {
    match (line.find('\''), line.find('"')) {
        (Some(x), Some(y)) => {
            if x < y {
                parse_quote(line, '\'')
            } else {
                parse_quote(line, '"')
            }
        }
        (Some(_), None) => parse_quote(line, '\''),
        (None, Some(_)) => parse_quote(line, '"'),
        (None, None) => line.split(' ').into_iter()
                                       .map(|s| s.to_owned())
                                       .collect::<Vec<String>>(),
    }
}

#[cfg(unix)]
fn build_command(c: &mut Command) -> &mut Command {
    c.before_exec(|| {
        unsafe { libc::setsid() };
        Ok(())
    })
}

#[cfg(windows)]
fn build_command(c: &mut Command) -> &mut Command {
    c
}

fn start_detached_process(line: &str) -> Option<String> {
    let args = parse_entry(line);
    let command = args[0].clone();

    let cmd = build_command(Command::new(&command).args(&args)).stdin(Stdio::null())
                                                               .stderr(Stdio::null())
                                                               .stdout(Stdio::null())
                                                               .spawn();
    if cmd.is_err() {
        Some(format!("Failed to start '{}'", &command))
    } else {
        None
    }
}

fn run_command<T: IsA<gtk::Window>>(input: &Entry, window: &T, d: &Dialog) {
    if let Some(text) = input.get_text() {
        let x = if let Some(x) = start_detached_process(&text) {
            x
        } else {
            "The command started successfully".to_owned()
        };
        d.destroy();
        let m = MessageDialog::new(Some(window),
                                   gtk::DialogFlags::DESTROY_WITH_PARENT,
                                   gtk::MessageType::Info,
                                   gtk::ButtonsType::Ok,
                                   &x);
        m.run();
        m.destroy();
    }
}

fn build_ui(application: &gtk::Application) {
    let menu = gio::Menu::new();
    let menu_bar = gio::Menu::new();
    let more_menu = gio::Menu::new();
    let settings_menu = gio::Menu::new();

    menu.append("Launch new executable", "app.new-task");
    menu.append("Quit", "app.quit");

    settings_menu.append("Display temperature in °F", "app.temperature");
    settings_menu.append("Display graphs", "app.graphs");
    menu_bar.append_submenu("_Setting", &settings_menu);

    more_menu.append("About", "app.about");
    menu_bar.append_submenu("?", &more_menu);

    application.set_app_menu(&menu);
    application.set_menubar(&menu_bar);

    let window = gtk::ApplicationWindow::new(application);
    let sys = sysinfo::System::new();
    let start_time = unsafe { if let Some(p) = sys.get_process(libc::getpid() as Pid) {
        p.start_time()
    } else {
        0
    }};
    let running_since = Rc::new(RefCell::new(0));
    let sys = Rc::new(RefCell::new(sys));
    let mut note = NoteBook::new();
    let procs = Procs::new(sys.borrow().get_process_list(), &mut note);
    let current_pid = Rc::clone(&procs.current_pid);
    let current_pid2 = Rc::clone(&procs.current_pid);
    let sys1 = Rc::clone(&sys);
    let sys2 = Rc::clone(&sys);
    let sys3 = Rc::clone(&sys);
    let info_button = procs.info_button.clone();

    window.set_title("Process viewer");
    window.set_position(gtk::WindowPosition::Center);
    // To silence the annying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    window.get_preferred_width();
    window.set_default_size(500, 700);

    window.connect_delete_event(|w, _| {
        w.destroy();
        Inhibit(false)
    });

    sys.borrow_mut().refresh_all();
    procs.kill_button.connect_clicked(move |_| {
        let sys = sys1.borrow();
        if let Some(process) = current_pid.get().and_then(|pid| sys.get_process(pid)) {
            process.kill(Signal::Kill);
        }
    });

    let mut display_tab = DisplaySysInfo::new(&sys, &mut note, &window);

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let ram_check_box = display_tab.ram_check_box.clone();
    let swap_check_box = display_tab.swap_check_box.clone();
    let network_check_box = display_tab.network_check_box.clone();
    let temperature_check_box = display_tab.temperature_check_box.clone();

    let graphs = gio::SimpleAction::new_stateful("graphs", None, &false.to_variant());
    graphs.connect_activate(move |g, _| {
        let mut is_active = false;
        if let Some(g) = g.get_state() {
            is_active = g.get().expect("couldn't get bool");
            ram_check_box.set_active(!is_active);
            swap_check_box.set_active(!is_active);
            network_check_box.set_active(!is_active);
            if let Some(ref temperature_check_box) = temperature_check_box {
                temperature_check_box.set_active(!is_active);
            }
        }
        // We need to change the toggle state ourselves. `gio` dark magic.
        g.change_state(&(!is_active).to_variant());
    });
    let temperature = gio::SimpleAction::new_stateful("temperature", None, &false.to_variant());
    temperature.connect_activate(move |g, _| {
        let mut is_active = false;
        if let Some(g) = g.get_state() {
            is_active = g.get().expect("couldn't get graph state");
        }
        // We need to change the toggle state ourselves. `gio` dark magic.
        g.change_state(&(!is_active).to_variant());
    });
    application.add_action(&temperature);

    // I think it's now useless to have this one...
    v_box.pack_start(&note.notebook, true, true, 0);

    window.add(&v_box);
    window.show_all();
    window.activate();

    let list_store = procs.list_store.clone();
    let run1 = Rc::clone(&running_since);
    let run2 = Rc::clone(&running_since);
    gtk::timeout_add(1000, move || {
        *run1.borrow_mut() += 1;
        // first part, deactivate sorting
        let sorted = TreeSortableExtManual::get_sort_column_id(&list_store);
        list_store.set_unsorted();

        // we update the tree view
        if let Some(temperature) = temperature.get_state() {
            update_window(&list_store, &sys, &mut display_tab,
                          temperature.get::<bool>().expect("couldn't get temperature state"));
        }

        // we re-enable the sorting
        if let Some((col, order)) = sorted {
            list_store.set_sort_column_id(col, order);
        }
        glib::Continue(true)
    });
    let window1 = window.clone();
    info_button.connect_clicked(move |_| {
        let sys = sys2.borrow();
        if let Some(process) = current_pid2.get().and_then(|pid| sys.get_process(pid)) {
            process_dialog::create_process_dialog(process, &window1, start_time,
                                                  *run2.borrow());
        }
    });
    let window2 = window.clone();
    procs.left_tree.connect_row_activated(move |tree_view, path, _| {
        let model = tree_view.get_model().expect("couldn't get model");
        let iter = model.get_iter(path).expect("couldn't get iter");
        let sys = sys3.borrow();
        if let Some(process) = sys.get_process(model.get_value(&iter, 0)
                                                    .get::<u32>()
                                                    .map(|x| x as Pid)
                                                    .expect("failed to get value from model")) {
            process_dialog::create_process_dialog(process, &window2, start_time,
                                                  *running_since.borrow());
        }
    });

    let window2 = window.clone();
    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate(move |_, _| {
        window2.destroy();
    });
    let window3 = window.clone();
    window3.present();
    let about = gio::SimpleAction::new("about", None);
    about.connect_activate(move |_, _| {
        let p = AboutDialog::new();
        p.set_authors(&["Guillaume Gomez"]);
        p.set_website_label(Some("my website"));
        p.set_website(Some("https://guillaume-gomez.fr/"));
        p.set_comments(Some("A process viewer GUI wrote with gtk-rs"));
        p.set_copyright(Some("This is under MIT license"));
        p.set_transient_for(Some(&window3));
        p.set_program_name("process-viewer");
        let memory_stream = MemoryInputStream::new_from_bytes(
                                &Bytes::from_static(include_bytes!("../assets/eye.png")));
        let logo = Pixbuf::new_from_stream(&memory_stream, None);
        if let Ok(logo) = logo {
            p.set_logo(Some(&logo));
        }
        p.run();
        p.destroy();
    });
    let new_task = gio::SimpleAction::new("new-task", None);
    new_task.connect_activate(move |_, _| {
        let d = Dialog::new();
        d.set_title("Launch new executable");
        let content_area = d.get_content_area();
        let input = Entry::new();
        let run = Button::new_with_label("Run");
        let cancel = Button::new_with_label("Cancel");
        let v_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let h_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        h_box.pack_start(&run, true, true, 0);
        h_box.pack_start(&cancel, true, true, 0);
        v_box.pack_start(&input, true, true, 0);
        v_box.pack_start(&h_box, true, true, 0);
        content_area.add(&v_box);
        let window2 = window.clone();
        d.set_transient_for(Some(&window2));
        // To silence the annying warning:
        // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
        // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
        // allocate?"
        d.get_preferred_width();
        d.set_size_request(400, 70);
        d.show_all();

        run.set_sensitive(false);
        let run2 = run.clone();
        let input2 = input.clone();
        input.connect_changed(move |_| {
            match input2.get_text() {
                Some(ref x) if !x.is_empty() => run2.set_sensitive(true),
                _ => run2.set_sensitive(false),
            }
        });
        let d2 = d.clone();
        cancel.connect_clicked(move |_| {
            d2.destroy();
        });
        let window3 = window.clone();
        let input3 = input.clone();
        let d3 = d.clone();
        input.connect_activate(move |_| {
            run_command(&input3, &window3, &d3);
        });
        let window4 = window.clone();
        run.connect_clicked(move |_| {
            run_command(&input, &window4, &d);
        });
    });

    application.add_action(&about);
    application.add_action(&graphs);
    application.add_action(&new_task);
    application.add_action(&quit);
    application.connect_activate(|_| {});
}

fn main() {
    let application = gtk::Application::new("com.github.GuillaumeGomez.process-viewer",
                                            gio::ApplicationFlags::empty())
                                       .expect("Initialization failed...");

    application.connect_startup(move |app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}
