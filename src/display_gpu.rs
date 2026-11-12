use gtk::prelude::*;
use gtk::{glib, pango};

use std::iter;

use crate::display_sysinfo::{create_header, show_if_necessary};
use crate::graph::GraphWidget;
use crate::settings::Settings;
use crate::utils::{format_number, RotateVec};

struct GpuInfo {
    usage: gtk::ProgressBar,
    memory: gtk::ProgressBar,
}

pub struct DisplayGpu {
    gpus: Vec<GpuInfo>,
    usage_history: GraphWidget,
    pub check_box: Option<gtk::CheckButton>,
}

fn gpu_name(gpu: &sysinfo::Gpu) -> String {
    match (gpu.vendor(), gpu.model()) {
        (Some(vendor), Some(model)) => format!("{vendor} {model}"),
        (Some(name), None) | (None, Some(name)) => name.to_owned(),
        (None, None) => gpu.pci().to_string(),
    }
}

// The GPU name is displayed above the progress bar (and ellipsized if needed) so that long names
// don't force the tab to be wider than the window.
fn create_gpu_progress_bar(parent: &gtk::Box, name: &str) -> gtk::ProgressBar {
    let label = gtk::Label::builder()
        .label(name)
        .tooltip_text(name)
        .xalign(0.)
        .ellipsize(pango::EllipsizeMode::End)
        .build();
    let progress = gtk::ProgressBar::builder()
        .show_text(true)
        .hexpand(true)
        .build();
    let layout = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();
    layout.append(&label);
    layout.append(&progress);
    parent.append(&layout);
    progress
}

fn create_section_layout() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(10)
        .margin_end(10)
        .build()
}

impl DisplayGpu {
    pub fn new(gpus: &sysinfo::Gpus, stack: &gtk::Stack, settings: &Settings) -> Self {
        let vertical_layout = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(5)
            .margin_top(10)
            .margin_bottom(10)
            .build();
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&vertical_layout)
            .build();

        let usage_history = GraphWidget::new(None, false);
        usage_history.set_margin_start(3);
        usage_history.set_margin_end(6);
        usage_history.set_labels_callback(Some(Box::new(|_| {
            [
                "100".to_string(),
                "50".to_string(),
                "0".to_string(),
                "%".to_string(),
            ]
        })));

        let mut infos = Vec::with_capacity(gpus.len());
        let mut check_box = None;

        if gpus.is_empty() {
            vertical_layout.append(
                &gtk::Label::builder()
                    .label("No GPU found")
                    .hexpand(true)
                    .vexpand(true)
                    .build(),
            );
        } else {
            let usage_layout = create_section_layout();
            let memory_layout = create_section_layout();

            let usage_check_box =
                create_header("GPUs usage", &vertical_layout, settings.display_graph);
            for gpu in gpus {
                let name = gpu_name(gpu);
                let usage = create_gpu_progress_bar(&usage_layout, &name);
                let memory = create_gpu_progress_bar(&memory_layout, &name);
                usage_history.push(
                    RotateVec::new(iter::repeat_n(0f32, 61).collect()),
                    &name,
                    None,
                );
                infos.push(GpuInfo { usage, memory });
            }
            vertical_layout.append(&usage_layout);
            vertical_layout.append(&usage_history);

            let label = gtk::Label::builder()
                .label("GPUs memory usage")
                .margin_top(20)
                .build();
            vertical_layout.append(&label);
            vertical_layout.append(&memory_layout);

            usage_check_box.connect_toggled(glib::clone!(
                #[weak]
                usage_layout,
                #[weak]
                usage_history,
                move |c| {
                    show_if_necessary(c, &usage_history, &usage_layout);
                }
            ));
            // To show the correct view based on the saved settings.
            show_if_necessary(&usage_check_box, &usage_history, &usage_layout);
            check_box = Some(usage_check_box);
        }

        stack.add_titled(&scroll, Some("GPUs"), "GPUs");

        let display = Self {
            gpus: infos,
            usage_history,
            check_box,
        };
        display.update(gpus);
        display
    }

    pub fn update(&self, gpus: &sysinfo::Gpus) {
        for (pos, (gpu, info)) in gpus.iter().zip(self.gpus.iter()).enumerate() {
            let usage = gpu.usage();
            match usage {
                Some(usage) => {
                    info.usage.set_text(Some(&format!("{usage:.1} %")));
                    info.usage.set_fraction(f64::from(usage / 100.));
                }
                None => {
                    info.usage.set_text(Some("Not available"));
                    info.usage.set_fraction(0.);
                }
            }
            self.usage_history.data(pos, |d| {
                d.move_start();
                if let Some(d) = d.get_mut(0) {
                    *d = usage.unwrap_or(0.) / 100.;
                }
            });

            match (gpu.used_memory(), gpu.total_memory()) {
                (Some(used), Some(total)) => {
                    info.memory.set_text(Some(&format!(
                        "{} / {}",
                        format_number(used),
                        format_number(total)
                    )));
                    info.memory.set_fraction(if total != 0 {
                        used as f64 / total as f64
                    } else {
                        0.
                    });
                }
                _ => {
                    info.memory.set_text(Some("Not available"));
                    info.memory.set_fraction(0.);
                }
            }
        }
        self.usage_history.queue_draw();
    }
}
