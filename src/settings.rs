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
use setup_timeout;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    pub display_degree: bool,
    // Timer length in milliseconds (500 minimum!).
    pub refresh_rate: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            display_degree: true,
            refresh_rate: 1000,
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

pub fn show_settings_dialog(
    application: &gtk::Application,
    settings: &Rc<RefCell<Settings>>,
    rfs: &Rc<RefCell<RequiredForSettings>>,
) {
    // Create an empty dialog with close button.
    let dialog = gtk::Dialog::new_with_buttons(
        Some("Process Viewer settings"),
        application.get_active_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[("Close", gtk::ResponseType::Close.into())],
    );

    // All the UI widgets are going to be stored in a grid.
    let grid = gtk::Grid::new();
    grid.set_column_spacing(4);
    grid.set_row_spacing(4);
    grid.set_margin_bottom(12);

    // Refresh rate.
    let refresh_label = gtk::Label::new("Refresh rate (in seconds)");
    // We allow 0.5 to 10 seconds, in 0.1 second steps.
    let refresh_entry = gtk::SpinButton::new_with_range(0.5, 10., 0.1);

    refresh_label.set_halign(gtk::Align::Start);
    refresh_entry.set_hexpand(true);

    refresh_entry.set_value(settings.borrow().refresh_rate as f64 / 1000.);

    grid.attach(&refresh_label, 0, 0, 1, 1);
    grid.attach(&refresh_entry, 1, 0, 3, 1);

    // Put the grid into the dialog's content area.
    let content_area = dialog.get_content_area();
    content_area.pack_start(&grid, true, true, 0);
    content_area.set_border_width(10);

    // Finally connect to all kinds of change notification signals for the different UI widgets.
    // Whenever something is changing we directly save the configuration file with the new values.
    refresh_entry.connect_value_changed(clone!(settings, rfs => move |entry| {
        settings.borrow_mut().refresh_rate = (entry.get_value() * 1000.) as u32;
        settings.borrow().save();
        setup_timeout(settings.borrow().refresh_rate, &rfs);
    }));

    dialog.connect_response(move |dialog, _| {
        dialog.destroy();
    });

    dialog.set_resizable(false);
    dialog.show_all();
}
