//
// Process viewer
//
// Copyright (c) 2019 Guillaume Gomez
//

use glib;
use gtk;

use gio::ApplicationExt;
use gtk::{
    BoxExt, ContainerExt, DialogExt, GridExt, GtkWindowExt, SpinButtonExt, SpinButtonSignals,
    WidgetExt,
};

use std::cell::RefCell;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

use utils::{get_app, get_main_window};
use RequiredForSettings;
use APPLICATION_NAME;

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
    fn load_from_file(p: &PathBuf) -> Result<Settings, String> {
        let mut input = String::new();
        let mut file =
            File::open(p).map_err(|e| format!("Error while opening '{}': {}", p.display(), e))?;
        file.read_to_string(&mut input)
            .map_err(|e| format!("Error while opening '{}': {}", p.display(), e))?;
        toml::from_str(&input).map_err(|e| format!("Error while opening '{}': {}", p.display(), e))
    }

    pub fn load() -> Settings {
        let s = Self::get_settings_file_path();
        if s.exists() && s.is_file() {
            match Self::load_from_file(&s) {
                Ok(settings) => settings,
                Err(e) => {
                    show_error_dialog(false, &e);
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
                            )
                            .as_str(),
                        );
                        return;
                    }
                }
            }
        }
        match toml::to_string_pretty(&self) {
            Ok(output) => {
                if let Err(e) = std::fs::write(&s, output) {
                    show_error_dialog(
                        false,
                        format!("Error while trying to save file: {}", e).as_str(),
                    );
                }
            }
            Err(e) => {
                show_error_dialog(
                    false,
                    format!("Error while trying to save file: {}", e).as_str(),
                );
            }
        }
    }
}

fn show_error_dialog(fatal: bool, text: &str) {
    let dialog = gtk::MessageDialog::new(
        get_main_window().as_ref(),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        text,
    );

    dialog.connect_response(move |dialog, _| {
        dialog.close();

        if fatal {
            get_app().quit();
        }
    });

    dialog.set_resizable(false);
    dialog.show_all();
}

pub fn build_spin(label: &str, grid: &gtk::Grid, top: i32, refresh: u32) -> gtk::SpinButton {
    // Refresh rate.
    let refresh_label = gtk::Label::new(Some(label));
    // We allow 0.5 to 5 seconds, in 0.1 second steps.
    let refresh_entry = gtk::SpinButton::with_range(0.5, 5., 0.1);

    refresh_label.set_halign(gtk::Align::Start);
    refresh_entry.set_hexpand(true);

    refresh_entry.set_value(f64::from(refresh) / 1000.);

    grid.attach(&refresh_label, 0, top, 1, 1);
    grid.attach(&refresh_entry, 1, top, 3, 1);
    refresh_entry
}

pub fn show_settings_dialog(
    settings: &Rc<RefCell<Settings>>,
    rfs: &Rc<RefCell<RequiredForSettings>>,
) {
    let bsettings = &*settings.borrow();
    // Create an empty dialog with close button.
    let dialog = gtk::Dialog::with_buttons(
        Some("Process Viewer settings"),
        get_main_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[("Close", gtk::ResponseType::Close)],
    );

    // All the UI widgets are going to be stored in a grid.
    let grid = gtk::Grid::new();
    grid.set_column_spacing(4);
    grid.set_row_spacing(4);
    grid.set_margin_bottom(12);

    let refresh_procs = build_spin(
        "Processes refresh rate (in seconds)",
        &grid,
        0,
        bsettings.refresh_processes_rate,
    );
    let refresh_network = build_spin(
        "System network refresh rate (in seconds)",
        &grid,
        1,
        bsettings.refresh_network_rate,
    );
    let refresh_sys = build_spin(
        "System information refresh rate (in seconds)",
        &grid,
        2,
        bsettings.refresh_system_rate,
    );

    // Put the grid into the dialog's content area.
    let content_area = dialog.get_content_area();
    content_area.pack_start(&grid, true, true, 0);
    content_area.set_border_width(10);

    // Finally connect to all kinds of change notification signals for the different UI widgets.
    // Whenever something is changing we directly save the configuration file with the new values.
    refresh_procs.connect_value_changed(clone!(@weak settings, @weak rfs => move |entry| {
        let mut settings = settings.borrow_mut();
        settings.refresh_processes_rate = (entry.get_value() * 1000.) as u32;
        *rfs.borrow().process_refresh_timeout.lock().expect("failed to lock process_refresh_timeout") = settings.refresh_processes_rate;
        settings.save();
    }));
    refresh_network.connect_value_changed(clone!(@weak settings, @weak rfs => move |entry| {
        let mut settings = settings.borrow_mut();
        settings.refresh_network_rate = (entry.get_value() * 1000.) as u32;
        *rfs.borrow().network_refresh_timeout.lock().expect("failed to lock network_refresh_timeout") = settings.refresh_network_rate;
        settings.save();
    }));
    refresh_sys.connect_value_changed(clone!(@weak settings, @weak rfs => move |entry| {
        let mut settings = settings.borrow_mut();
        settings.refresh_system_rate = (entry.get_value() * 1000.) as u32;
        *rfs.borrow().system_refresh_timeout.lock().expect("failed to lock system_refresh_timeout") = settings.refresh_system_rate;
        settings.save();
    }));

    dialog.connect_response(move |dialog, _| {
        dialog.close();
    });

    dialog.set_resizable(false);
    dialog.show_all();
}
