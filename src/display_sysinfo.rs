use gdk;
use glib::object::Cast;
use gtk::prelude::{
    AdjustmentExt, BoxExt, ContainerExt, GridExt, GtkWindowExt, LabelExt, ProgressBarExt,
    ScrolledWindowExt, ToggleButtonExt, WidgetExt, WidgetExtManual,
};
use sysinfo::{self, ComponentExt, NetworkExt, ProcessorExt, SystemExt};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use graph::Graph;
use notebook::NoteBook;
use settings::Settings;
use utils::{connect_graph, format_number, RotateVec};

fn create_header(
    label_text: &str,
    parent_layout: &gtk::Box,
    display_graph: bool,
) -> gtk::CheckButton {
    let check_box = gtk::CheckButton::new_with_label("Graph view");
    check_box.set_active(display_graph);

    let label = gtk::Label::new(Some(label_text));
    let empty = gtk::Label::new(None);
    let grid = gtk::Grid::new();
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    horizontal_layout.pack_start(&gtk::Label::new(None), true, true, 0);
    horizontal_layout.pack_start(&check_box, false, false, 0);
    grid.attach(&empty, 0, 0, 3, 1);
    grid.attach_next_to(&label, Some(&empty), gtk::PositionType::Right, 3, 1);
    grid.attach_next_to(
        &horizontal_layout,
        Some(&label),
        gtk::PositionType::Right,
        3,
        1,
    );
    grid.set_column_homogeneous(true);
    parent_layout.pack_start(&grid, false, false, 15);
    check_box
}

pub fn create_progress_bar(
    non_graph_layout: &gtk::Grid,
    line: i32,
    label: &str,
    text: &str,
) -> gtk::ProgressBar {
    let p = gtk::ProgressBar::new();
    let l = gtk::Label::new(Some(label));

    p.set_text(Some(text));
    p.set_show_text(true);
    non_graph_layout.attach(&l, 0, line, 1, 1);
    non_graph_layout.attach(&p, 1, line, 11, 1);
    p
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
    pub network_check_box: gtk::CheckButton,
    pub temperature_check_box: Option<gtk::CheckButton>,
}

impl DisplaySysInfo {
    pub fn new(
        sys: &Rc<RefCell<sysinfo::System>>,
        note: &mut NoteBook,
        win: &gtk::ApplicationWindow,
        settings: &Settings,
    ) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let mut components = vec![];

        // CPU
        let mut cpu_usage_history = Graph::new(None, false);
        cpu_usage_history.set_label_callbacks(Some(Box::new(|_| {
            [
                "100".to_string(),
                "50".to_string(),
                "0".to_string(),
                "%".to_string(),
            ]
        })));

        // RAM
        let mut ram_usage_history = Graph::new(Some(sys.borrow().get_total_memory() as f64), true);
        ram_usage_history.set_label_callbacks(Some(Box::new(|v| {
            if v < 100_000. {
                [
                    v.to_string(),
                    format!("{}", v / 2.),
                    "0".to_string(),
                    "kB".to_string(),
                ]
            } else if v < 10_000_000. {
                [
                    format!("{:.1}", v / 1_024f64),
                    format!("{:.1}", v / 2_048f64),
                    "0".to_string(),
                    "MB".to_string(),
                ]
            } else if v < 10_000_000_000. {
                [
                    format!("{:.1}", v / 1_048_576f64),
                    format!("{:.1}", v / 2_097_152f64),
                    "0".to_string(),
                    "GB".to_string(),
                ]
            } else {
                [
                    format!("{:.1}", v / 1_073_741_824f64),
                    format!("{:.1}", v / 1_073_741_824f64),
                    "0".to_string(),
                    "TB".to_string(),
                ]
            }
        })));
        ram_usage_history.set_labels_width(70);

        // TEMPERATURE
        let mut temperature_usage_history = Graph::new(Some(1.), false);
        temperature_usage_history.set_overhead(Some(20.));
        temperature_usage_history.set_label_callbacks(Some(Box::new(|v| {
            [
                format!("{:.1}", v),
                format!("{:.1}", v / 2.),
                "0".to_string(),
                "°C".to_string(),
            ]
        })));
        temperature_usage_history.set_labels_width(70);
        // NETWORK
        let mut network_history = Graph::new(Some(1.), false);
        network_history.set_overhead(Some(20.));
        network_history.set_label_callbacks(Some(Box::new(|v| {
            let v = v as u64;
            if v < 1000 {
                return [
                    v.to_string(),
                    (v >> 1).to_string(),
                    "0".to_string(),
                    "B/sec".to_string(),
                ];
            }
            let nb = v >> 10; // / 1_024
            if nb < 100_000 {
                [
                    nb.to_string(),
                    (nb >> 1).to_string(),
                    "0".to_string(),
                    "kB/sec".to_string(),
                ]
            } else if nb < 10_000_000 {
                [
                    (nb >> 10).to_string(),
                    (nb >> 11).to_string(),
                    "0".to_string(),
                    "MB/sec".to_string(),
                ]
            } else if nb < 10_000_000_000 {
                [
                    (nb >> 20).to_string(),
                    (nb >> 21).to_string(),
                    "0".to_string(),
                    "GB/sec".to_string(),
                ]
            } else {
                [
                    (nb >> 30).to_string(),
                    (nb >> 31).to_string(),
                    "0".to_string(),
                    "TB/sec".to_string(),
                ]
            }
        })));
        network_history.set_labels_width(70);

        let mut check_box3 = None;

        vertical_layout.set_spacing(5);
        vertical_layout.set_margin_top(10);
        vertical_layout.set_margin_bottom(10);

        let non_graph_layout = gtk::Grid::new();
        non_graph_layout.set_column_homogeneous(true);
        non_graph_layout.set_margin_end(5);
        let non_graph_layout2 = gtk::Grid::new();
        non_graph_layout2.set_column_homogeneous(true);
        non_graph_layout2.set_margin_start(5);
        let non_graph_layout3 = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let non_graph_layout4 = gtk::Box::new(gtk::Orientation::Vertical, 0);

        vertical_layout.pack_start(&gtk::Label::new(Some("Total CPU usage")), false, false, 7);
        procs.push(gtk::ProgressBar::new());
        {
            let p: &gtk::ProgressBar = &procs[0];
            let s = sys.borrow();

            p.set_margin_end(5);
            p.set_margin_start(5);
            p.set_show_text(true);
            let processor_list = s.get_processor_list();
            if !processor_list.is_empty() {
                let pro = &processor_list[0];
                p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
                p.set_fraction(f64::from(pro.get_cpu_usage()));
            } else {
                p.set_text(Some("0.0 %"));
                p.set_fraction(0.);
            }
            vertical_layout.add(p);
        }

        //
        // PROCESS PART
        //
        let check_box = create_header("Process usage", &vertical_layout, settings.display_graph);
        for (i, pro) in sys.borrow().get_processor_list().iter().skip(1).enumerate() {
            let i = i + 1;
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[i];
            let l = gtk::Label::new(Some(&format!("{}", i)));

            p.set_text(Some(&format!("{:.2} %", pro.get_cpu_usage() * 100.)));
            p.set_show_text(true);
            p.set_fraction(f64::from(pro.get_cpu_usage()));
            non_graph_layout.attach(&l, 0, i as i32 - 1, 1, 1);
            non_graph_layout.attach(p, 1, i as i32 - 1, 11, 1);
            cpu_usage_history.push(
                RotateVec::new(iter::repeat(0f64).take(61).collect()),
                &format!("process {}", i),
                None,
            );
        }
        vertical_layout.add(&non_graph_layout);
        cpu_usage_history.attach_to(&vertical_layout);

        //
        // MEMORY PART
        //
        let check_box2 = create_header("Memory usage", &vertical_layout, settings.display_graph);
        let ram = create_progress_bar(&non_graph_layout2, 0, "RAM", "");
        let swap = create_progress_bar(&non_graph_layout2, 1, "Swap", "");
        vertical_layout.pack_start(&non_graph_layout2, false, false, 15);
        //vertical_layout.add(&non_graph_layout2);
        ram_usage_history.push(
            RotateVec::new(iter::repeat(0f64).take(61).collect()),
            "RAM",
            Some(4),
        );
        ram_usage_history.push(
            RotateVec::new(iter::repeat(0f64).take(61).collect()),
            "Swap",
            Some(2),
        );
        ram_usage_history.attach_to(&vertical_layout);

        //
        // TEMPERATURES PART
        //
        if !sys.borrow().get_components_list().is_empty() {
            check_box3 = Some(create_header(
                "Components' temperature",
                &vertical_layout,
                settings.display_graph,
            ));
            for component in sys.borrow().get_components_list() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                // TODO: add max and critical temperatures as well
                let temp = gtk::Label::new(Some(&format!("{:.1} °C", component.get_temperature())));
                horizontal_layout.pack_start(
                    &gtk::Label::new(Some(component.get_label())),
                    true,
                    false,
                    0,
                );
                horizontal_layout.pack_start(&temp, true, false, 0);
                horizontal_layout.set_homogeneous(true);
                non_graph_layout3.add(&horizontal_layout);
                components.push(temp);
                temperature_usage_history.push(
                    RotateVec::new(iter::repeat(0f64).take(61).collect()),
                    component.get_label(),
                    None,
                );
            }
            vertical_layout.add(&non_graph_layout3);
            temperature_usage_history.attach_to(&vertical_layout);
        }

        //
        // NETWORK PART
        //
        let check_box4 = create_header("Network usage", &vertical_layout, settings.display_graph);
        // input data
        let in_usage = gtk::Label::new(Some(&format_number(0)));
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        horizontal_layout.pack_start(&gtk::Label::new(Some("Input data")), true, false, 0);
        horizontal_layout.pack_start(&in_usage, true, false, 0);
        horizontal_layout.set_homogeneous(true);
        non_graph_layout4.add(&horizontal_layout);
        network_history.push(
            RotateVec::new(iter::repeat(0f64).take(61).collect()),
            "Input data",
            None,
        );
        // output data
        let out_usage = gtk::Label::new(Some(&format_number(0)));
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        horizontal_layout.pack_start(&gtk::Label::new(Some("Output data")), true, false, 0);
        horizontal_layout.pack_start(&out_usage, true, false, 0);
        horizontal_layout.set_homogeneous(true);
        non_graph_layout4.add(&horizontal_layout);
        network_history.push(
            RotateVec::new(iter::repeat(0f64).take(61).collect()),
            "Output data",
            None,
        );
        vertical_layout.add(&non_graph_layout4);
        network_history.attach_to(&vertical_layout);
        network_history.area.set_margin_bottom(20);

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
        note.create_tab("System usage", &scroll);

        // It greatly improves the scrolling on the system information tab. No more clipping.
        if let Some(adjustment) = scroll.get_vadjustment() {
            adjustment.connect_value_changed(
                clone!(@weak cpu_usage_history, @weak ram_usage_history, @weak temperature_usage_history,
                       @weak network_history => move |_| {
                cpu_usage_history.borrow().invalidate();
                ram_usage_history.borrow().invalidate();
                temperature_usage_history.borrow().invalidate();
                network_history.borrow().invalidate();
            }));
        }

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram: ram.clone(),
            swap: swap.clone(),
            out_usage: out_usage.clone(),
            in_usage: in_usage.clone(),
            vertical_layout,
            components,
            cpu_usage_history: Rc::clone(&cpu_usage_history),
            ram_usage_history: Rc::clone(&ram_usage_history),
            ram_check_box: check_box.clone(),
            swap_check_box: check_box2.clone(),
            temperature_usage_history: Rc::clone(&temperature_usage_history),
            temperature_check_box: check_box3.clone(),
            network_history: Rc::clone(&network_history),
            network_check_box: check_box4.clone(),
        };
        tmp.update_system_info(&sys.borrow(), settings.display_fahrenheit);

        win.add_events(gdk::EventMask::STRUCTURE_MASK);
        // TODO: ugly way to resize drawing area, I should find a better way
        win.connect_configure_event(move |w, _| {
            // To silence the annoying warning:
            // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
            // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
            // allocate?"
            w.get_preferred_width();
            let w = w.clone().upcast::<gtk::Window>().get_size().0 - 130;
            area.set_size_request(w, 200);
            area2.set_size_request(w, 200);
            area3.set_size_request(w, 200);
            area4.set_size_request(w, 200);
            false
        });

        check_box
            .clone()
            .upcast::<gtk::ToggleButton>()
            .connect_toggled(
                clone!(@weak non_graph_layout, @weak cpu_usage_history => move |c| {
                    show_if_necessary(c, &cpu_usage_history.borrow(), &non_graph_layout);
                }),
            );
        check_box2
            .clone()
            .upcast::<gtk::ToggleButton>()
            .connect_toggled(
                clone!(@weak non_graph_layout2, @weak ram_usage_history => move |c| {
                    show_if_necessary(c, &ram_usage_history.borrow(), &non_graph_layout2);
                }),
            );
        if let Some(ref check_box3) = check_box3 {
            check_box3.clone().upcast::<gtk::ToggleButton>()
                      .connect_toggled(
                          clone!(@weak non_graph_layout3, @weak temperature_usage_history => move |c| {
                show_if_necessary(c, &temperature_usage_history.borrow(), &non_graph_layout3);
            }));
        }
        check_box4
            .clone()
            .upcast::<gtk::ToggleButton>()
            .connect_toggled(
                clone!(@weak non_graph_layout4, @weak network_history => move |c| {
                    show_if_necessary(c, &network_history.borrow(), &non_graph_layout4);
                }),
            );

        scroll.connect_show(
            clone!(@weak cpu_usage_history, @weak ram_usage_history => move |_| {
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
            }),
        );
        tmp
    }

    pub fn update_system_info(&mut self, sys: &sysinfo::System, display_fahrenheit: bool) {
        let disp = |total, used| {
            if total < 100_000 {
                format!("{} / {} kB", used, total)
            } else if total < 10_000_000 {
                format!("{:.2} / {} MB", used as f64 / 1_024f64, total >> 10) // / 1024
            } else if total < 10_000_000_000 {
                format!("{:.2} / {} GB", used as f64 / 1_048_576f64, total >> 20)
            // / 1_048_576
            } else {
                format!("{:.2} / {} TB", used as f64 / 1_073_741_824f64, total >> 30)
                // / 1_073_741_824
            }
        };

        let total_ram = sys.get_total_memory();
        let used = sys.get_used_memory();
        self.ram.set_text(Some(&disp(total_ram, used)));
        if total_ram != 0 {
            self.ram.set_fraction(used as f64 / total_ram as f64);
        } else {
            self.ram.set_fraction(0.0);
        }
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[0].move_start();
            if let Some(p) = r.data[0].get_mut(0) {
                *p = used as f64;
            }
        }

        let total = ::std::cmp::max(sys.get_total_swap(), total_ram);
        let used = sys.get_used_swap();
        self.swap.set_text(Some(&disp(sys.get_total_swap(), used)));

        let mut fraction = if total != 0 {
            used as f64 / total as f64
        } else {
            0f64
        };
        if fraction.is_nan() {
            fraction = 0f64;
        }
        self.swap.set_fraction(fraction);
        {
            let mut r = self.ram_usage_history.borrow_mut();
            r.data[1].move_start();
            if let Some(p) = r.data[1].get_mut(0) {
                *p = used as f64;
            }
        }

        // temperature part
        let mut t = self.temperature_usage_history.borrow_mut();
        for (pos, (component, label)) in sys
            .get_components_list()
            .iter()
            .zip(self.components.iter())
            .enumerate()
        {
            t.data[pos].move_start();
            if let Some(t) = t.data[pos].get_mut(0) {
                *t = f64::from(component.get_temperature());
            }
            if let Some(t) = t.data[pos].get_mut(0) {
                *t = f64::from(component.get_temperature());
            }
            if display_fahrenheit {
                label.set_text(&format!(
                    "{:.1} °F",
                    component.get_temperature() * 1.8 + 32.
                ));
            } else {
                label.set_text(&format!("{:.1} °C", component.get_temperature()));
            }
        }
    }

    pub fn update_network(&mut self, sys: &sysinfo::System) {
        let mut t = self.network_history.borrow_mut();
        self.in_usage
            .set_text(format_number(sys.get_network().get_income()).as_str());
        self.out_usage
            .set_text(format_number(sys.get_network().get_outcome()).as_str());
        t.data[0].move_start();
        *t.data[0].get_mut(0).expect("cannot get data 0") = sys.get_network().get_income() as f64;
        t.data[1].move_start();
        *t.data[1].get_mut(0).expect("cannot get data 1") = sys.get_network().get_outcome() as f64;
    }

    pub fn update_system_info_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let h = &mut *self.cpu_usage_history.borrow_mut();

        for (i, pro) in sys.get_processor_list().iter().enumerate() {
            v[i].set_text(Some(&format!("{:.1} %", pro.get_cpu_usage() * 100.)));
            v[i].set_show_text(true);
            v[i].set_fraction(f64::from(pro.get_cpu_usage()));
            if i > 0 {
                h.data[i - 1].move_start();
                if let Some(h) = h.data[i - 1].get_mut(0) {
                    *h = f64::from(pro.get_cpu_usage());
                }
            }
        }
        h.invalidate();
        self.ram_usage_history.borrow().invalidate();
        self.temperature_usage_history.borrow().invalidate();
        self.network_history.borrow().invalidate();
    }
}

fn show_if_necessary<T: WidgetExt>(
    check_box: &gtk::ToggleButton,
    proc_horizontal_layout: &Graph,
    non_graph_layout: &T,
) {
    if check_box.get_active() {
        proc_horizontal_layout.show_all();
        non_graph_layout.hide();
    } else {
        non_graph_layout.show_all();
        proc_horizontal_layout.hide();
    }
}
