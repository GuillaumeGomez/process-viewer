#![crate_type = "bin"]

extern crate cairo;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate glib;
extern crate gtk;
extern crate libc;
extern crate pango;
extern crate sysinfo;

use sysinfo::*;

use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    AboutDialog, Button, Dialog, Entry, IconSize, Image, Label, MenuBar, MenuItem, MessageDialog,
};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};
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
    let entries: &HashMap<i32, Process> = system.get_process_list();
    let mut seen: HashSet<i32> = HashSet::new();

    if let Some(mut iter) = list.get_iter_first() {
        let mut valid = true;
        while valid {
            if let Some(pid) = list.get_value(&iter, 0).get::<i32>() {
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
    }

    for (pid, pro) in entries.iter() {
        if !seen.contains(pid) {
            create_and_fill_model(list, pro.pid, &format!("{:?}", &pro.cmd), &pro.name,
                                  pro.cpu_usage, pro.memory);
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

fn start_detached_process(line: &str) -> Option<String> {
    let args = parse_entry(line);
    let command = args[0].clone();

    if let Err(_) = Command::new(&command)
                            .args(&args)
                            .before_exec(|| {
                                unsafe { libc::setsid() };
                                Ok(())
                            })
                            .stdin(Stdio::null())
                            .stderr(Stdio::null())
                            .stdout(Stdio::null())
                            .spawn() {
        Some(format!("Failed to start '{}'", &command))
    } else {
        None
    }
}

fn run_command(input: &Entry, window: &gtk::Window, d: &Dialog) {
    if let Some(text) = input.get_text() {
        let x = if let Some(x) = start_detached_process(&text) {
            x
        } else {
            "The command started successfully".to_owned()
        };
        d.destroy();
        let m = MessageDialog::new(Some(window),
                                   gtk::DIALOG_DESTROY_WITH_PARENT,
                                   gtk::MessageType::Info,
                                   gtk::ButtonsType::Ok,
                                   &x);
        m.run();
        m.destroy();
    }
}

fn create_image_menu_item(label: &str, icon: &str) -> MenuItem {
    let item = gtk::MenuItem::new();
    let _box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let image = Image::new_from_icon_name(icon, IconSize::Menu.into());
    let label = Label::new(Some(label));

    _box.add(&image);
    _box.add(&label);
    item.add(&_box);
    item
}

fn main() {
    gtk::init().expect("GTK couldn't start normally");

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let sys = sysinfo::System::new();
    let start_time = unsafe { if let Some(p) = sys.get_process(libc::getpid()) {
        p.start_time
    } else {
        0
    }};
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
    // To silence the annying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    window.get_preferred_width();
    window.set_default_size(500, 700);

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

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let menu_bar = MenuBar::new();
    let menu = gtk::Menu::new();
    let more_menu = gtk::Menu::new();
    let settings_menu = gtk::Menu::new();
    let file = gtk::MenuItem::new_with_mnemonic("_File");
    let new_task = create_image_menu_item("Launch new executable", "system-run");
    let quit = create_image_menu_item("Quit", "application-exit");
    let settings = gtk::MenuItem::new_with_mnemonic("_Settings");
    let temperature_setting = gtk::CheckMenuItem::new_with_label("Display temperature in °F");
    let graph_setting = gtk::CheckMenuItem::new_with_label("Display graphs");
    let more = gtk::MenuItem::new_with_label("?");
    let about = create_image_menu_item("About", "help-about");

    let ram_check_box = display_tab.ram_check_box.clone();
    let swap_check_box = display_tab.swap_check_box.clone();
    let temperature_check_box = display_tab.temperature_check_box.clone();
    graph_setting.connect_toggled(move |g| {
        let is_active = g.get_active();
        ram_check_box.set_active(is_active);
        swap_check_box.set_active(is_active);
        if let Some(ref temperature_check_box) = temperature_check_box {
            temperature_check_box.set_active(is_active);
        }
    });

    menu.append(&new_task);
    menu.append(&quit);
    file.set_submenu(Some(&menu));
    menu_bar.append(&file);
    settings_menu.append(&temperature_setting);
    settings_menu.append(&graph_setting);
    settings.set_submenu(Some(&settings_menu));
    menu_bar.append(&settings);
    more_menu.append(&about);
    more.set_submenu(Some(&more_menu));
    menu_bar.append(&more);

    v_box.pack_start(&menu_bar, false, false, 0);
    v_box.pack_start(&note.notebook, true, true, 0);
    //v_box.set_size_request(v_box.get_preferred_width().1, 600);

    window.add(&v_box);
    window.show_all();
    window.activate();

    let list_store = procs.list_store.clone();
    let run1 = running_since.clone();
    let run2 = running_since.clone();
    gtk::timeout_add(1000, move || {
        *run1.borrow_mut() += 1;
        // first part, deactivate sorting
        let sorted = TreeSortableExtManual::get_sort_column_id(&list_store);
        list_store.set_unsorted();

        // we update the tree view
        update_window(&list_store, &sys, &mut display_tab, temperature_setting.get_active());

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
            process_dialog::create_process_dialog(&process, &window1, start_time,
                                                  run2.borrow().clone());
        }
    });
    let window2 = window.clone();
    procs.left_tree.connect_row_activated(move |tree_view, path, _| {
        let model = tree_view.get_model().unwrap();
        let iter = model.get_iter(path).unwrap();
        let sys = sys3.borrow();
        if let Some(process) = sys.get_process(model.get_value(&iter, 0).get().unwrap()) {
            process_dialog::create_process_dialog(&process, &window2, start_time,
                                                  running_since.borrow().clone());
        }
    });

    quit.connect_activate(|_| {
        gtk::main_quit();
    });
    let window3 = window.clone();
    about.connect_activate(move |_| {
        let p = AboutDialog::new();
        p.set_authors(&["Guillaume Gomez"]);
        p.set_website_label(Some("my website"));
        p.set_website(Some("https://guillaume-gomez.fr/"));
        p.set_comments(Some("A process viewer GUI wrote with gtk-rs"));
        p.set_copyright(Some("This is under MIT license"));
        p.set_transient_for(Some(&window3));
        p.set_program_name("process-viewer");
        let logo = Pixbuf::new_from_file("assets/eye.png");
        if let Ok(logo) = logo {
            p.set_logo(Some(&logo));
        }
        p.run();
        p.destroy();
    });
    new_task.connect_activate(move |_| {
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
                Some(ref x) if x.len() > 0 => run2.set_sensitive(true),
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

    gtk::main();
}
