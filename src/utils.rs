use gtk::gio;
use gtk::glib::Cast;
use gtk::prelude::*;

use std::ops::Index;

pub const MAIN_WINDOW_NAME: &str = "main-window";

#[derive(Debug)]
pub struct RotateVec<T> {
    data: Vec<T>,
    start: usize,
}

impl<T> RotateVec<T> {
    pub fn new(d: Vec<T>) -> RotateVec<T> {
        RotateVec { data: d, start: 0 }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn move_start(&mut self) {
        if self.start > 0 {
            self.start -= 1;
        } else {
            self.start = self.data.len() - 1;
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let pos = self.get_real_pos(index);
        self.data.get_mut(pos)
    }

    fn get_real_pos(&self, index: usize) -> usize {
        if self.start + index >= self.data.len() {
            self.start + index - self.data.len()
        } else {
            index + self.start
        }
    }
}

pub fn format_number(nb: u64) -> String {
    format_number_full(nb, true)
}

pub fn format_number_full(nb: u64, use_unit: bool) -> String {
    if nb < 1_000 {
        format!("{}{}", nb, if use_unit { " B" } else { "" })
    } else if nb < 1_000_000 {
        format!(
            "{}.{}{}",
            nb / 1_000,
            nb / 100 % 10,
            if use_unit { " KB" } else { "" }
        )
    } else if nb < 1_000_000_000 {
        format!(
            "{}.{}{}",
            nb / 1_000_000,
            nb / 100_000 % 10,
            if use_unit { " MB" } else { "" }
        )
    } else if nb < 1_000_000_000_000 {
        format!(
            "{}.{}{}",
            nb / 1_000_000_000,
            nb / 100_000_000 % 10,
            if use_unit { " GB" } else { "" }
        )
    } else {
        format!(
            "{}.{}{}",
            nb / 1_000_000_000_000,
            nb / 100_000_000_000 % 10,
            if use_unit { " TB" } else { "" }
        )
    }
}

pub fn graph_label_units(v: f32) -> [String; 4] {
    graph_label_units_full(v, true)
}

pub fn graph_label(v: f32) -> [String; 4] {
    graph_label_units_full(v, false)
}

pub fn graph_label_units_full(v: f32, use_unit: bool) -> [String; 4] {
    if v < 1_000. {
        [
            v.to_string(),
            format!("{}", v / 2.),
            "0".to_owned(),
            if use_unit { "B" } else { "" }.to_owned(),
        ]
    } else if v < 1_000_000. {
        [
            format!("{:.1}", v / 1_000f32),
            format!("{:.1}", v / 2_000f32),
            "0".to_owned(),
            if use_unit { "KB" } else { "K" }.to_owned(),
        ]
    } else if v < 1_000_000_000. {
        [
            format!("{:.1}", v / 1_000_000f32),
            format!("{:.1}", v / 2_000_000f32),
            "0".to_owned(),
            if use_unit { "MB" } else { "M" }.to_owned(),
        ]
    } else if v < 1_000_000_000_000. {
        [
            format!("{:.1}", v / 1_000_000_000f32),
            format!("{:.1}", v / 2_000_000_000f32),
            "0".to_owned(),
            if use_unit { "GB" } else { "G" }.to_owned(),
        ]
    } else {
        [
            format!("{:.1}", v / 1_000_000_000_000f32),
            format!("{:.1}", v / 2_000_000_000_000f32),
            "0".to_owned(),
            if use_unit { "TB" } else { "T" }.to_owned(),
        ]
    }
}

impl<T> Index<usize> for RotateVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.data[self.get_real_pos(index)]
    }
}

pub fn get_app() -> gtk::Application {
    gio::Application::default()
        .expect("No default application")
        .downcast::<gtk::Application>()
        .expect("Default application has wrong type")
}

pub fn get_main_window() -> Option<gtk::Window> {
    for window in get_app().windows() {
        if window.widget_name() == MAIN_WINDOW_NAME {
            return Some(window);
        }
    }
    None
}
