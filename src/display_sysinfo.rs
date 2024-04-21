use gtk::glib;
use gtk::prelude::*;
use sysinfo::{ComponentExt, CpuExt, SystemExt};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::graph::GraphWidget;
use crate::settings::Settings;
use crate::utils::{format_number, graph_label_units, RotateVec};

pub fn create_header(
    label_text: &str,
    parent_layout: &gtk::Box,
    display_graph: bool,
) -> gtk::CheckButton {
    let check_box = gtk::CheckButton::builder()
        .label("Graph view")
        .active(display_graph)
        .halign(gtk::Align::End)
        .build();

    let label = gtk::Label::new(Some(label_text));
    let grid = gtk::Grid::builder()
        .hexpand(true)
        .column_homogeneous(true)
        .build();
    grid.attach(&gtk::Label::new(None), 0, 0, 2, 1); // needed otherwise it won't take space
    grid.attach(&label, 1, 0, 2, 1);
    grid.attach(&check_box, 3, 0, 1, 1);
    parent_layout.append(&grid);
    check_box
}

pub fn create_progress_bar(
    non_graph_layout: &gtk::Grid,
    line: i32,
    label: &str,
    text: &str,
) -> gtk::ProgressBar {
    let p = gtk::ProgressBar::builder()
        .text(text)
        .show_text(true)
        .build();
    let l = gtk::Label::new(Some(label));

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
    components: Vec<gtk::Label>,
    cpu_usage_history: Rc<RefCell<GraphWidget>>,
    // 0 = RAM
    // 1 = SWAP
    ram_usage_history: Rc<RefCell<GraphWidget>>,
    temperature_usage_history: Rc<RefCell<GraphWidget>>,
    pub ram_check_box: gtk::CheckButton,
    pub swap_check_box: gtk::CheckButton,
    pub temperature_check_box: Option<gtk::CheckButton>,
}

impl DisplaySysInfo {
    pub fn new(
        sys: &Arc<Mutex<sysinfo::System>>,
        stack: &gtk::Stack,
        settings: &Settings,
    ) -> DisplaySysInfo {
        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mut procs = Vec::new();
        let scroll = gtk::ScrolledWindow::new();
        let mut components = vec![];

        // CPU
        let cpu_usage_history = GraphWidget::new(None, false);
        cpu_usage_history.set_margin_start(3);
        cpu_usage_history.set_margin_end(6);
        cpu_usage_history.set_labels_callback(Some(Box::new(|_| {
            [
                "100".to_string(),
                "50".to_string(),
                "0".to_string(),
                "%".to_string(),
            ]
        })));

        let sys = sys.lock().expect("failed to lock in DisplaySysInfo::new");
        // RAM
        let ram_usage_history = GraphWidget::new(Some(sys.total_memory() as f32), true);
        ram_usage_history.set_margin_start(3);
        ram_usage_history.set_margin_end(6);
        ram_usage_history.set_labels_callback(Some(Box::new(graph_label_units)));

        // TEMPERATURE
        let temperature_usage_history = GraphWidget::new(Some(1.), false);
        temperature_usage_history.set_margin_start(3);
        temperature_usage_history.set_margin_end(6);
        temperature_usage_history.set_overhead(Some(20.));
        temperature_usage_history.set_labels_callback(Some(Box::new(|v| {
            [
                format!("{:.1}", v),
                format!("{:.1}", v / 2.),
                "0".to_string(),
                "°C".to_string(),
            ]
        })));

        let mut check_box3 = None;

        vertical_layout.set_spacing(5);
        vertical_layout.set_margin_top(10);
        vertical_layout.set_margin_bottom(10);

        let non_graph_layout = gtk::Grid::builder()
            .column_homogeneous(true)
            .margin_end(5)
            .build();
        let non_graph_layout2 = gtk::Grid::builder()
            .column_homogeneous(true)
            .margin_start(5)
            .build();
        let non_graph_layout3 = gtk::Box::new(gtk::Orientation::Vertical, 0);

        //
        // PROCESSOR PART
        //
        let label = gtk::Label::new(Some("Total CPU usage"));
        label.set_margin_start(7);
        vertical_layout.append(&label);
        procs.push(gtk::ProgressBar::new());
        {
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[0];

            p.set_margin_end(5);
            p.set_margin_start(5);
            p.set_show_text(true);
            let processor = sys.global_cpu_info();
            p.set_text(Some(&format!("{:.1} %", processor.cpu_usage())));
            p.set_fraction(f64::from(processor.cpu_usage() / 100.));
            vertical_layout.append(p);
        }
        let check_box = create_header("Processors usage", &vertical_layout, settings.display_graph);
        for (i, pro) in sys.cpus().iter().enumerate() {
            procs.push(gtk::ProgressBar::new());
            let p: &gtk::ProgressBar = &procs[i + 1];
            let l = gtk::Label::new(Some(&format!("{}", i)));

            p.set_text(Some(&format!("{:.1} %", pro.cpu_usage())));
            p.set_show_text(true);
            p.set_fraction(f64::from(pro.cpu_usage()));
            non_graph_layout.attach(&l, 0, i as i32 - 1, 1, 1);
            non_graph_layout.attach(p, 1, i as i32 - 1, 11, 1);
            cpu_usage_history.push(
                RotateVec::new(iter::repeat(0f32).take(61).collect()),
                &format!("processor {}", i),
                None,
            );
        }
        vertical_layout.append(&non_graph_layout);
        vertical_layout.append(&cpu_usage_history);

        //
        // MEMORY PART
        //
        let check_box2 = create_header("Memory usage", &vertical_layout, settings.display_graph);
        let ram = create_progress_bar(&non_graph_layout2, 0, "RAM", "");
        let swap = create_progress_bar(&non_graph_layout2, 1, "Swap", "");
        non_graph_layout2.set_margin_start(15);
        vertical_layout.append(&non_graph_layout2);
        //vertical_layout.append(&non_graph_layout2);
        ram_usage_history.push(
            RotateVec::new(iter::repeat(0f32).take(61).collect()),
            "RAM",
            Some(4),
        );
        ram_usage_history.push(
            RotateVec::new(iter::repeat(0f32).take(61).collect()),
            "Swap",
            Some(2),
        );
        vertical_layout.append(&ram_usage_history);

        //
        // TEMPERATURES PART
        //
        if !sys.components().is_empty() {
            check_box3 = Some(create_header(
                "Components' temperature",
                &vertical_layout,
                settings.display_graph,
            ));
            for component in sys.components() {
                let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                // TODO: add max and critical temperatures as well
                let temp = gtk::Label::new(Some(&format!("{:.1} °C", component.temperature())));
                horizontal_layout.append(&gtk::Label::new(Some(component.label())));
                horizontal_layout.append(&temp);
                horizontal_layout.set_homogeneous(true);
                non_graph_layout3.append(&horizontal_layout);
                components.push(temp);
                temperature_usage_history.push(
                    RotateVec::new(iter::repeat(0f32).take(61).collect()),
                    component.label(),
                    None,
                );
            }
            vertical_layout.append(&non_graph_layout3);
            vertical_layout.append(&temperature_usage_history);
        }

        //
        // Putting everyting into places now.
        //
        let cpu_usage_history = Rc::new(RefCell::new(cpu_usage_history));
        let ram_usage_history = Rc::new(RefCell::new(ram_usage_history));
        let temperature_usage_history = Rc::new(RefCell::new(temperature_usage_history));

        scroll.set_child(Some(&vertical_layout));

        stack.add_titled(&scroll, Some("System"), "System");

        cpu_usage_history.borrow().hide();
        ram_usage_history.borrow().hide();
        temperature_usage_history.borrow().hide();

        check_box.connect_toggled(
            glib::clone!(@weak non_graph_layout, @weak cpu_usage_history => move |c| {
                show_if_necessary(c, &cpu_usage_history.borrow(), &non_graph_layout);
            }),
        );
        // To show the correct view based on the saved settings.
        show_if_necessary(&check_box, &cpu_usage_history.borrow(), &non_graph_layout);
        check_box2.connect_toggled(
            glib::clone!(@weak non_graph_layout2, @weak ram_usage_history => move |c| {
                show_if_necessary(c, &ram_usage_history.borrow(), &non_graph_layout2);
            }),
        );
        // To show the correct view based on the saved settings.
        show_if_necessary(&check_box2, &ram_usage_history.borrow(), &non_graph_layout2);
        if let Some(ref check_box3) = check_box3 {
            check_box3.connect_toggled(
                glib::clone!(@weak non_graph_layout3, @weak temperature_usage_history => move |c| {
                    show_if_necessary(c, &temperature_usage_history.borrow(), &non_graph_layout3);
                }),
            );
            // To show the correct view based on the saved settings.
            show_if_necessary(
                check_box3,
                &temperature_usage_history.borrow(),
                &non_graph_layout3,
            );
        }

        let mut tmp = DisplaySysInfo {
            procs: Rc::new(RefCell::new(procs)),
            ram,
            swap,
            vertical_layout,
            components,
            cpu_usage_history,
            ram_usage_history,
            ram_check_box: check_box,
            swap_check_box: check_box2,
            temperature_usage_history,
            temperature_check_box: check_box3,
        };
        tmp.update_system_info(&sys, settings.display_fahrenheit);
        tmp
    }

    pub fn set_checkboxes_state(&self, active: bool) {
        self.ram_check_box.set_active(active);
        self.swap_check_box.set_active(active);
        if let Some(ref temperature_check_box) = self.temperature_check_box {
            temperature_check_box.set_active(active);
        }
    }

    pub fn update_system_info(&mut self, sys: &sysinfo::System, display_fahrenheit: bool) {
        let disp = |total, used| {
            format!(
                "{} / {}",
                format_number(used),
                format_number(total) // We need to multiply to get the "right" unit.
            )
        };

        let total_ram = sys.total_memory();
        let used = sys.used_memory();
        self.ram.set_text(Some(&disp(total_ram, used)));
        if total_ram != 0 {
            self.ram.set_fraction(used as f64 / total_ram as f64);
        } else {
            self.ram.set_fraction(0.0);
        }
        {
            let r = self.ram_usage_history.borrow_mut();
            r.data(0, |d| {
                d.move_start();
                if let Some(p) = d.get_mut(0) {
                    *p = used as _;
                }
            });
        }

        let total = ::std::cmp::max(sys.total_swap(), total_ram);
        let used = sys.used_swap();
        self.swap.set_text(Some(&disp(sys.total_swap(), used)));

        let mut fraction = if total != 0 {
            used as f64 / total as f64
        } else {
            0f64
        };
        if fraction.is_nan() {
            fraction = 0.;
        }
        self.swap.set_fraction(fraction);
        {
            let r = self.ram_usage_history.borrow_mut();
            r.data(1, |d| {
                d.move_start();
                if let Some(p) = d.get_mut(0) {
                    *p = used as _;
                }
            });
        }

        // temperature part
        let t = self.temperature_usage_history.borrow_mut();
        for (pos, (component, label)) in sys
            .components()
            .iter()
            .zip(self.components.iter())
            .enumerate()
        {
            t.data(pos, |d| {
                d.move_start();
                if let Some(t) = d.get_mut(0) {
                    *t = component.temperature();
                }
                if let Some(t) = d.get_mut(0) {
                    *t = component.temperature();
                }
            });
            if display_fahrenheit {
                label.set_text(&format!("{:.1} °F", component.temperature() * 1.8 + 32.));
            } else {
                label.set_text(&format!("{:.1} °C", component.temperature()));
            }
        }
    }

    pub fn update_system_info_display(&mut self, sys: &sysinfo::System) {
        let v = &*self.procs.borrow_mut();
        let h = &mut *self.cpu_usage_history.borrow_mut();

        v[0].set_text(Some(&format!("{:.1} %", sys.global_cpu_info().cpu_usage())));
        v[0].set_show_text(true);
        v[0].set_fraction(f64::from(sys.global_cpu_info().cpu_usage() / 100.));
        for (i, pro) in sys.cpus().iter().enumerate() {
            let i = i + 1;
            v[i].set_text(Some(&format!("{:.1} %", pro.cpu_usage())));
            v[i].set_show_text(true);
            v[i].set_fraction(f64::from(pro.cpu_usage() / 100.));
            h.data(i - 1, |d| {
                d.move_start();
                if let Some(h) = d.get_mut(0) {
                    *h = pro.cpu_usage() / 100.;
                }
            });
        }
        h.queue_draw();
        self.ram_usage_history.borrow().queue_draw();
        self.temperature_usage_history.borrow().queue_draw();
    }
}

pub fn show_if_necessary<U: gtk::glib::object::IsA<gtk::CheckButton>, T: WidgetExt>(
    check_box: &U,
    proc_horizontal_layout: &GraphWidget,
    non_graph_layout: &T,
) {
    if check_box.is_active() {
        proc_horizontal_layout.show();
        non_graph_layout.hide();
    } else {
        non_graph_layout.show();
        proc_horizontal_layout.hide();
    }
}
