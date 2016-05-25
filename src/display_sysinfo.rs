use gdk;
use glib::object::Cast;
use gtk::{self, BoxExt, ContainerExt};
use gtk::{ToggleButtonExt, Widget, WindowExt};
use gtk::prelude::{Inhibit, WidgetExt};
use sysinfo;

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use graph::Graph;
use notebook::NoteBook;
use utils::RotateVec;

fn create_header(label_text: &str, parent_layout: &gtk::Box) -> gtk::CheckButton {
    let check_box = gtk::CheckButton::new_with_label("Graph view");
    let label = gtk::Label::new(Some(label_text));
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
    parent_layout.pack_start(&grid, false, false, 15);
    check_box
}

#[allow(dead_code)]
pub struct DisplaySysInfo {
    procs: Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram: gtk::ProgressBar,
    swap: gtk::ProgressBar,
    vertical_layout: gtk::Box,
    components: Vec<gtk::Label>,
    cpu_usage_history: Rc<RefCell<Graph>>,
    ram_usage_history: Rc<RefCell<Graph>>,
}

impl DisplaySysInfo {
    pub fn new(sys1: Rc<RefCell<sysinfo::System>>, note: &mut NoteBook,
               win: &gtk::Window) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let ram = gtk::ProgressBar::new();
        let swap = gtk::ProgressBar::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let mut components = vec!();
        let mut cpu_usage_history = Graph::new();
        let mut ram_usage_history = Graph::new();
        let mut check_box = None;

        ram.set_show_text(true);
        swap.set_show_text(true);
        vertical_layout.set_spacing(5);

        let mut total = false;
        let non_graph_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let check_box2 = create_header("Memory usage", &vertical_layout);
        vertical_layout.add(&ram);
        ram_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()), "RAM");
        ram_usage_history.attach_to(&vertical_layout);

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
                cpu_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                                       &format!("process {}", i));
            } else {
                procs.push(gtk::ProgressBar::new());
                let p : &gtk::ProgressBar = &procs[i];

                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_show_text(true);
                p.set_fraction(pro.get_cpu_usage() as f64);

                vertical_layout.add(p);
                check_box = Some(create_header("Process usage", &vertical_layout));
                total = true;
            }
        }
        vertical_layout.add(&non_graph_layout);

        let area = cpu_usage_history.area.clone();
        cpu_usage_history.attach_to(&vertical_layout);
        let cpu_usage_history = Rc::new(RefCell::new(cpu_usage_history));
        let c_cpu_usage_history = cpu_usage_history.clone();
        area.connect_draw(move |w, c| {
            c_cpu_usage_history.borrow()
                               .draw(&c,
                                     w.get_allocated_width() as f64,
                                     w.get_allocated_height() as f64);
            Inhibit(false)
        });
        let area = ram_usage_history.area.clone();
        let ram_usage_history = Rc::new(RefCell::new(ram_usage_history));
        let c_ram_usage_history = ram_usage_history.clone();
        area.connect_draw(move |w, c| {
            c_ram_usage_history.borrow()
                               .draw(&c,
                                     w.get_allocated_width() as f64,
                                     w.get_allocated_height() as f64);
            Inhibit(false)
        });

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

        scroll.add(&vertical_layout);
        let scroll : Widget = scroll.upcast();
        note.create_tab("System usage", &scroll);

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: ram.clone(),
            swap: swap,
            vertical_layout: vertical_layout,
            components: components,
            cpu_usage_history: cpu_usage_history,
            ram_usage_history: ram_usage_history,
        };
        tmp.update_ram_display(&sys1.borrow(), false);
        win.add_events(gdk::EventType::Configure as i32);
        // ugly way to resize drawing area, I should find a better way
        win.connect_configure_event(move |w, _| {
            let w = w.clone().upcast::<gtk::Window>().get_size().0 - 130;
            area.set_size_request(w, 200);
            false
        });
        if let Some(check_box) = check_box {
            let c_check_box = check_box.clone();
            let c_non_graph_layout = non_graph_layout.clone();
            let c_cpu_usage_history = tmp.cpu_usage_history.clone();
            c_check_box.upcast::<gtk::ToggleButton>().connect_toggled(move |c| {
                show_if_necessary(c, &c_cpu_usage_history.borrow(), &c_non_graph_layout);
            });
            let c_cpu_usage_history = tmp.cpu_usage_history.clone();
            scroll.connect_show(move |_| {
                show_if_necessary(&check_box.clone().upcast::<gtk::ToggleButton>(),
                                  &c_cpu_usage_history.borrow(),
                                  &non_graph_layout);
            });
        }
        let c_check_box = check_box2.clone();
        let c_non_graph_layout = ram.clone();
        let c_ram_usage_history = tmp.ram_usage_history.clone();
        c_check_box.upcast::<gtk::ToggleButton>().connect_toggled(move |c| {
            show_if_necessary(c, &c_ram_usage_history.borrow(), &ram);
        });
        /*let c_cpu_usage_history = tmp.cpu_usage_history.clone();
        scroll.connect_show(move |_| {
            show_if_necessary(&check_box2.clone().upcast::<gtk::ToggleButton>(),
                              &c_cpu_usage_history.borrow(),
                              &non_graph_layout);
        });*/
        tmp
    }

    pub fn update_ram_display(&mut self, sys: &sysinfo::System, display_fahrenheit: bool) {
        let total = sys.get_total_memory();
        let used = sys.get_used_memory();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1024, total / 1024)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1048576, total / 1048576)
        } else {
            format!("{} / {}TB", used / 1073741824, total / 1073741824)
        };

        self.ram.set_text(Some(&disp));
        self.ram.set_fraction(used as f64 / total as f64);

        let total = sys.get_total_swap();
        let used = total - sys.get_used_swap();
        let disp = if total < 100000 {
            format!("{} / {}KB", used, total)
        } else if total < 10000000 {
            format!("{} / {}MB", used / 1024, total / 1024)
        } else if total < 10000000000 {
            format!("{} / {}GB", used / 1048576, total / 1048576)
        } else {
            format!("{} / {}TB", used / 1073741824, total / 1073741824)
        };

        self.swap.set_text(Some(&disp));

        let mut fraction = used as f64 / total as f64;
        if fraction.is_nan() {
            fraction = 0 as f64;
        }
        self.swap.set_fraction(fraction);

        for (component, label) in sys.get_components_list().iter().zip(self.components.iter()) {
            if display_fahrenheit {
                label.set_text(&format!("{:.1} °F", component.temperature * 1.8 + 32.));
            } else {
                label.set_text(&format!("{:.1} °C", component.temperature));
            }
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
                h.data[i - 1].move_start();
                *h.data[i - 1].get_mut(0).unwrap() = pro.get_cpu_usage() as f64;
            }
        }
    }
}

fn show_if_necessary<T: WidgetExt>(check_box: &gtk::ToggleButton, proc_horizontal_layout: &Graph,
                                   non_graph_layout: &T) {
    if check_box.get_active() {
        proc_horizontal_layout.show_all();
        non_graph_layout.hide();
    } else {
        non_graph_layout.show_all();
        proc_horizontal_layout.hide();
    }
}
