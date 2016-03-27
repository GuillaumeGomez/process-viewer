use cairo;
use gdk;
use glib::object::Cast;
use gtk::{self, BoxExt, ContainerExt, DrawingArea, ScrolledWindowExt, StateFlags};
use gtk::{ToggleButtonSignals, ToggleButtonExt, Widget, WidgetSignals, WindowExt};
use gtk::prelude::{Inhibit, WidgetExt};
use sysinfo;

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::time::Instant;

use color::Color;
use notebook::NoteBook;
use utils::RotateVec;

#[allow(dead_code)]
pub struct DisplaySysInfo {
    procs : Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram : gtk::ProgressBar,
    swap : gtk::ProgressBar,
    vertical_layout : gtk::Box,
    components: Vec<gtk::Label>,
    cpu_usage_history: Rc<RefCell<Vec<(Color, RotateVec<f64>)>>>,
}

impl DisplaySysInfo {
    pub fn new(sys1: Rc<RefCell<sysinfo::System>>, note: &mut NoteBook,
               win: &gtk::Window) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let ram = gtk::ProgressBar::new();
        let swap = gtk::ProgressBar::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let proc_scroll = gtk::ScrolledWindow::new(None, None);
        let mut components = vec!();
        let mut cpu_usage_history = vec!();
        let check_box = gtk::CheckButton::new_with_label("Graph view");

        ram.set_show_text(true);
        swap.set_show_text(true);
        vertical_layout.set_spacing(5);

        let mut total = false;
        let proc_horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let proc_vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let non_graph_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

        vertical_layout.pack_start(&gtk::Label::new(Some("Memory usage")), false, false, 15);
        vertical_layout.add(&ram);
        vertical_layout.pack_start(&gtk::Label::new(Some("Swap usage")), false, false, 15);
        vertical_layout.add(&swap);
        vertical_layout.pack_start(&gtk::Label::new(Some("Total CPU usage")), false, false, 7);
        for (i, pro) in sys1.borrow().get_processor_list().iter().enumerate() {
            if total {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];
                let l = gtk::Label::new(Some(&format!("{}", i)));
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);
                horizontal_layout.pack_start(&l, false, false, 5);
                horizontal_layout.pack_start(p, true, true, 5);
                non_graph_layout.add(&horizontal_layout);
                let c = Color::generate(i + 10);
                let l = gtk::Label::new(Some(&format!("process {}", i)));
                l.override_color(StateFlags::from_bits(0).unwrap(), &c.to_gdk());
                cpu_usage_history.push((c,
                                        RotateVec::new(iter::repeat(0f64).take(60).collect())));
                proc_vertical_layout.add(&l);
            } else {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);

                vertical_layout.add(p);
                let label = gtk::Label::new(Some("Process usage"));
                let empty = gtk::Label::new(None);
                let grid = gtk::Grid::new();
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                horizontal_layout.pack_start(&gtk::Label::new(None), true, true, 0);
                horizontal_layout.pack_start(&check_box, false, false, 0);
                grid.attach(&empty, 0, 0, 3, 1);
                grid.attach_next_to(&label, Some(&empty), gtk::PositionType::Right, 3, 1);
                grid.attach_next_to(&horizontal_layout, Some(&label),
                                    gtk::PositionType::Right, 3, 1);
                grid.set_column_homogeneous(true);
                vertical_layout.pack_start(&grid, false, false, 15);
                total = true;
            }
        }
        vertical_layout.add(&non_graph_layout);
        proc_scroll.set_min_content_width(90);
        if sys1.borrow().get_components_list().len() > 0 {
            vertical_layout.pack_start(&gtk::Label::new(Some("Components' temperature")),
                                       false, false, 15);
            for component in sys1.borrow().get_components_list() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                // TODO: add max and critical temperatures as well
                let temp = gtk::Label::new(Some(&format!("{:.1} °C", component.temperature)));
                horizontal_layout.pack_start(&gtk::Label::new(Some(&component.label)),
                                             true, false, 0);
                horizontal_layout.pack_start(&temp, true, false, 0);
                horizontal_layout.set_homogeneous(true);
                vertical_layout.add(&horizontal_layout);
                components.push(temp);
            }
        }

        let area = DrawingArea::new();
        //vertical_layout.add(&area);
        let start_time = Instant::now();
        let cpu_usage_history = Rc::new(RefCell::new(cpu_usage_history));
        let c_cpu_usage_history = cpu_usage_history.clone();
        area.connect_draw(move |w, c| {
            draw_grid(&c, w.get_allocated_width() as f64, w.get_allocated_height() as f64,
                      start_time.elapsed().as_secs(),
                      &mut c_cpu_usage_history.borrow_mut());
            Inhibit(false)
        });
        proc_horizontal_layout.add(&area);
        //proc_horizontal_layout.add(&proc_vertical_layout);
        proc_scroll.add(&proc_vertical_layout);
        proc_horizontal_layout.pack_start(&proc_scroll, false, true, 15);
        vertical_layout.add(&proc_horizontal_layout);

        scroll.add(&vertical_layout);
        let scroll : Widget = scroll.upcast();
        note.create_tab("System usage", &scroll);

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: ram,
            swap: swap,
            vertical_layout: vertical_layout,
            components: components,
            cpu_usage_history: cpu_usage_history,
        };
        tmp.update_ram_display(&sys1.borrow());
        win.add_events(gdk::EventType::Configure as i32);
        // ugly way to resize drawing area, I should find a better way
        win.connect_configure_event(move |w, _| {
            let w = w.clone().upcast::<gtk::Window>().get_size().0 - 130;
            area.set_size_request(w, 200);
            Inhibit(false)
        });
        let c_check_box = check_box.clone();
        let c_proc_horizontal_layout = proc_horizontal_layout.clone();
        let c_non_graph_layout = non_graph_layout.clone();
        c_check_box.upcast::<gtk::ToggleButton>().connect_toggled(move |c| {
            show_if_necessary(c, &c_proc_horizontal_layout, &c_non_graph_layout);
        });
        scroll.connect_show(move |_| {
            show_if_necessary(&check_box.clone().upcast::<gtk::ToggleButton>(),
                              &proc_horizontal_layout, &non_graph_layout);
        });
        tmp
    }

    pub fn update_ram_display(&mut self, sys: &sysinfo::System) {
        let total = sys.get_total_memory();
        let used = sys.get_used_memory();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        self.ram.set_text(Some(&disp));
        self.ram.set_fraction(used as f64 / total as f64);

        let total = sys.get_total_swap();
        let used = total - sys.get_used_swap();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1000, total / 1000)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1000000, total / 1000000)
        } else {
            format!("{} / {}TB", used / 1000000000, total / 1000000000)
        };

        self.swap.set_text(Some(&disp));
        self.swap.set_fraction(used as f64 / total as f64);

        for (component, label) in sys.get_components_list().iter().zip(self.components.iter()) {
            label.set_text(&format!("{:.1} °C", component.temperature));
        }
    }

    pub fn update_process_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let mut h = &mut *self.cpu_usage_history.borrow_mut();

        for (i, pro) in sys.get_processor_list().iter().enumerate() {
            v[i].set_text(Some(&format!("{:.1} %", pro.get_cpu_usage() * 100.)));
            v[i].set_show_text(true);
            v[i].set_fraction(pro.get_cpu_usage() as f64);
            if i > 0 {
                h[i - 1].1.move_start();
                *h[i - 1].1.get_mut(0).unwrap() = pro.get_cpu_usage() as f64;
            }
        }
    }
}

fn show_if_necessary(check_box: &gtk::ToggleButton, proc_horizontal_layout: &gtk::Box,
                     non_graph_layout: &gtk::Box) {
    if check_box.get_active() {
        proc_horizontal_layout.show_all();
        non_graph_layout.hide();
    } else {
        non_graph_layout.show_all();
        proc_horizontal_layout.hide();
    }
}

fn draw_grid(c: &cairo::Context, width: f64, height: f64, mut elapsed: u64,
             cpu_usage_history: &mut [(Color, RotateVec<f64>)]) {
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
    elapsed = elapsed % 5;
    let x_step = width * 5.0 / 60.0;
    let mut current = width - elapsed as f64 * (x_step / 5.0) - 1.0;
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
    let len = cpu_usage_history[0].1.len() - 1;
    let step = (width - 2.0) / (len as f64);
    current = 1.0;
    let mut index = len;
    while current > 0.0 && index > 0 {
        for &(ref color, ref entry) in cpu_usage_history.iter() {
            c.set_source_rgb(color.r, color.g, color.b);
            c.move_to(current + step, height - entry[index - 1] * height - 2.0);
            c.line_to(current, height - entry[index] * height - 2.0);
            c.stroke();
        }
        current += step;
        index -= 1;
    }
}
