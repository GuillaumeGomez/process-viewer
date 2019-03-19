//
// Process viewer
//
// Copyright (c) 2019 Guillaume Gomez
//

use glib;
use gtk;

use gio::ApplicationExt;
use glib::Cast;
use gtk::{BoxExt, ContainerExt, DialogExt, GridExt, GtkApplicationExt, GtkWindowExt, SpinButtonExt,
          SpinButtonSignals, WidgetExt};

use std::cell::RefCell;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::rc::Rc;

use APPLICATION_NAME;
use RequiredForSettings;
use {setup_timeout, setup_network_timeout, setup_system_timeout};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    pub display_fahrenheit: bool,
    pub display_graph: bool,
    // Timer length in milliseconds (500 minimum!).
    pub refresh_processes_rate: u32,
    // Timer length in milliseconds (500 minimum!).
    pub refresh_system_rate: u32,
    // Timer length in milliseconds (500 minimum!).
    pub refresh_network_rate: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            display_fahrenheit: false,
            display_graph: false,
            refresh_processes_rate: 1500,
            refresh_system_rate: 2000,
            refresh_network_rate: 1500,
        }
    }
}

impl Settings {
    pub fn load() -> Settings {
        let s = Self::get_settings_file_path();
        if s.exists() && s.is_file() {
            match serde_any::from_file::<Settings, _>(&s) {
                Ok(s) => s,
                Err(e) => {
                    show_error_dialog(
                        false,
                        format!("Error while opening '{}': {}", s.display(), e).as_str(),
                    );
                    Settings::default()
                }
            }
        } else {
            Settings::default()
        }
    }

    pub fn get_settings_file_path() -> PathBuf {
        let mut path = glib::get_user_config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(APPLICATION_NAME);
        path.push("settings.toml");
        path
    }

    pub fn save(&self) {
        let s = Self::get_settings_file_path();
        if !s.exists() {
            if let Some(parent_dir) = s.parent() {
                if !parent_dir.exists() {
                    if let Err(e) = create_dir_all(parent_dir) {
                        show_error_dialog(
                            false,
                            format!(
                                "Error while trying to build settings snapshot_directory '{}': {}",
                                parent_dir.display(),
                                e
                            ).as_str(),
                        );
                    }
                }
            }
        }
        if let Err(e) = serde_any::to_file(&s, self) {
            show_error_dialog(
                false,
                format!("Error while trying to save file: {}", e).as_str(),
            );
        }
    }
}

fn show_error_dialog(fatal: bool, text: &str) {
    let app = gio::Application::get_default()
        .expect("No default application")
        .downcast::<gtk::Application>()
        .expect("Default application has wrong type");

    let dialog = gtk::MessageDialog::new(
        app.get_active_window().as_ref(),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        text,
    );

    dialog.connect_response(move |dialog, _| {
        dialog.destroy();

        if fatal {
            app.quit();
        }
    });

    dialog.set_resizable(false);
    dialog.show_all();
}

pub fn build_spin(label: &str, grid: &gtk::Grid,
                  top: i32, refresh: u32) -> gtk::SpinButton {
    // Refresh rate.
    let refresh_label = gtk::Label::new(label);
    // We allow 0.5 to 5 seconds, in 0.1 second steps.
    let refresh_entry = gtk::SpinButton::new_with_range(0.5, 5., 0.1);

    refresh_label.set_halign(gtk::Align::Start);
    refresh_entry.set_hexpand(true);

    refresh_entry.set_value(f64::from(refresh) / 1000.);

    grid.attach(&refresh_label, 0, top, 1, 1);
    grid.attach(&refresh_entry, 1, top, 3, 1);
    refresh_entry
}

pub fn show_settings_dialog(
    application: &gtk::Application,
    settings: &Rc<RefCell<Settings>>,
    rfs: &Rc<RefCell<RequiredForSettings>>,
) {
    let bsettings = &*settings.borrow();
    // Create an empty dialog with close button.
    let dialog = gtk::Dialog::new_with_buttons(
        Some("Process Viewer settings"),
        application.get_active_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[("Close", gtk::ResponseType::Close)],
    );

    // All the UI widgets are going to be stored in a grid.
    let grid = gtk::Grid::new();
    grid.set_column_spacing(4);
    grid.set_row_spacing(4);
    grid.set_margin_bottom(12);

    let refresh_procs = build_spin("Processes refresh rate (in seconds)",
                                   &grid, 0, bsettings.refresh_processes_rate);
    let refresh_network = build_spin("System network refresh rate (in seconds)",
                                     &grid, 1, bsettings.refresh_network_rate);
    let refresh_sys = build_spin("System information refresh rate (in seconds)",
                                 &grid, 2, bsettings.refresh_system_rate);

    // Put the grid into the dialog's content area.
    let content_area = dialog.get_content_area();
    content_area.pack_start(&grid, true, true, 0);
    content_area.set_border_width(10);

    // Finally connect to all kinds of change notification signals for the different UI widgets.
    // Whenever something is changing we directly save the configuration file with the new values.
    refresh_procs.connect_value_changed(clone!(settings, rfs => move |entry| {
        let mut settings = settings.borrow_mut();
        let refresh_processes_rate = settings.refresh_processes_rate;
        setup_timeout(refresh_processes_rate, &rfs);

        settings.refresh_processes_rate = (entry.get_value() * 1000.) as u32;
        settings.save();
    }));
    refresh_network.connect_value_changed(clone!(settings, rfs => move |entry| {
        let mut settings = settings.borrow_mut();
        let refresh_network_rate = settings.refresh_network_rate;
        setup_network_timeout(refresh_network_rate, &rfs);

        settings.refresh_network_rate = (entry.get_value() * 1000.) as u32;
        settings.save();
    }));
    refresh_sys.connect_value_changed(clone!(settings, rfs => move |entry| {
        let refresh_system_rate = settings.borrow().refresh_system_rate;
        setup_system_timeout(refresh_system_rate, &rfs, &settings);

        let mut settings = settings.borrow_mut();
        settings.refresh_system_rate = (entry.get_value() * 1000.) as u32;
        settings.save();
    }));

    dialog.connect_response(move |dialog, _| {
        dialog.destroy();
    });

    dialog.set_resizable(false);
    dialog.show_all();
}
