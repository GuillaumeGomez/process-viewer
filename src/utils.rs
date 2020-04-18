use graph::Graph;

use gdk_pixbuf::Pixbuf;
use gio::{self, MemoryInputStream};
use glib::{Bytes, Cast};
use gtk::{ButtonExt, GtkApplicationExt, Inhibit, WidgetExt};

use std::cell::RefCell;
use std::ops::Index;
use std::rc::Rc;

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

    /*pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(self.get_real_pos(index))
    }*/

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
        return format!("{}{}", nb, if use_unit { " B" } else { "" });
    }
    if nb < 1_000_000 {
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

pub fn connect_graph(graph: Graph) -> Rc<RefCell<Graph>> {
    let area = graph.area.clone();
    let graph = Rc::new(RefCell::new(graph));
    area.connect_draw(
        clone!(@weak graph => @default-return Inhibit(false), move |w, c| {
            graph.borrow()
                 .draw(c,
                       f64::from(w.get_allocated_width()),
                       f64::from(w.get_allocated_height()));
            Inhibit(false)
        }),
    );
    graph
}

impl<T> Index<usize> for RotateVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.data[self.get_real_pos(index)]
    }
}

pub fn get_app() -> gtk::Application {
    gio::Application::get_default()
        .expect("No default application")
        .downcast::<gtk::Application>()
        .expect("Default application has wrong type")
}

pub fn get_main_window() -> Option<gtk::Window> {
    for window in get_app().get_windows() {
        if window.get_widget_name().as_ref().map(|ref s| s.as_str()) == Some(MAIN_WINDOW_NAME) {
            return Some(window);
        }
    }
    None
}

pub fn create_button_with_image(image_bytes: &'static [u8], fallback_text: &str) -> gtk::Button {
    let button = gtk::Button::new();
    let memory_stream = MemoryInputStream::new_from_bytes(&Bytes::from_static(image_bytes));
    let image =
        Pixbuf::new_from_stream_at_scale(&memory_stream, 32, 32, true, None::<&gio::Cancellable>);
    if let Ok(image) = image {
        let image = gtk::Image::new_from_pixbuf(Some(&image));
        button.set_image(Some(&image));
        button.set_always_show_image(true);
    } else {
        button.set_label(fallback_text);
    }
    button
}
