use cairo;
use gtk::{self, BoxExt, ContainerExt, DrawingArea, ScrolledWindowExt, StateFlags, WidgetExt};
use std::cell::RefCell;
use gdk::{self, WindowExt};

use std::time::Instant;

use color::Color;
use utils::RotateVec;

pub struct Graph {
    elapsed: Instant,
    colors: Vec<Color>,
    pub data: Vec<RotateVec<f64>>,
    vertical_layout: gtk::Box,
    scroll_layout: gtk::ScrolledWindow,
    horizontal_layout: gtk::Box,
    pub area: DrawingArea,
    max: Option<RefCell<f64>>,
}

impl Graph {
    // If `max` is `None`, the graph will expect values between 0 and 1.
    pub fn new(max: Option<f64>) -> Graph {
        let g = Graph {
            elapsed: Instant::now(),
            colors: vec!(),
            data: vec!(),
            vertical_layout: gtk::Box::new(gtk::Orientation::Vertical, 0),
            scroll_layout: gtk::ScrolledWindow::new(None, None),
            horizontal_layout: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            area: DrawingArea::new(),
            max: if let Some(max) = max { Some(RefCell::new(max)) } else { None },
        };
        g.scroll_layout.set_min_content_width(90);
        g.scroll_layout.add(&g.vertical_layout);
        g.horizontal_layout.add(&g.area);
        g.horizontal_layout.pack_start(&g.scroll_layout, false, true, 15);
        g.horizontal_layout.set_margin_left(5);
        g
    }

    pub fn hide(&self) {
        self.horizontal_layout.hide();
    }

    pub fn show_all(&self) {
        self.horizontal_layout.show_all();
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
        l.override_color(StateFlags::from_bits(0).unwrap(), &c.to_gdk());
        self.vertical_layout.add(&l);
        self.colors.push(c);
        self.data.push(d);
    }

    pub fn draw(&self, c: &cairo::Context, width: f64, height: f64) {
        c.set_source_rgb(0.8, 0.8, 0.8);
        c.rectangle(2.0, 1.0, width - 2.0, height - 2.0);
        c.fill();
        c.set_source_rgb(0.0, 0.0, 0.0);
        c.set_line_width(1.0);
        c.move_to(1.0, 0.0);
        c.line_to(1.0, height);
        c.move_to(width, 0.0);
        c.line_to(width, height);
        c.move_to(1.0, 0.0);
        c.line_to(width, 0.0);
        c.move_to(1.0, height);
        c.line_to(width, height);
        // For now it's always 60 seconds.
        let time = 60.;

        let elapsed = self.elapsed.elapsed().as_secs() % 5;
        let x_step = (width - 2.0) * 5.0 / (time as f64);
        let mut current = width - elapsed as f64 * (x_step / 5.0) - 1.0;
        if x_step < 0.1 {
            c.stroke();
            return;
        }

        while current > 0.0 {
            c.move_to(current, 0.0);
            c.line_to(current, height);
            current -= x_step;
        }
        let step = height / 10.0;
        current = step - 1.0;
        while current < height {
            c.move_to(1.0, current);
            c.line_to(width - 1.0, current);
            current += step;
        }
        c.stroke();

        if self.max.is_some() {
            let mut max = 1.;
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
                let step = (width - 2.0) / len as f64;
                current = 1.0;
                let mut index = len;
                while current > 0.0 && index > 0 {
                    for (entry, color) in self.data.iter().zip(self.colors.iter()) {
                        c.set_source_rgb(color.r, color.g, color.b);
                        c.move_to(current + step, height - entry[index - 1] / max * height - 2.0);
                        c.line_to(current, height - entry[index] / max * height - 2.0);
                        c.stroke();
                    }
                    current += step;
                    index -= 1;
                }
            }
            if let Some(ref cmax) = self.max {
                *cmax.borrow_mut() = max;
            }
        } else if !self.data.is_empty() && !self.data[0].is_empty() {
            let len = self.data[0].len() - 1;
            let step = (width - 2.0) / (len as f64);
            current = 1.0;
            let mut index = len;
            while current > 0.0 && index > 0 {
                for (entry, color) in self.data.iter().zip(self.colors.iter()) {
                    c.set_source_rgb(color.r, color.g, color.b);
                    c.move_to(current + step, height - entry[index - 1] * height - 2.0);
                    c.line_to(current, height - entry[index] * height - 2.0);
                    c.stroke();
                }
                current += step;
                index -= 1;
            }
        }
    }

    pub fn invalidate(&self) {
        if let Some(t_win) = self.area.get_window() {
            let (x, y) = self.area.translate_coordinates(&self.area, 0, 0).unwrap();
            let rect = gdk::Rectangle { x: x, y: y,
                width: self.area.get_allocated_width(), height: self.area.get_allocated_height() };
            t_win.invalidate_rect(&rect, true);
        }
    }
}
