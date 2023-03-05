//
// Process viewer
//
// Copyright (c) 2017 Guillaume Gomez
//

#![crate_type = "bin"]

use sysinfo::*;

use gdk::Texture;
use gdk_pixbuf::Pixbuf;
use gio::MemoryInputStream;
use glib::Bytes;
use gtk::prelude::*;
use gtk::{gdk, gdk_pixbuf, gio, glib};
use gtk::{AboutDialog, Dialog, Entry, MessageDialog};

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
                let memory = p.memory();
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
                pro.memory(),
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

#[derive(Clone)]
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

fn setup_timeout(rfs: &RequiredForSettings) {
    let (ready_tx, ready_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let sys = &rfs.sys;
    let process_dialogs = &rfs.process_dialogs;
    let list_store = &rfs.list_store;
    let process_refresh_timeout = &rfs.process_refresh_timeout;

    thread::spawn(
        glib::clone!(@weak sys, @weak process_refresh_timeout => move || {
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
            }
        } else {
            panic!("failed to lock sys to refresh UI");
        }
        dialogs.retain(|x| !x.need_remove());
        glib::Continue(true)
    }));
}

fn setup_network_timeout(rfs: &RequiredForSettings) {
    let (ready_tx, ready_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let network_refresh_timeout = &rfs.network_refresh_timeout;
    let network_tab = &rfs.network_tab;
    let sys = &rfs.sys;

    thread::spawn(
        glib::clone!(@weak sys, @weak network_refresh_timeout => move || {
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

fn setup_system_timeout(rfs: &RequiredForSettings, settings: &Rc<RefCell<Settings>>) {
    let (ready_tx, ready_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let system_refresh_timeout = &rfs.system_refresh_timeout;
    let sys = &rfs.sys;
    let display_tab = &rfs.display_tab;

    thread::spawn(
        glib::clone!(@weak sys, @weak system_refresh_timeout => move || {
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
        glib::clone!(@weak sys, @weak display_tab, @weak settings => @default-panic, move |_: bool| {
            let mut info = display_tab.borrow_mut();
            let sys = sys.lock().expect("failed to lock to update system");
            let display_fahrenheit = settings.borrow().display_fahrenheit;

            info.update_system_info(&*sys, display_fahrenheit);
            info.update_system_info_display(&*sys);
            glib::Continue(true)
        }),
    );
}

fn create_header_bar(stack: &gtk::Stack) -> (gtk::HeaderBar, gtk::Button) {
    let header_buttons = gtk::StackSwitcher::new();
    header_buttons.set_stack(Some(stack));

    let menu_bar = gio::Menu::new();

    menu_bar.append(Some("Launch new executable"), Some("app.new-task"));

    let settings_menu = gio::Menu::new();
    settings_menu.append(Some("Display temperature in °F"), Some("app.temperature"));
    settings_menu.append(Some("Display graphs"), Some("app.graphs"));
    settings_menu.append(Some("More settings..."), Some("app.settings"));
    menu_bar.append_section(None, &settings_menu);

    let more_menu = gio::Menu::new();
    more_menu.append(Some("About"), Some("app.about"));
    menu_bar.append_section(None, &more_menu);

    let menu = gio::Menu::new();
    menu.append(Some("Quit"), Some("app.quit"));
    menu_bar.append_section(None, &menu);

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu_bar)
        .build();
    menu_button.add_css_class("titlebar-button");

    let search_filter_button = gtk::Button::from_icon_name("edit-find-symbolic");
    search_filter_button.add_css_class("titlebar-button");

    let header_bar = gtk::HeaderBar::new();

    header_bar.pack_end(&menu_button);
    header_bar.pack_end(&search_filter_button);
    header_bar.set_title_widget(Some(&header_buttons));

    (header_bar, search_filter_button)
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    let stack = gtk::Stack::new();

    let (header_bar, search_filter_button) = create_header_bar(&stack);
    window.set_titlebar(Some(&header_bar));

    let mut sys =
        sysinfo::System::new_with_specifics(RefreshKind::everything().without_users_list());
    let procs = Procs::new(sys.processes(), &stack);
    let current_pid = Rc::clone(&procs.current_pid);
    let info_button = procs.info_button.clone();

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

    let settings = Settings::load();
    let display_tab = DisplaySysInfo::new(&sys, &stack, &settings);

    let settings = Rc::new(RefCell::new(settings));
    let network_tab = Rc::new(RefCell::new(Network::new(&stack, &sys)));
    display_disk::create_disk_info(&sys, &stack);

    let display_tab = Rc::new(RefCell::new(display_tab));

    window.set_child(Some(&stack));

    let process_dialogs: Rc<RefCell<Vec<process_dialog::ProcDialog>>> =
        Rc::new(RefCell::new(Vec::new()));
    let list_store = procs.list_store.clone();

    let rfs = RequiredForSettings {
        process_refresh_timeout: Arc::new(Mutex::new(settings.borrow().refresh_processes_rate)),
        network_refresh_timeout: Arc::new(Mutex::new(settings.borrow().refresh_network_rate)),
        system_refresh_timeout: Arc::new(Mutex::new(settings.borrow().refresh_system_rate)),
        sys: sys.clone(),
        process_dialogs: process_dialogs.clone(),
        list_store,
        display_tab,
        network_tab: network_tab.clone(),
    };

    setup_timeout(&rfs);
    setup_network_timeout(&rfs);
    setup_system_timeout(&rfs, &settings);

    let settings_action = gio::SimpleAction::new("settings", None);
    settings_action.connect_activate(glib::clone!(@weak settings, @strong rfs => move |_, _| {
        settings::show_settings_dialog(&settings, &rfs);
    }));

    info_button.connect_clicked(
        glib::clone!(@weak current_pid, @weak process_dialogs, @weak sys => move |_| {
            if let Some(pid) = current_pid.get() {
                create_new_proc_diag(
                    &process_dialogs,
                    pid,
                    &*sys.lock().expect("failed to lock to create new proc dialog"),
                );
            }
        }),
    );

    procs
        .left_tree
        .connect_row_activated(glib::clone!(@weak sys => move |tree_view, path, _| {
                let model = tree_view.model().expect("couldn't get model");
                let iter = model.iter(path).expect("couldn't get iter");
                let pid = model.get_value(&iter, 0)
                               .get::<u32>()
                               .expect("Model::get failed");
                create_new_proc_diag(
                    &process_dialogs,
                    Pid::from_u32(pid),
                    &*sys.lock().expect("failed to lock to create new proc dialog (from tree)"),
                );
            }
        ));

    let about = gio::SimpleAction::new("about", None);
    about.connect_activate(glib::clone!(@weak window => move |_, _| {
        let p = AboutDialog::builder()
            .authors(vec!["Guillaume Gomez".to_owned()])
            .website_label("my website")
            .website("https://guillaume-gomez.fr/")
            .comments("A process viewer GUI written with gtk-rs")
            .copyright("Licensed under MIT")
            .program_name("process-viewer")
            .transient_for(&window)
            .modal(true);

        let bytes = Bytes::from_static(include_bytes!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/eye.png")));
        let memory_stream = MemoryInputStream::from_bytes(&bytes);
        let pixbuf = Pixbuf::from_stream(&memory_stream, None::<&gio::Cancellable>);
        let p = if let Ok(pixbuf) = pixbuf {
            let logo = Texture::for_pixbuf(&pixbuf);
            p.logo(&logo)
        } else {
            p
        };

        p.build().show();
    }));

    let new_task = gio::SimpleAction::new("new-task", None);
    new_task.connect_activate(glib::clone!(@weak window => move |_, _| {
        let dialog = gtk::Dialog::with_buttons(
            Some("Launch new executable"),
            Some(&window),
            gtk::DialogFlags::MODAL,
            &[("Run", gtk::ResponseType::Other(0)), ("Cancel", gtk::ResponseType::Close)],
        );
        let input = Entry::builder()
            .css_classes(vec!["button-with-margin".to_owned()])
            .vexpand(false)
            .hexpand(true)
            .build();

        // To set "run" button disabled by default.
        dialog.set_response_sensitive(gtk::ResponseType::Other(0), false);

        input.connect_changed(glib::clone!(@weak dialog => move |input| {
            if !input.text().is_empty() {
                dialog.set_response_sensitive(gtk::ResponseType::Other(0), true);
            } else {
                dialog.set_response_sensitive(gtk::ResponseType::Other(0), false);
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
        dialog.set_size_request(400, 70);
        dialog.show();
    }));

    let graphs = gio::SimpleAction::new_stateful(
        "graphs",
        None,
        settings.borrow().display_graph.to_variant(),
    );
    graphs.connect_activate(glib::clone!(@weak settings => move |g, _| {
        let mut is_active = false;
        if let Some(g) = g.state() {
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
        settings.borrow().display_fahrenheit.to_variant(),
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
    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate(glib::clone!(@weak application => move |_,_| {
        application.quit();
    }));
    application.set_accels_for_action("app.quit", &["<Primary>Q"]);
    let finder = gio::SimpleAction::new("finder", None);
    // Little hack to correctly handle `ctrl+F` shortcut.
    finder.connect_activate(glib::clone!(@weak search_filter_button => move |_,_| {
        search_filter_button.emit_clicked();
    }));
    application.set_accels_for_action("app.finder", &["<Primary>F"]);

    application.add_action(&about);
    application.add_action(&graphs);
    application.add_action(&temperature);
    application.add_action(&settings_action);
    application.add_action(&new_task);
    application.add_action(&quit);
    application.add_action(&finder);

    window.set_widget_name(utils::MAIN_WINDOW_NAME);

    application.connect_activate(glib::clone!(@weak window => move |_| {
        window.present();
    }));

    procs.search_bar.set_key_capture_widget(Some(&window));

    fn revert_display(search_bar: &gtk::SearchBar) {
        if search_bar.is_search_mode() {
            search_bar.set_search_mode(false);
        } else {
            search_bar.set_search_mode(true);
        }
    }

    search_filter_button.connect_clicked(glib::clone!(
        @strong stack,
        @weak procs.search_bar as procs_search_bar,
        @weak network_tab,
        => move |_| {
        if let Some(name) = stack.visible_child_name() {
            match name.as_str() {
                "Processes" => revert_display(&procs_search_bar),
                "Networks" => revert_display(&network_tab.borrow().search_bar),
                _ => {}
            };
        }
    }));

    stack.connect_visible_child_notify(move |s| {
        if let Some(name) = s.visible_child_name() {
            match name.as_str() {
                "Processes" => {
                    procs.search_bar.set_key_capture_widget(Some(&window));
                    network_tab
                        .borrow()
                        .search_bar
                        .set_key_capture_widget(None::<&gtk::Widget>);
                    search_filter_button.set_sensitive(true);
                    return;
                }
                "Networks" => {
                    network_tab
                        .borrow()
                        .search_bar
                        .set_key_capture_widget(Some(&window));
                    procs
                        .search_bar
                        .set_key_capture_widget(None::<&gtk::Widget>);
                    search_filter_button.set_sensitive(true);
                    return;
                }
                _ => {}
            }
        }
        search_filter_button.set_sensitive(false);
        procs
            .search_bar
            .set_key_capture_widget(None::<&gtk::Widget>);
        network_tab
            .borrow()
            .search_bar
            .set_key_capture_widget(None::<&gtk::Widget>);
    });
}

fn main() {
    let application = gtk::Application::new(Some(APPLICATION_NAME), gio::ApplicationFlags::empty());

    application.connect_startup(move |app| {
        let provider = gtk::CssProvider::new();
        // Style needed for graph.
        provider.load_from_data(
            r#"
graph_widget {
    color: @theme_fg_color;
}

.titlebar-button {
    background-color: @theme_bg_color;
    border-radius: 4px;
}
.titlebar-button:hover {
    background-color: shade(@theme_bg_color, 1.50);
}

.button-with-margin {
    margin: 6px;
}
"#,
        );
        gtk::StyleContext::add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        build_ui(app);
    });

    glib::set_application_name("process-viewer");
    application.run();
}
