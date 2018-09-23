use glib::object::Cast;
use gtk::{self, BoxExt, ButtonExt, ContainerExt, DialogExt, LabelExt, ScrolledWindowExt};
use gtk::{WidgetExt, GtkWindowExt};
use pango;
use sysinfo::{self, Pid, ProcessExt};

use utils::format_number;

pub struct ProcDialog {
    working_directory: gtk::Label,
    memory_usage: gtk::Label,
    cpu_usage: gtk::Label,
    run_time: gtk::Label,
    pub popup: gtk::Window,
    pub pid: Pid,
}

impl ProcDialog {
    pub fn update(&self, process: &sysinfo::Process, running_since: u64, start_time: u64) {
        self.working_directory.set_text(&format!("current working directory: {}",
                                                 process.cwd().display()));
        self.memory_usage.set_text(&format!("memory usage: {}",
                                            format_number(process.memory() << 10))); // * 1_024
        self.cpu_usage.set_text(&format!("cpu usage: {:.1}%", process.cpu_usage()));
        let running_since = compute_running_since(process, start_time, running_since);
        self.run_time.set_text(&format!("Running since: {}", format_time(running_since)));
    }
}

fn format_time(t: u64) -> String {
    format!("{}{}{}{}s",
            {
                let days = t / 86_400;
                if days > 0 {
                    format!("{}d ", days)
                } else {
                    "".to_owned()
                }
            },
            {
                let hours = t / 3_600 % 24;
                if hours > 0 {
                    format!("{}h ", hours)
                } else {
                    "".to_owned()
                }
            },
            {
                let minutes = t / 60 % 60;
                if minutes > 0 {
                    format!("{}m ", minutes)
                } else {
                    "".to_owned()
                }
            },
            t % 60)
}

fn create_and_add_new_label(scroll: &gtk::Box, text: &str) -> gtk::Label {
    let label = gtk::Label::new(text);
    label.set_selectable(true);
    label.set_justify(gtk::Justification::Left);
    label.set_line_wrap(true);
    label.set_line_wrap_mode(pango::WrapMode::Char);
    scroll.add(&label);
    label
}

fn compute_running_since(
    process: &sysinfo::Process,
    start_time: u64,
    running_since: u64,
) -> u64 {
    if start_time > process.start_time() {
        start_time - process.start_time() + running_since
    } else {
        start_time + running_since - process.start_time()
    }
}

pub fn create_process_dialog(
    process: &sysinfo::Process,
    window: &gtk::ApplicationWindow,
    running_since: u64,
    start_time: u64
) -> ProcDialog {
    let scroll = gtk::ScrolledWindow::new(None, None);
    let close_button = gtk::Button::new_with_label("Close");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

    let flags = gtk::DialogFlags::DESTROY_WITH_PARENT | gtk::DialogFlags::USE_HEADER_BAR;
    let popup = gtk::Dialog::new_with_buttons(
                    Some(&format!("Information about {}", process.name())),
                    Some(window),
                    flags,
                    &[]);
    let running_since = compute_running_since(process, start_time, running_since);
    let area = popup.get_content_area();

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);

    create_and_add_new_label(&labels, &format!("name: {}", process.name()));
    create_and_add_new_label(&labels, &format!("pid: {}", process.pid()));
    let memory_usage = create_and_add_new_label(&labels,
                                                &format!("memory usage: {}",
                                                         format_number(process.memory() << 10)));
    let cpu_usage = create_and_add_new_label(&labels,
                                             &format!("cpu usage: {:.1}%", process.cpu_usage()));
    let run_time = create_and_add_new_label(&labels,
                                            &format!("Running since: {}",
                                                     format_time(running_since)));
    create_and_add_new_label(&labels, &format!("command: {:?}", process.cmd()));
    create_and_add_new_label(&labels, &format!("executable path: {}", process.exe().display()));
    let working_directory = create_and_add_new_label(&labels,
                                                     &format!("current working directory: {}",
                                                              process.cwd().display()));
    create_and_add_new_label(&labels, &format!("root directory: {}", process.root().display()));
    let mut text = format!("environment:");
    for env in process.environ() {
        text.push_str(&format!("\n{:?}", env));
    }
    create_and_add_new_label(&labels, &text);

    scroll.add(&labels);
    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);
    vertical_layout.set_spacing(10);
    area.pack_start(&vertical_layout, true, true, 0);
    // To silence the annoying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    popup.get_preferred_width();
    popup.set_size_request(500, 700);

    let popup = popup.upcast::<gtk::Window>();
    popup.set_resizable(false);
    popup.show_all();
    let pop = popup.clone();
    close_button.connect_clicked(move |_| {
        pop.destroy();
    });
    ProcDialog {
        working_directory,
        memory_usage,
        cpu_usage,
        run_time,
        popup,
        pid: process.pid(),
    }
}
