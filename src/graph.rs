use cairo;
use gtk::{self, BoxExt, ContainerExt, DrawingArea, ScrolledWindowExt, StateFlags, WidgetExt};
use std::cell::RefCell;
use gdk::{self, WindowExt};

use std::rc::Rc;
use std::time::Instant;

use color::Color;
use utils::RotateVec;

const LEFT_WIDTH: f64 = 31.;

pub struct Graph {
    elapsed: Instant,
    colors: Vec<Color>,
    pub data: Vec<RotateVec<f64>>,
    vertical_layout: gtk::Box,
    scroll_layout: gtk::ScrolledWindow,
    horizontal_layout: gtk::Box,
    pub area: DrawingArea,
    max: Option<RefCell<f64>>,
    keep_max: bool,
    display_labels: RefCell<bool>,
    initial_diff: Option<i32>,
    label_callbacks: Option<Box<Fn(f64) -> [String; 4]>>,
    labels_layout_width: i32,
}

impl Graph {
    // If `max` is `None`, the graph will expect values between 0 and 1.
    //
    // If `keep_max` is set to `true`, then this value will never go down, meaning that graphs
    // won't rescale down. It is not taken into account if `max` is `None`.
    pub fn new(max: Option<f64>, keep_max: bool) -> Graph {
        let g = Graph {
            elapsed: Instant::now(),
            colors: vec!(),
            data: vec!(),
            vertical_layout: gtk::Box::new(gtk::Orientation::Vertical, 0),
            scroll_layout: gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>),
            horizontal_layout: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            area: DrawingArea::new(),
            max: if let Some(max) = max { Some(RefCell::new(max)) } else { None },
            keep_max,
            display_labels: RefCell::new(true),
            initial_diff: None,
            label_callbacks: None,
            labels_layout_width: 80,
        };
        g.scroll_layout.set_min_content_width(g.labels_layout_width);
        g.scroll_layout.add(&g.vertical_layout);
        g.horizontal_layout.pack_start(&g.area, true, true, 0);
        g.horizontal_layout.pack_start(&g.scroll_layout, false, true, 10);
        g.horizontal_layout.set_margin_start(5);
        g
    }

    /// Changes the size of the layout containing labels (the one on the right).
    pub fn set_labels_width(&mut self, labels_layout_width: u32) {
        self.scroll_layout.set_min_content_width(labels_layout_width as i32);
        self.labels_layout_width = labels_layout_width as i32;
    }

    pub fn set_label_callbacks(&mut self, label_callbacks: Option<Box<Fn(f64) -> [String; 4]>>) {
        self.label_callbacks = label_callbacks;
    }

    pub fn set_display_labels(&self, display_labels: bool) {
        *self.display_labels.borrow_mut() = display_labels;
        if display_labels == true {
            self.scroll_layout.show_all();
        } else {
            self.scroll_layout.hide();
        }
        self.invalidate();
    }

    pub fn hide(&self) {
        self.horizontal_layout.hide();
    }

    pub fn show_all(&self) {
        self.horizontal_layout.show_all();
        if *self.display_labels.borrow() == false {
            self.scroll_layout.hide();
        }
    }

    pub fn attach_to(&self, to: &gtk::Box) {
        to.add(&self.horizontal_layout);
    }

    pub fn push(&mut self, d: RotateVec<f64>, s: &str, override_color: Option<usize>) {
        let c = if let Some(over) = override_color {
            Color::generate(over)
        } else {
            Color::generate(self.data.len() + 11)
        };
        let l = gtk::Label::new(Some(s));
        l.override_color(StateFlags::from_bits(0).expect("from_bits failed"), &c.to_gdk());
        self.vertical_layout.add(&l);
        self.colors.push(c);
        self.data.push(d);
    }

    fn draw_labels(&self, c: &cairo::Context, max: f64, height: f64) {
        if let Some(ref call) = self.label_callbacks {
            let entries = call(max);
            let font_size = 8.;

            c.set_source_rgb(0., 0., 0.);
            c.set_font_size(font_size);

            c.move_to(LEFT_WIDTH - 4. - entries[0].len() as f64 * 4., font_size);
            c.show_text(entries[0].as_str());

            c.move_to(LEFT_WIDTH - 4. - entries[1].len() as f64 * 4., height / 2.);
            c.show_text(entries[1].as_str());

            c.move_to(LEFT_WIDTH - 4. - entries[2].len() as f64 * 4., height - 2.);
            c.show_text(entries[2].as_str());

            c.move_to(font_size - 1., height / 2. + 4. * (entries[3].len() >> 1) as f64);
            c.rotate(-1.5708);
            c.show_text(entries[3].as_str());
        }
    }

    pub fn draw(&self, c: &cairo::Context, width: f64, height: f64) {
        let x_start = if self.label_callbacks.is_some() {
            LEFT_WIDTH
        } else {
            1.0
        };

        c.set_source_rgb(0.95, 0.95, 0.95);
        c.rectangle(x_start, 1.0, width - 1.0, height - 2.0);
        c.fill();
        c.set_source_rgb(0.0, 0.0, 0.0);
        c.set_line_width(1.0);
        c.move_to(x_start, 0.0);
        c.line_to(x_start, height);
        c.move_to(width, 0.0);
        c.line_to(width, height);
        c.move_to(x_start, 0.0);
        c.line_to(width, 0.0);
        c.move_to(x_start, height);
        c.line_to(width, height);

        // For now it's always 60 seconds.
        let time = 60.;

        let elapsed = self.elapsed.elapsed().as_secs() % 5;
        let x_step = (width - 2.0 - x_start) * 5.0 / (time as f64);
        let mut current = width - elapsed as f64 * (x_step / 5.0) - 1.0;
        if x_step < 0.1 {
            c.stroke();
            return;
        }

        while current > x_start {
            c.move_to(current, 0.0);
            c.line_to(current, height);
            current -= x_step;
        }
        let step = height / 10.0;
        current = step - 1.0;
        while current < height - 1. {
            c.move_to(x_start, current);
            c.line_to(width - 1.0, current);
            current += step;
        }
        c.stroke();

        if let Some(ref self_max) = self.max {
            let mut max = if self.keep_max { *self_max.borrow() } else { 1. };
            let len = self.data[0].len() - 1;
            for x in 0..len {
                for entry in &self.data {
                    if entry[x] > max {
                        max = entry[x];
                    }
                }
            }
            if !self.data.is_empty() && !self.data[0].is_empty() {
                let len = self.data[0].len() - 1;
                let step = (width - 2.0 - x_start) / len as f64;
                current = x_start + 1.0;
                let mut index = len;
                while current > x_start && index > 0 {
                    for (entry, color) in self.data.iter().zip(self.colors.iter()) {
                        c.set_source_rgb(color.r, color.g, color.b);
                        c.move_to(current + step, height - entry[index - 1] / max * (height - 1.0));
                        c.line_to(current, height - entry[index] / max * (height - 1.0));
                        c.stroke();
                    }
                    current += step;
                    index -= 1;
                }
            }
            if max > *self_max.borrow() || !self.keep_max {
                *self_max.borrow_mut() = max;
            }
            self.draw_labels(c, max, height);
        } else if !self.data.is_empty() && !self.data[0].is_empty() {
            let len = self.data[0].len() - 1;
            let step = (width - 2.0 - x_start) / (len as f64);
            current = x_start + 1.0;
            let mut index = len;
            while current > x_start && index > 0 {
                for (entry, color) in self.data.iter().zip(self.colors.iter()) {
                    c.set_source_rgb(color.r, color.g, color.b);
                    c.move_to(current + step, height - entry[index - 1] * (height - 1.0));
                    c.line_to(current, height - entry[index] * (height - 1.0));
                    c.stroke();
                }
                current += step;
                index -= 1;
            }
            // To be called in last to avoid having to restore state (rotation).
            self.draw_labels(c, 100., height);
        }
    }

    pub fn invalidate(&self) {
        if let Some(t_win) = self.area.get_window() {
            let (x, y) = self.area.translate_coordinates(&self.area, 0, 0)
                                  .expect("translate_coordinates failed");
            let rect = gdk::Rectangle { x: x, y: y,
                width: self.area.get_allocated_width(), height: self.area.get_allocated_height() };
            t_win.invalidate_rect(&rect, true);
        }
    }

    pub fn send_size_request(&self, width: Option<i32>) {
        let mut width = match width {
            Some(w) => w,
            None => {
                if let Some(parent) = self.area.get_parent() {
                    parent.get_allocation().width -
                        parent.get_margin_start() - parent.get_margin_end()
                } else {
                    eprintln!("<Graph::send_size_request> A parent is required if no width is \
                               provided...");
                    return;
                }
            }
        };
        // This condition is to avoid having a graph with a bigger width than the window.
        if let Some(top) = self.area.get_toplevel() {
            let max_width = top.get_allocation().width;
            if width > max_width {
                width = max_width;
            }
        }
        self.area.set_size_request(
            if *self.display_labels.borrow() == true {
                width - if width >= self.labels_layout_width {
                    self.labels_layout_width
                } else {
                    width
                }
            } else {
                width
            }, 200);
    }
}

pub trait Connecter {
    fn connect_to_window_events(&self);
}

impl Connecter for Rc<RefCell<Graph>> {
    fn connect_to_window_events(&self) {
        let s = self.clone();
        if let Some(parent) = self.borrow().horizontal_layout.get_toplevel() {
            // TODO: ugly way to resize drawing area, I should find a better way
            parent.connect_configure_event(move |w, _| {
                let need_diff = s.borrow().initial_diff.is_none();
                if need_diff {
                    let mut s = s.borrow_mut();
                    let parent_width = if let Some(p) = s.area.get_parent() {
                        p.get_allocation().width
                    } else {
                        0
                    };
                    s.initial_diff = Some(w.get_allocation().width - parent_width);
                }
                s.borrow().send_size_request(None);
                false
            });
        } else {
            eprintln!("This method needs to be called *after* it has been put inside a window");
        }
    }
}
