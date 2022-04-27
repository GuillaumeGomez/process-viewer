//
// Process viewer
//
// Copyright (c) 2017 Guillaume Gomez
//

#![crate_type = "bin"]

use sysinfo::*;

use gtk::gio::prelude::*;
use gtk::gio::MemoryInputStream;
use gtk::glib::{Bytes, Cast, IsA, ToVariant};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use gtk::gdk::Texture;
use gtk::{AboutDialog, Dialog, Entry, EventControllerKey, Inhibit, MessageDialog};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod color;
mod display_disk;
#[macro_use]
mod display_sysinfo;
mod display_network;
mod display_procs;
mod graph;
mod network_dialog;
mod notebook;
mod process_dialog;
mod settings;
mod utils;

use display_network::Network;
use display_procs::{create_and_fill_model, Procs};
use display_sysinfo::DisplaySysInfo;
use notebook::NoteBook;
use settings::Settings;
use utils::format_number;

pub const APPLICATION_NAME: &str = "fr.guillaume_gomez.ProcessViewer";

fn update_window(list: &gtk::ListStore, entries: &HashMap<Pid, sysinfo::Process>) {
    let mut seen: HashSet<Pid> = HashSet::new();

    if let Some(iter) = list.iter_first() {
        let mut valid = true;
        while valid {
            let pid = match list.get_value(&iter, 0).get::<u32>() {
                Ok(pid) => Pid::from_u32(pid),
                _ => {
                    valid = list.iter_next(&iter);
                    continue;
                }
            };
            if let Some(p) = entries.get(&(pid)) {
                let disk_usage = p.disk_usage();
                let disk_usage = disk_usage.written_bytes + disk_usage.read_bytes;
                let memory = p.memory() * 1_000;
                list.set(
                    &iter,
                    &[
                        (2, &format!("{:.1}", p.cpu_usage())),
                        (3, &format_number(memory)),
                        (
                            4,
                            &if disk_usage > 0 {
                                format_number(disk_usage)
                            } else {
                                String::new()
                            },
                        ),
                        (6, &p.cpu_usage()),
                        (7, &memory),
                        (8, &disk_usage),
                    ],
                );
                valid = list.iter_next(&iter);
                seen.insert(pid);
            } else {
                valid = list.remove(&iter);
            }
        }
    }

    for (pid, pro) in entries.iter() {
        if !seen.contains(pid) {
            create_and_fill_model(
                list,
                pid.as_u32(),
                pro.cmd(),
                pro.name(),
                pro.cpu_usage(),
                pro.memory() * 1_000,
            );
        }
    }
}

fn parse_quote(line: &str, quote: char) -> Vec<String> {
    let args = line.split(quote).collect::<Vec<&str>>();
    let mut out_args = vec![];

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
        (None, None) => line
            .split(' ')
            .map(::std::borrow::ToOwned::to_owned)
            .collect::<Vec<String>>(),
    }
}

#[cfg(unix)]
fn build_command(c: &mut Command) -> &mut Command {
    unsafe {
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        })
    }
}

#[cfg(windows)]
fn build_command(c: &mut Command) -> &mut Command {
    c
}

fn start_detached_process(line: &str) -> Option<String> {
    let args = parse_entry(line);
    let command = args[0].clone();

    let cmd = build_command(Command::new(&command).args(&args))
        .stdin(Stdio::null())
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
    let text = input.text();
    let x = if let Some(x) = start_detached_process(&text) {
        x
    } else {
        "The command started successfully".to_owned()
    };
    d.close();
    let m = MessageDialog::new(
        Some(window),
        gtk::DialogFlags::DESTROY_WITH_PARENT,
        gtk::MessageType::Info,
        gtk::ButtonsType::Ok,
        &x,
    );
    m.set_modal(true);
    m.connect_response(|dialog, response| {
        if response == gtk::ResponseType::DeleteEvent
            || response == gtk::ResponseType::Close
            || response == gtk::ResponseType::Ok
        {
            dialog.close();
        }
    });
    m.show();
}

fn create_new_proc_diag(
    process_dialogs: &Rc<RefCell<Vec<process_dialog::ProcDialog>>>,
    pid: Pid,
    sys: &sysinfo::System,
) {
    if let Some(proc_diag) = process_dialogs
        .borrow()
        .iter()
        .filter(|x| !x.is_dead)
        .find(|x| x.pid == pid)
    {
        proc_diag.popup.present();
        return;
    }
    let total_memory = sys.total_memory();
    if let Some(process) = sys.process(pid) {
        process_dialogs
            .borrow_mut()
            .push(process_dialog::create_process_dialog(process, total_memory));
    }
}

pub struct RequiredForSettings {
    process_refresh_timeout: Arc<Mutex<u32>>,
    network_refresh_timeout: Arc<Mutex<u32>>,
    system_refresh_timeout: Arc<Mutex<u32>>,
    sys: Arc<Mutex<sysinfo::System>>,
    process_dialogs: Rc<RefCell<Vec<process_dialog::ProcDialog>>>,
    list_store: gtk::ListStore,
    display_tab: Rc<RefCell<DisplaySysInfo>>,
    network_tab: Rc<RefCell<Network>>,
}

fn setup_timeout(rfs: &Rc<RefCell<RequiredForSettings>>) {
    let (ready_tx, ready_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let rfs = rfs.borrow();

    let sys = &rfs.sys;
    let process_dialogs = &rfs.process_dialogs;
    let list_store = &rfs.list_store;
    let process_refresh_timeout = &rfs.process_refresh_timeout;

    thread::spawn(
        glib::clone!(@weak sys, @strong ready_tx, @weak process_refresh_timeout => move || {
            loop {
                let sleep_dur = Duration::from_millis(
                    *process_refresh_timeout.lock().expect("failed to lock process refresh mutex") as _);
                thread::sleep(sleep_dur);
                sys.lock().expect("failed to lock to refresh processes").refresh_processes();
                ready_tx.send(false).expect("failed to send data through process refresh channel");
            }
        }),
    );

    ready_rx.attach(None,
        glib::clone!(@weak sys, @weak list_store, @weak process_dialogs => @default-return glib::Continue(true), move |_: bool| {
        // first part, deactivate sorting
        let sorted = TreeSortableExtManual::sort_column_id(&list_store);
        list_store.set_unsorted();

        let mut to_remove = 0;
        let mut dialogs = process_dialogs.borrow_mut();

        if let Ok(sys) = sys.lock() {
            // we update the tree view
            update_window(&list_store, sys.processes());

            // we re-enable the sorting
            if let Some((col, order)) = sorted {
                list_store.set_sort_column_id(col, order);
            }
            for dialog in dialogs.iter_mut().filter(|x| !x.is_dead) {
                // TODO: check if the process name matches the PID too!
                if let Some(process) = sys.processes().get(&dialog.pid) {
                    dialog.update(process);
                } else {
                    dialog.set_dead();
                }
                if dialog.need_remove() {
                    to_remove += 1;
                }
            }
        } else {
            panic!("failed to lock sys to refresh UI");
        }
        if to_remove > 0 {
            dialogs.retain(|x| !x.need_remove());
        }
        glib::Continue(true)
    }));
}

fn setup_network_timeout(rfs: &Rc<RefCell<RequiredForSettings>>) {
    let (ready_tx, ready_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let rfs = rfs.borrow();

    let network_refresh_timeout = &rfs.network_refresh_timeout;
    let network_tab = &rfs.network_tab;
    let sys = &rfs.sys;

    thread::spawn(
        glib::clone!(@weak sys, @strong ready_tx, @weak network_refresh_timeout => move || {
            loop {
                let sleep_dur = Duration::from_millis(
                    *network_refresh_timeout.lock().expect("failed to lock networks refresh mutex") as _);
                thread::sleep(sleep_dur);
                sys.lock().expect("failed to lock to refresh networks").refresh_networks();
                ready_tx.send(false).expect("failed to send data through networks refresh channel");
            }
        }),
    );

    ready_rx.attach(None,
        glib::clone!(@weak sys, @weak network_tab => @default-panic, move |_: bool| {
            network_tab.borrow_mut().update_networks(&*sys.lock().expect("failed to lock to update networks"));
            glib::Continue(true)
        })
    );
}

fn setup_system_timeout(rfs: &Rc<RefCell<RequiredForSettings>>, settings: &Rc<RefCell<Settings>>) {
    let (ready_tx, ready_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let rfs = rfs.borrow();

    let system_refresh_timeout = &rfs.system_refresh_timeout;
    let sys = &rfs.sys;
    let display_tab = &rfs.display_tab;

    thread::spawn(
        glib::clone!(@weak sys, @strong ready_tx, @weak system_refresh_timeout => move || {
            loop {
                let sleep_dur = Duration::from_millis(
                    *system_refresh_timeout.lock().expect("failed to lock system refresh mutex") as _);
                thread::sleep(sleep_dur);
                sys.lock().expect("failed to lock to refresh system").refresh_system();
                ready_tx.send(false).expect("failed to send data through system refresh channel");
            }
        }),
    );

    ready_rx.attach(
        None,
        glib::clone!(@weak sys, @weak display_tab, @weak settings => @default-return glib::Continue(true), move |_: bool| {
            let mut info = display_tab.borrow_mut();
            let sys = sys.lock().expect("failed to lock to update system");
            let display_fahrenheit = settings.borrow().display_fahrenheit;

            info.update_system_info(&*sys, display_fahrenheit);
            info.update_system_info_display(&*sys);
            glib::Continue(true)
        }),
    );
}

fn build_ui(application: &gtk::Application) {
    let settings = Settings::load();

    let menu = gio::Menu::new();
    let menu_bar = gio::Menu::new();
    let more_menu = gio::Menu::new();
    let settings_menu = gio::Menu::new();

    menu.append(Some("Launch new executable"), Some("app.new-task"));
    menu.append(Some("Quit"), Some("app.quit"));
    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate(glib::clone!(@weak application => move |_,_| {
        application.quit();
    }));
    application.set_accels_for_action("app.quit", &["<Primary>Q"]);

    settings_menu.append(Some("Display temperature in °F"), Some("app.temperature"));
    settings_menu.append(Some("Display graphs"), Some("app.graphs"));
    settings_menu.append(Some("More settings..."), Some("app.settings"));
    menu_bar.append_submenu(Some("_Settings"), &settings_menu);

    more_menu.append(Some("About"), Some("app.about"));
    menu_bar.append_submenu(Some("?"), &more_menu);

    // application.set_menu_bar(Some(&menu));
    application.set_menubar(Some(&menu_bar));

    let window = gtk::ApplicationWindow::new(application);

    let mut sys =
        sysinfo::System::new_with_specifics(RefreshKind::everything().without_users_list());
    let mut note = NoteBook::new();
    let procs = Procs::new(sys.processes(), &mut note, &window);
    let current_pid = Rc::clone(&procs.current_pid);
    let info_button = procs.info_button.clone();

    window.set_title(Some("Process viewer"));
    // window.set_position(gtk::WindowPosition::Center);
    // To silence the annying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    // window.preferred_width();
    window.set_default_size(630, 700);

    sys.refresh_all();
    let sys = Arc::new(Mutex::new(sys));
    procs
        .kill_button
        .connect_clicked(glib::clone!(@weak current_pid, @weak sys => move |_| {
            let sys = sys.lock().expect("failed to lock to kill a process");
            if let Some(process) = current_pid.get().and_then(|pid| sys.process(pid)) {
                process.kill();
            }
        }));

    let display_tab = DisplaySysInfo::new(&sys, &mut note, &settings);

    let settings = Rc::new(RefCell::new(settings));
    let network_tab = Rc::new(RefCell::new(Network::new(&mut note, &window, &sys)));
    display_disk::create_disk_info(&sys, &mut note);

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let display_tab = Rc::new(RefCell::new(display_tab));

    // I think it's now useless to have this one...
    v_box.append(&note.notebook);

    window.set_child(Some(&v_box));

    let process_dialogs: Rc<RefCell<Vec<process_dialog::ProcDialog>>> =
        Rc::new(RefCell::new(Vec::new()));
    let list_store = procs.list_store.clone();

    let rfs = Rc::new(RefCell::new(RequiredForSettings {
        process_refresh_timeout: Arc::new(Mutex::new(settings.borrow().refresh_processes_rate)),
        network_refresh_timeout: Arc::new(Mutex::new(settings.borrow().refresh_network_rate)),
        system_refresh_timeout: Arc::new(Mutex::new(settings.borrow().refresh_system_rate)),
        sys: sys.clone(),
        process_dialogs: process_dialogs.clone(),
        list_store,
        display_tab,
        network_tab: network_tab.clone(),
    }));

    setup_timeout(&rfs);
    setup_network_timeout(&rfs);
    setup_system_timeout(&rfs, &settings);

    let settings_action = gio::SimpleAction::new("settings", None);
    settings_action.connect_activate(glib::clone!(@weak settings, @weak rfs => move |_, _| {
        settings::show_settings_dialog(&settings, &rfs);
    }));

    info_button.connect_clicked(
        glib::clone!(@weak current_pid, @weak process_dialogs, @weak sys => move |_| {
                if let Some(pid) = current_pid.get() {
                    create_new_proc_diag(&process_dialogs, pid, &*sys.lock().expect("failed to lock to create new proc dialog"));
                }
            }
        ),
    );

    procs
        .left_tree
        .connect_row_activated(glib::clone!(@weak sys => move |tree_view, path, _| {
                let model = tree_view.model().expect("couldn't get model");
                let iter = model.iter(path).expect("couldn't get iter");
                let pid = model.get_value(&iter, 0)
                               .get::<u32>()
                               .expect("Model::get failed");
                create_new_proc_diag(&process_dialogs, Pid::from_u32(pid), &*sys.lock().expect("failed to lock to create new proc dialog (from tree)"));
            }
        ));

    let about = gio::SimpleAction::new("about", None);
    about.connect_activate(glib::clone!(@weak window => move |_, _| {
        let p = AboutDialog::new();
        p.set_authors(&["Guillaume Gomez"]);
        p.set_website_label("my website");
        p.set_website(Some("https://guillaume-gomez.fr/"));
        p.set_comments(Some("A process viewer GUI written with gtk-rs"));
        p.set_copyright(Some("Licensed under MIT"));
        p.set_program_name(Some("process-viewer"));
        // let pixbuf = Pixbuf::from_stream(Bytes::from_static(include_bytes!(
        //             concat!(env!("CARGO_MANIFEST_DIR"), "/assets/eye.png"))), None::<&gio::Cancellable>);
        // if let Ok(pixbuf) = pixbuf {
        //     let logo = Texture::for_pixbuf(&pixbuf);
        //     p.set_logo(Some(&logo));
        // }
        p.set_transient_for(Some(&window));
        p.set_modal(true);
        p.show();
    }));

    let new_task = gio::SimpleAction::new("new-task", None);
    new_task.connect_activate(glib::clone!(@weak window => move |_, _| {
        let dialog = gtk::Dialog::with_buttons(
            Some("Launch new executable"),
            Some(&window),
            gtk::DialogFlags::USE_HEADER_BAR,
            &[("Run", gtk::ResponseType::Other(0)), ("Cancel", gtk::ResponseType::Close)],
        );
        let input = Entry::new();

        // To set "run" button disabled by default.
        dialog.set_response_sensitive(gtk::ResponseType::Other(0), false);
        // To make "run" and "cancel" button take all spaces.
        // if let Some(run) = dialog.widget_for_response(gtk::ResponseType::Other(0)) {
        //     if let Some(parent) = run.parent() {
        //         match parent.downcast::<gtk::ButtonBox>() {
        //             Ok(parent) => parent.set_layout_style(gtk::ButtonBoxStyle::Expand),
        //             Err(e) => {
        //                 eprintln!(
        //                     "<Process_Viewer::build_ui> Failed to set layout style for new task \
        //                      button box: {}", e)
        //             }
        //         }
        //     }
        // }
        input.connect_changed(glib::clone!(@weak dialog => move |input| {
            if !input.text().is_empty() {
                    dialog.set_response_sensitive(gtk::ResponseType::Other(0), true);
               }
               else { dialog.set_response_sensitive(gtk::ResponseType::Other(0), false);
            }
        }));
        input.connect_activate(glib::clone!(@weak window, @weak dialog => move |input| {
            run_command(input, &window, &dialog);
        }));
        dialog.connect_response(glib::clone!(@weak input, @weak window => move |dialog, response| {
            match response {
                gtk::ResponseType::Close => {
                    dialog.close();
                }
                gtk::ResponseType::Other(0) => {
                    run_command(&input, &window, dialog);
                }
                _ => {}
            }
        }));

        dialog.content_area().append(&input);
        // To silence the annying warning:
        // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
        // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
        // allocate?"
        // dialog.preferred_width();
        dialog.set_size_request(400, 70);
        dialog.show();
    }));

    let graphs = gio::SimpleAction::new_stateful(
        "graphs",
        None,
        &settings.borrow().display_graph.to_variant(),
    );
    graphs.connect_activate(glib::clone!(@weak settings, @weak rfs => move |g, _| {
        let mut is_active = false;
        if let Some(g) = g.state() {
            let rfs = rfs.borrow();
            is_active = g.get().expect("couldn't get bool");
            rfs.display_tab.borrow().set_checkboxes_state(!is_active);
        }
        // We need to change the toggle state ourselves. `gio` dark magic.
        g.change_state(&(!is_active).to_variant());

        // We update the setting and save it!
        settings.borrow_mut().display_graph = !is_active;
        settings.borrow().save();
    }));

    let temperature = gio::SimpleAction::new_stateful(
        "temperature",
        None,
        &settings.borrow().display_fahrenheit.to_variant(),
    );
    temperature.connect_activate(move |g, _| {
        let mut is_active = false;
        if let Some(g) = g.state() {
            is_active = g.get().expect("couldn't get graph state");
        }
        // We need to change the toggle state ourselves. `gio` dark magic.
        g.change_state(&(!is_active).to_variant());

        // We update the setting and save it!
        settings.borrow_mut().display_fahrenheit = !is_active;
        settings.borrow().save();
    });

    application.add_action(&about);
    application.add_action(&graphs);
    application.add_action(&temperature);
    application.add_action(&settings_action);
    application.add_action(&new_task);
    application.add_action(&quit);

    window.set_widget_name(utils::MAIN_WINDOW_NAME);

    // window.add_events(gdk::EventMask::STRUCTURE_MASK);
    // TODO: ugly way to resize drawing area, I should find a better way
    // window.connect_configure_event(move |w, _| {
    //     // To silence the annoying warning:
    //     // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    //     // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    //     // allocate?"
    //     w.preferred_width();
    //     let w = w.size().0 - 130;
    //     let rfs = rfs.borrow();
    //     rfs.display_tab.borrow().set_size_request(w, 200);
    //     false
    // });

    application.connect_activate(glib::clone!(@weak procs.filter_entry as filter_entry, @weak network_tab, @weak window => move |_| {
        filter_entry.hide();
        network_tab.borrow().filter_entry.hide();
        window.present();
    }));

    let mut event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(glib::clone!(
        @weak note.notebook as notebook,
        @weak window as win
        => @default-return Inhibit(false),
        move |event_controller, key, _, modifier_| {
            let current_page = notebook.current_page();
            if current_page == Some(0) || current_page == Some(2) {
                // the process list
                if key == gdk::Key::Escape {
                    if current_page == Some(0) {
                        procs.hide_filter();
                    } else {
                        network_tab.borrow().hide_filter();
                    }
                } else if current_page == Some(0) {
                    let ret = event_controller.forward(&procs.search_bar);
                    if !procs.filter_entry.text_length() == 0 {
                        if win.focus()
                            != Some(procs.filter_entry.clone().upcast::<gtk::Widget>())
                        {
                            win.set_focus(Some(&procs.filter_entry));
                        }
                    }
                    return Inhibit(ret);
                } else {
                    let network = network_tab.borrow();
                    let ret = event_controller.forward(&network.search_bar);
                    if !network.filter_entry.text_length() == 0 {
                        if win.focus()
                            != Some(network.filter_entry.clone().upcast::<gtk::Widget>())
                        {
                            win.set_focus(Some(&network.filter_entry));
                        }
                    }
                    return Inhibit(ret);
                }
            }
            Inhibit(false)
        }),
    );
    window.add_controller(&event_controller);
}

fn main() {
    let application = gtk::Application::new(Some(APPLICATION_NAME), gio::ApplicationFlags::empty());

    application.connect_startup(move |app| {
        build_ui(app);
    });

    glib::set_application_name("process-viewer");
    application.run();
}
