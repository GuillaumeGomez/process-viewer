use gdk::RGBA;
use graphene::Rect;
use gsk::RoundedRect;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{cairo, gdk, glib, graphene, gsk};
use std::cell::{Cell, RefCell};

use crate::color::Color;
use crate::utils::RotateVec;

const LEFT_WIDTH: f32 = 31.;
const HEIGHT: f32 = 200.;

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<GraphWidgetImp>)
         @extends gtk::Widget,
         @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl GraphWidget {
    /// If `max` is `None`, the graph will expect values between 0 and 1.
    ///
    /// If `keep_max` is set to `true`, then this value will never go down, meaning that graphs
    /// won't rescale down. It is not taken into account if `max` is `None`.
    pub fn new(max: Option<f32>, keep_max: bool) -> Self {
        let widget = glib::Object::new::<Self>();
        widget.imp().graph.borrow().set_max(max);
        widget.imp().graph.borrow().set_keep_max(keep_max);
        widget.imp().graph.borrow().set_hexpand(true);
        widget
    }

    pub fn set_labels_callback(&self, labels_callback: Option<Box<dyn Fn(f32) -> [String; 4]>>) {
        self.imp()
            .graph
            .borrow()
            .set_labels_callback(labels_callback);
    }

    pub fn push(&self, d: RotateVec<f32>, s: &str, override_color: Option<usize>) {
        let color = self.imp().graph.borrow().push(d, override_color);

        let layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let square = SquareWidget::new(color);
        square.set_margin_end(5);
        let l = gtk::Label::new(Some(s));
        layout.append(&square);
        layout.append(&l);
        self.imp().labels.borrow().insert(&layout, -1);
    }

    pub fn data<F: FnMut(&mut RotateVec<f32>)>(&self, pos: usize, f: F) {
        self.imp().graph.borrow().data(pos, f);
    }

    pub fn set_overhead(&self, overhead: Option<f32>) {
        self.imp().graph.borrow().set_overhead(overhead);
    }

    pub fn set_minimum(&self, minimum: Option<f32>) {
        self.imp().graph.borrow().set_minimum(minimum);
    }

    pub fn set_display_labels(&self, display_labels: bool) {
        self.imp().display_labels.set(display_labels);
        if display_labels {
            self.imp().labels.borrow().show();
        } else {
            self.imp().labels.borrow().hide();
        }
    }
}

pub struct GraphWidgetImp {
    graph: RefCell<GraphInnerWidget>,
    labels: RefCell<gtk::FlowBox>,
    display_labels: Cell<bool>,
}

impl Default for GraphWidgetImp {
    fn default() -> Self {
        Self {
            graph: RefCell::new(GraphInnerWidget::new()),
            labels: RefCell::new(gtk::FlowBox::new()),
            display_labels: Cell::new(true),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for GraphWidgetImp {
    const NAME: &'static str = "GraphWidgetImp";
    type Type = GraphWidget;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<gtk::BoxLayout>();
    }
}

impl WidgetImpl for GraphWidgetImp {
    fn show(&self) {
        self.parent_show();
        if !self.display_labels.get() {
            self.labels.borrow().hide();
        }
    }
}

impl ObjectImpl for GraphWidgetImp {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        let layout = obj
            .layout_manager()
            .and_downcast::<gtk::BoxLayout>()
            .unwrap();
        layout.set_orientation(gtk::Orientation::Vertical);
        layout.set_spacing(5);
        self.labels.borrow().set_homogeneous(true);
        self.graph.borrow().set_parent(&*obj);
        self.labels.borrow().set_parent(&*obj);
    }

    fn dispose(&self) {
        // Child widgets need to be manually unparented in `dispose()`.
        self.graph.borrow().unparent();
        self.labels.borrow().unparent();
    }
}

glib::wrapper! {
    pub struct GraphInnerWidget(ObjectSubclass<GraphPainter>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl GraphInnerWidget {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_max(&self, max: Option<f32>) {
        self.imp().max.set(max);
    }

    pub fn set_keep_max(&self, keep_max: bool) {
        self.imp().keep_max.set(keep_max);
    }

    pub fn set_minimum(&self, minimum: Option<f32>) {
        self.imp().minimum.set(minimum);
    }

    pub fn set_overhead(&self, overhead: Option<f32>) {
        if let Some(o) = overhead {
            assert!(o >= 0.);
        }
        self.imp().overhead.set(overhead);
    }

    pub fn set_labels_callback(&self, labels_callback: Option<Box<dyn Fn(f32) -> [String; 4]>>) {
        *self.imp().labels_callback.borrow_mut() = labels_callback;
    }

    pub fn attach_to(&self, to: &gtk::Box) {
        to.append(self);
    }

    pub fn push(&self, d: RotateVec<f32>, override_color: Option<usize>) -> Color {
        let c = if let Some(over) = override_color {
            Color::generate(over)
        } else {
            Color::generate(self.imp().data.borrow().len() + 11)
        };
        self.imp().colors.borrow_mut().push(c);
        self.imp().data.borrow_mut().push(d);
        c
    }

    pub fn data<F: FnMut(&mut RotateVec<f32>)>(&self, pos: usize, mut f: F) {
        f(&mut self.imp().data.borrow_mut()[pos]);
        self.queue_draw();
    }
}

pub struct GraphPainter {
    colors: RefCell<Vec<Color>>,
    data: RefCell<Vec<RotateVec<f32>>>,
    max: Cell<Option<f32>>,
    keep_max: Cell<bool>,
    /// `minimum` is used only if `max` is set: it'll be the minimum that the `max` value will
    /// be able to go down.
    minimum: Cell<Option<f32>>,
    // In %, from 0 to whatever
    overhead: Cell<Option<f32>>,
    #[allow(clippy::type_complexity)]
    labels_callback: RefCell<Option<Box<dyn Fn(f32) -> [String; 4]>>>,
}

impl Default for GraphPainter {
    fn default() -> Self {
        Self {
            colors: RefCell::new(Vec::new()),
            data: RefCell::new(Vec::new()),
            max: Cell::new(None),
            keep_max: Cell::new(false),
            minimum: Cell::new(None),
            overhead: Cell::new(None),
            // need_label_update: Cell::new(true),
            labels_callback: RefCell::new(None),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for GraphPainter {
    const NAME: &'static str = "GraphPainter";
    type Type = GraphInnerWidget;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("graph_widget");
    }
}

impl GraphPainter {
    fn draw_labels(&self, widget: &GraphInnerWidget, c: &cairo::Context, max: f32) {
        if let Some(ref call) = *self.labels_callback.borrow() {
            let entries = call(max);
            let font_size = 8.;
            let left_width = LEFT_WIDTH as f64;
            let height = HEIGHT as f64;

            let color = widget.style_context().color();

            c.set_source_rgba(
                color.red() as _,
                color.green() as _,
                color.blue() as _,
                color.alpha() as _,
            );
            c.set_font_size(font_size);

            c.move_to(left_width - 4. - entries[0].len() as f64 * 4., font_size);
            let _ = c.show_text(entries[0].as_str());

            c.move_to(left_width - 4. - entries[1].len() as f64 * 4., height / 2.);
            let _ = c.show_text(entries[1].as_str());

            c.move_to(left_width - 4. - entries[2].len() as f64 * 4., height - 2.);
            let _ = c.show_text(entries[2].as_str());

            c.move_to(
                font_size + 1.,
                height / 2. + 4. * (entries[3].len() / 2) as f64,
            );
            c.rotate(-::std::f64::consts::FRAC_PI_2);
            let _ = c.show_text(entries[3].as_str());

            // *Better* code that should be used but crashes at `sub_snap.to_node()`.
            // let ctx = widget.create_pango_context();
            // let font_description = ctx.font_description().unwrap();
            // font_description.set_size (font_size * pango::SCALE);
            // ctx.set_font_description(&font_description);

            // let entries = call(max);

            // let layout = pango::Layout::new(&ctx);
            // layout.set_text(&entries[0]);
            // snapshot.render_layout(&ctx, LEFT_WIDTH - 4. - (entries[0].len() * 4) as _, font_size);

            // let layout = pango::Layout::new(&ctx);
            // layout.set_text(&entries[1]);
            // snapshot.render_layout(&ctx, LEFT_WIDTH - 4. - (entries[1].len() * 4) as _, HEIGHT / 2.);

            // let layout = pango::Layout::new(&ctx);
            // layout.set_text(&entries[2]);
            // snapshot.render_layout(&ctx, LEFT_WIDTH - 4. - (entries[2].len() * 4) as _, HEIGHT - 2.);

            // let sub_snap = gtk::Snapshot::new();
            // let layout = pango::Layout::new(&ctx);
            // layout.set_text(&entries[3]);
            // sub_snap.render_layout(&ctx, font_size - 1., HEIGHT / 2. + 4. * (entries[3].len() / 2) as _);
            // sub_snap.rotate(-90.);
            // snapshot.append_node(sub_snap.to_node().unwrap());
        }
    }
}

impl ObjectImpl for GraphPainter {}

impl WidgetImpl for GraphPainter {
    fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        if orientation == gtk::Orientation::Vertical {
            // Minimum height is HEIGHT.
            (HEIGHT as i32, HEIGHT as i32, -1, -1)
        } else {
            // Minimum width is 50.
            (50, 50, -1, -1)
        }
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let widget = self.obj();
        let x_start = if self.labels_callback.borrow().is_some() {
            LEFT_WIDTH
        } else {
            0.
        };
        let width = widget.width() as f32 - x_start - 2.;

        // to limit line "fuzziness"
        #[inline]
        fn rounder(x: f32) -> f32 {
            let fract = x.fract();
            if fract < 0.5 {
                x.trunc() + 0.5
            } else {
                x.trunc() + 1.5
            }
        }

        snapshot.append_border(
            &RoundedRect::from_rect(Rect::new(x_start, 0., width + 2., HEIGHT), 0.),
            &[1., 1., 1., 1.],
            &[RGBA::WHITE, RGBA::WHITE, RGBA::WHITE, RGBA::WHITE],
        );
        snapshot.append_color(
            &RGBA::BLACK,
            &Rect::new(x_start + 1., 1., width, HEIGHT - 2.),
        );
        let color = RGBA::new(0.5, 0.5, 0.5, 1.);

        // We always draw 10 lines (12 if we count the borders).
        let x_step = width / 12.;
        let mut current = width - width / 12. + x_start + 1.;
        if x_step < 0.1 {
            return;
        }

        while current > x_start {
            snapshot.append_color(&color, &Rect::new(current, 1., 1., HEIGHT - 2.));
            current -= x_step;
        }
        let step = HEIGHT / 10.;
        current = step - 1.0;
        while current < HEIGHT - 1. {
            let y = rounder(current) - 1.;
            snapshot.append_color(&color, &Rect::new(x_start + 1., y, width, 1.));
            current += step;
        }

        if let Some(self_max) = self.max.get() {
            let data = self.data.borrow();
            let mut max = if self.keep_max.get() { self_max } else { 1. };
            if !data.is_empty() && !data[0].is_empty() {
                let len = data[0].len() - 1;
                for x in 0..len {
                    for entry in &*self.data.borrow() {
                        if entry[x] > max {
                            max = entry[x];
                        }
                    }
                }
                if let Some(min) = self.minimum.get() {
                    if min > max {
                        max = min;
                    }
                } else if let Some(over) = self.overhead.get() {
                    max = max + max * over / 100.;
                }
                let step = width / len as f32;
                let c =
                    snapshot.append_cairo(&Rect::new(0., 1., width + 1. + x_start, HEIGHT - 2.));
                current = x_start + 2.0;
                let colors = self.colors.borrow();
                let mut index = len;
                while current > 0. && index > 0 {
                    for (entry, color) in data.iter().zip(colors.iter()) {
                        c.set_source_rgb(color.r as _, color.g as _, color.b as _);
                        c.move_to(
                            (current + step) as f64,
                            (HEIGHT - entry[index - 1] / max * (HEIGHT - 1.0)) as f64,
                        );
                        c.line_to(
                            current as f64,
                            (HEIGHT - entry[index] / max * (HEIGHT - 1.0)) as f64,
                        );
                        let _ = c.stroke();
                    }
                    current += step;
                    index -= 1;
                }
                if max > self_max || !self.keep_max.get() {
                    self.max.set(Some(max));
                }
            }
            let c = snapshot.append_cairo(&Rect::new(1., 0., x_start, HEIGHT - 2.));
            self.draw_labels(&widget, &c, max);
        } else {
            let data = self.data.borrow();
            if !data.is_empty() && !data[0].is_empty() {
                let c =
                    snapshot.append_cairo(&Rect::new(0., 1., width + 1. + x_start, HEIGHT - 2.));
                let len = data[0].len() - 1;
                let step = width / (len as f32);
                current = x_start + 2.0;
                let mut index = len;
                let colors = self.colors.borrow();
                while current > 0. && index > 0 {
                    for (entry, color) in data.iter().zip(colors.iter()) {
                        c.set_source_rgb(color.r as _, color.g as _, color.b as _);
                        c.move_to(
                            (current + step) as f64,
                            (HEIGHT - entry[index - 1] * (HEIGHT - 1.0)) as f64,
                        );
                        c.line_to(
                            current as f64,
                            (HEIGHT - entry[index] * (HEIGHT - 1.0)) as f64,
                        );
                        let _ = c.stroke();
                    }
                    current += step;
                    index -= 1;
                }
            }
            let c = snapshot.append_cairo(&Rect::new(1., 0., x_start, HEIGHT));
            // To be called in last to avoid having to restore state (rotation).
            self.draw_labels(&widget, &c, 100.);
        }
    }
}

glib::wrapper! {
    pub struct SquareWidget(ObjectSubclass<SquarePainter>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SquareWidget {
    pub fn new(color: Color) -> Self {
        let widget = glib::Object::new::<Self>();
        widget.imp().color.set(color);
        widget
    }
}

pub struct SquarePainter {
    color: Cell<Color>,
}

impl Default for SquarePainter {
    fn default() -> Self {
        Self {
            color: Cell::new(Color::new(0, 0, 0)),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SquarePainter {
    const NAME: &'static str = "SquarePainter";
    type Type = SquareWidget;
    type ParentType = gtk::Widget;

    fn class_init(_klass: &mut Self::Class) {}
}

impl ObjectImpl for SquarePainter {}

impl WidgetImpl for SquarePainter {
    fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        // Minimum width is 20.
        (20, 20, -1, -1)
    }

    fn request_mode(&self) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::WidthForHeight
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let widget = self.obj();
        let width = widget.width() as f32;
        let height = widget.height() as f32;
        let margin = 2.; // only to limit the height

        snapshot.append_border(
            &RoundedRect::from_rect(Rect::new(0., margin, width, height - margin * 2.), 0.),
            &[1., 1., 1., 1.],
            &[RGBA::WHITE, RGBA::WHITE, RGBA::WHITE, RGBA::WHITE],
        );
        let color = self.color.get();
        snapshot.append_color(
            &RGBA::new(color.red(), color.green(), color.blue(), 1.),
            &Rect::new(1., margin + 1., width - 2., height - 2. - margin * 2.),
        );
    }
}
