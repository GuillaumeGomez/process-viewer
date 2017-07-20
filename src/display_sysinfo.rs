use gdk;
use glib::object::Cast;
use glib::translate::ToGlib;
use gtk::{self, BoxExt, ContainerExt};
use gtk::{ToggleButtonExt, Widget, WindowExt};
use gtk::prelude::{Inhibit, WidgetExt};
use sysinfo::{self, NetworkExt, ProcessorExt, SystemExt};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use graph::Graph;
use notebook::NoteBook;
use utils::RotateVec;

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

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

fn create_progress_bar(non_graph_layout: &gtk::Grid, line: i32, label: &str,
                       text: &str) -> gtk::ProgressBar {
    let p = gtk::ProgressBar::new();
    let l = gtk::Label::new(Some(label));

    p.set_text(Some(text));
    p.set_show_text(true);
    non_graph_layout.attach(&l, 0, line, 1, 1);
    non_graph_layout.attach(&p, 1, line, 11, 1);
    p
}

fn format_number(mut nb: u64) -> String {
    if nb < 1000 {
        return format!("{} B", nb);
    }
    nb /= 1024;
    if nb < 100000 {
        format!("{} kB", nb / 1024)
    } else if nb < 10000000 {
        format!("{} MB", nb / 1024)
    } else if nb < 10000000000 {
        format!("{} GB", nb / 1_048_576)
    } else {
        format!("{} TB", nb / 1_073_741_824)
    }
}

#[allow(dead_code)]
pub struct DisplaySysInfo {
    procs: Rc<RefCell<Vec<gtk::ProgressBar>>>,
    ram: gtk::ProgressBar,
    swap: gtk::ProgressBar,
    vertical_layout: gtk::Box,
    // network in usage
    in_usage: gtk::Label,
    // network out usage
    out_usage: gtk::Label,
    components: Vec<gtk::Label>,
    cpu_usage_history: Rc<RefCell<Graph>>,
    // 0 = RAM
    // 1 = SWAP
    ram_usage_history: Rc<RefCell<Graph>>,
    temperature_usage_history: Rc<RefCell<Graph>>,
    network_history: Rc<RefCell<Graph>>,
    pub ram_check_box: gtk::CheckButton,
    pub swap_check_box: gtk::CheckButton,
    pub temperature_check_box: Option<gtk::CheckButton>,
}

impl DisplaySysInfo {
    pub fn new(sys1: Rc<RefCell<sysinfo::System>>, note: &mut NoteBook,
               win: &gtk::Window) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let scroll = gtk::ScrolledWindow::new(None, None);
        let mut components = vec!();
        let mut cpu_usage_history = Graph::new();
        let mut ram_usage_history = Graph::new();
        let mut temperature_usage_history = Graph::new();
        let mut network_history = Graph::new();
        let mut check_box3 = None;

        vertical_layout.set_spacing(5);

        let non_graph_layout = gtk::Grid::new();
        non_graph_layout.set_column_homogeneous(true);
        non_graph_layout.set_margin_right(5);
        let non_graph_layout2 = gtk::Grid::new();
        non_graph_layout2.set_column_homogeneous(true);
        non_graph_layout2.set_margin_right(5);
        let non_graph_layout3 = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let non_graph_layout4 = gtk::Box::new(gtk::Orientation::Vertical, 0);


        vertical_layout.pack_start(&gtk::Label::new(Some("Total CPU usage")), false, false, 7);
        procs.push(gtk::ProgressBar::new());
        {
            let p: &gtk::ProgressBar = &procs[0];
            let s = sys1.borrow();

            p.set_margin_right(5);
            p.set_margin_left(5);
            p.set_show_text(true);
            let processor_list = s.get_processor_list();
            if !processor_list.is_empty() {
                let pro = &processor_list[0];
                p.set_text(format!("{:.2} %", pro.get_cpu_usage() * 100.).as_str());
                p.set_fraction(pro.get_cpu_usage() as f64);
            } else {
                p.set_text(Some("0.0 %"));
                p.set_fraction(0.);
            }
            vertical_layout.add(p);
        }


        //
        // PROCESS PART
        //
        let check_box = create_header("Process usage", &vertical_layout);
        for (i, pro) in sys1.borrow().get_processor_list().iter().skip(1).enumerate() {
            let i = i + 1;
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[i];
            let l = gtk::Label::new(format!("{}", i).as_str());

            p.set_text(format!("{:.2} %", pro.get_cpu_usage() * 100.).as_str());
            p.set_show_text(true);
            p.set_fraction(pro.get_cpu_usage() as f64);
            non_graph_layout.attach(&l, 0, i as i32 - 1, 1, 1);
            non_graph_layout.attach(p, 1, i as i32 - 1, 11, 1);
            cpu_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                                   &format!("process {}", i), None);
        }
        vertical_layout.add(&non_graph_layout);
        cpu_usage_history.attach_to(&vertical_layout);


        //
        // MEMORY PART
        //
        let check_box2 = create_header("Memory usage", &vertical_layout);
        let ram = create_progress_bar(&non_graph_layout2, 0, "RAM", "");
        let swap = create_progress_bar(&non_graph_layout2, 1, "Swap", "");
        vertical_layout.pack_start(&non_graph_layout2, false, false, 15);
        //vertical_layout.add(&non_graph_layout2);
        ram_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                               "RAM", Some(4));
        ram_usage_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                               "Swap", Some(2));
        ram_usage_history.attach_to(&vertical_layout);


        //
        // TEMPERATURES PART
        //
        if sys1.borrow().get_components_list().len() > 0 {
            check_box3 = Some(create_header("Components' temperature", &vertical_layout));
            for component in sys1.borrow().get_components_list() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                // TODO: add max and critical temperatures as well
                let temp = gtk::Label::new(format!("{:.1} °C", component.temperature).as_str());
                horizontal_layout.pack_start(&gtk::Label::new(component.label.as_str()),
                                             true, false, 0);
                horizontal_layout.pack_start(&temp, true, false, 0);
                horizontal_layout.set_homogeneous(true);
                non_graph_layout3.add(&horizontal_layout);
                components.push(temp);
                temperature_usage_history.push(RotateVec::new(iter::repeat(0f64)
                                                                   .take(61)
                                                                   .collect()),
                                               &component.label, None);
            }
            vertical_layout.add(&non_graph_layout3);
            temperature_usage_history.attach_to(&vertical_layout);
        }


        //
        // NETWORK PART
        //
        let check_box4 = create_header("Network usage", &vertical_layout);
        // input data
        let in_usage = gtk::Label::new(format_number(0).as_str());
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        horizontal_layout.pack_start(&gtk::Label::new("Input data"), true, false, 0);
        horizontal_layout.pack_start(&in_usage, true, false, 0);
        horizontal_layout.set_homogeneous(true);
        non_graph_layout4.add(&horizontal_layout);
        network_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                             "Input data", None);
        // output data
        let out_usage = gtk::Label::new(format_number(0).as_str());
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        horizontal_layout.pack_start(&gtk::Label::new("Output data"), true, false, 0);
        horizontal_layout.pack_start(&out_usage, true, false, 0);
        horizontal_layout.set_homogeneous(true);
        non_graph_layout4.add(&horizontal_layout);
        network_history.push(RotateVec::new(iter::repeat(0f64).take(61).collect()),
                             "Output data", None);
        vertical_layout.add(&non_graph_layout4);
        network_history.attach_to(&vertical_layout);


        //
        // Putting everyting into places now.
        //
        let area = cpu_usage_history.area.clone();
        let area2 = ram_usage_history.area.clone();
        let area3 = temperature_usage_history.area.clone();
        let area4 = network_history.area.clone();
        let cpu_usage_history = connect_graph(cpu_usage_history);
        let ram_usage_history = connect_graph(ram_usage_history);
        let temperature_usage_history = connect_graph(temperature_usage_history);
        let network_history = connect_graph(network_history);

        scroll.add(&vertical_layout);
        let scroll : Widget = scroll.upcast();
        note.create_tab("System usage", &scroll);

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: ram.clone(),
            swap: swap.clone(),
            out_usage: out_usage.clone(),
            in_usage: in_usage.clone(),
            vertical_layout: vertical_layout,
            components: components,
            cpu_usage_history: cpu_usage_history.clone(),
            ram_usage_history: ram_usage_history.clone(),
            ram_check_box: check_box.clone(),
            swap_check_box: check_box2.clone(),
            temperature_usage_history: temperature_usage_history.clone(),
            temperature_check_box: check_box3.clone(),
            network_history: network_history.clone(),
        };
        tmp.update_ram_display(&sys1.borrow(), false);

        win.add_events(gdk::EventType::Configure.to_glib() as i32);
        // ugly way to resize drawing area, I should find a better way
        win.connect_configure_event(move |w, _| {
            let w = w.clone().upcast::<gtk::Window>().get_size().0 - 130;
            area.set_size_request(w, 200);
            area2.set_size_request(w, 200);
            area3.set_size_request(w, 200);
            area4.set_size_request(w, 200);
            false
        });

        check_box.clone().upcast::<gtk::ToggleButton>()
                 .connect_toggled(clone!(non_graph_layout, cpu_usage_history => move |c| {
            show_if_necessary(c, &cpu_usage_history.borrow(), &non_graph_layout);
        }));
        check_box2.clone().upcast::<gtk::ToggleButton>()
                  .connect_toggled(clone!(non_graph_layout2, ram_usage_history => move |c| {
            show_if_necessary(c, &ram_usage_history.borrow(), &non_graph_layout2);
        }));
        if let Some(ref check_box3) = check_box3 {
            check_box3.clone().upcast::<gtk::ToggleButton>()
                 .connect_toggled(clone!(non_graph_layout3, temperature_usage_history => move |c| {
                show_if_necessary(c, &temperature_usage_history.borrow(), &non_graph_layout3);
            }));
        }
        check_box4.clone().upcast::<gtk::ToggleButton>()
                  .connect_toggled(clone!(non_graph_layout4, network_history => move |c| {
            show_if_necessary(c, &network_history.borrow(), &non_graph_layout4);
        }));

        scroll.connect_show(clone!(cpu_usage_history, ram_usage_history => move |_| {
            show_if_necessary(&check_box.clone().upcast::<gtk::ToggleButton>(),
                              &cpu_usage_history.borrow(), &non_graph_layout);
            show_if_necessary(&check_box2.clone().upcast::<gtk::ToggleButton>(),
                              &ram_usage_history.borrow(), &non_graph_layout2);
            if let Some(ref check_box3) = check_box3 {
                show_if_necessary(&check_box3.clone().upcast::<gtk::ToggleButton>(),
                                  &temperature_usage_history.borrow(), &non_graph_layout3);
            }
            show_if_necessary(&check_box4.clone().upcast::<gtk::ToggleButton>(),
                              &network_history.borrow(), &non_graph_layout4);
        }));
        tmp
    }

    pub fn update_ram_display(&mut self, sys: &sysinfo::System, display_fahrenheit: bool) {
        let disp = |total, used| {
            if total < 100000 {
                format!("{} / {} kB", used, total)
            } else if total < 10000000 {
                format!("{:.2} / {} MB", used as f64 / 1024f64, total / 1024)
            } else if total < 10000000000 {
                format!("{:.2} / {} GB", used as f64 / 1048576f64, total / 1048576)
            } else {
                format!("{:.2} / {} TB", used as f64 / 1073741824f64, total / 1073741824)
            }
        };

        let total = sys.get_total_memory();
        let used = sys.get_used_memory();
        self.ram.set_text(disp(total, used).as_str());
        if total != 0 {
            self.ram.set_fraction(used as f64 / total as f64);
        } else {
            self.ram.set_fraction(0.0);
        }
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[0].move_start();
            if let Some(p) = r.data[0].get_mut(0) {
                *p = used as f64 / total as f64;
            }
        }

        let total = sys.get_total_swap();
        let used = sys.get_used_swap();
        self.swap.set_text(disp(total, used).as_str());

        let mut fraction = if total != 0 { used as f64 / total as f64 } else { 0f64 };
        if fraction.is_nan() {
            fraction = 0f64;
        }
        self.swap.set_fraction(fraction);
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[1].move_start();
            if let Some(p) = r.data[1].get_mut(0) {
                *p = used as f64 / total as f64;
            }
        }

        let mut t = self.temperature_usage_history.borrow_mut();
        for (pos, (component, label)) in sys.get_components_list()
                                            .iter().zip(self.components.iter()).enumerate() {
            t.data[pos].move_start();
            if let Some(critical) = component.critical {
                if let Some(t) = t.data[pos].get_mut(0) {
                    *t = component.temperature as f64 / critical as f64;
                }
            } else {
                if let Some(t) = t.data[pos].get_mut(0) {
                    *t = component.temperature as f64 / component.max as f64;
                }
            }
            if display_fahrenheit {
                label.set_text(&format!("{:.1} °F", component.temperature * 1.8 + 32.));
            } else {
                label.set_text(&format!("{:.1} °C", component.temperature));
            }
        }

        // network part
        let mut t = self.network_history.borrow_mut();
        self.in_usage.set_text(format_number(sys.get_network().get_income()).as_str());
        self.out_usage.set_text(format_number(sys.get_network().get_outcome()).as_str());
        t.data[0].move_start();
        *t.data[0].get_mut(0).unwrap() = sys.get_network().get_income() as f64;
        t.data[1].move_start();
        *t.data[1].get_mut(0).unwrap() = sys.get_network().get_outcome() as f64;
    }

    pub fn update_process_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let mut h = &mut *self.cpu_usage_history.borrow_mut();

        for (i, pro) in sys.get_processor_list().iter().enumerate() {
            v[i].set_text(format!("{:.1} %", pro.get_cpu_usage() * 100.).as_str());
            v[i].set_show_text(true);
            v[i].set_fraction(pro.get_cpu_usage() as f64);
            if i > 0 {
                h.data[i - 1].move_start();
                if let Some(h) = h.data[i - 1].get_mut(0) {
                    *h = pro.get_cpu_usage() as f64;
                }
            }
        }
        h.invalidate();
        self.ram_usage_history.borrow().invalidate();
        self.temperature_usage_history.borrow().invalidate();
    }
}

fn connect_graph(graph: Graph) -> Rc<RefCell<Graph>> {
    let area = graph.area.clone();
    let graph = Rc::new(RefCell::new(graph));
    let c_graph = graph.clone();
    area.connect_draw(move |w, c| {
        let graph = c_graph.borrow();
        graph.draw(&c,
                   w.get_allocated_width() as f64,
                   w.get_allocated_height() as f64);
        Inhibit(false)
    });
    graph
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
