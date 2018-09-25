use glib::object::Cast;
use gtk::{
    self, AdjustmentExt, BoxExt, ButtonExt, ContainerExt, DialogExt, LabelExt, ScrolledWindowExt
};
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
        self.working_directory.set_text(&process.cwd().display().to_string());
        self.memory_usage.set_text(&format_number(process.memory() << 10)); // * 1_024
        self.cpu_usage.set_text(&format!("{:.1}%", process.cpu_usage()));
        let running_since = compute_running_since(process, start_time, running_since);
        self.run_time.set_text(&format_time(running_since));
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

fn create_and_add_new_label(scroll: &gtk::Box, title: &str, text: &str) -> gtk::Label {
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    horizontal_layout.set_margin_top(5);
    horizontal_layout.set_margin_bottom(5);
    horizontal_layout.set_margin_right(5);
    horizontal_layout.set_margin_left(5);

    let label = gtk::Label::new(None);
    label.set_justify(gtk::Justification::Left);
    label.set_markup(&format!("<b>{}:</b>", title));

    let text = gtk::Label::new(text);
    text.set_selectable(true);
    text.set_justify(gtk::Justification::Left);
    text.set_line_wrap(true);
    text.set_line_wrap_mode(pango::WrapMode::Char);

    horizontal_layout.add(&label);
    horizontal_layout.add(&text);
    scroll.add(&horizontal_layout);
    text
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

    create_and_add_new_label(&labels, "name", process.name());
    create_and_add_new_label(&labels, "pid", &process.pid().to_string());
    let memory_usage = create_and_add_new_label(&labels,
                                                "memory usage",
                                                &format_number(process.memory() << 10));
    let cpu_usage = create_and_add_new_label(&labels,
                                             "cpu usage",
                                             &format!("{:.1}%", process.cpu_usage()));
    let run_time = create_and_add_new_label(&labels,
                                            "Running since",
                                            &format_time(running_since));
    create_and_add_new_label(&labels, "command", &format!("{:?}", process.cmd()));
    create_and_add_new_label(&labels, "executable path", &process.exe().display().to_string());
    let working_directory = create_and_add_new_label(&labels, "current working directory",
                                                     &process.cwd().display().to_string());
    create_and_add_new_label(&labels, "root directory", &process.root().display().to_string());
    let mut text = String::with_capacity(100);
    for env in process.environ() {
        text.push_str(&format!("\n{:?}", env));
    }
    create_and_add_new_label(&labels, "environment", &text);

    scroll.add(&labels);
    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);
    area.pack_start(&vertical_layout, true, true, 0);
    // To silence the annoying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    popup.get_preferred_width();
    popup.set_size_request(500, 500);

    let popup = popup.upcast::<gtk::Window>();
    popup.set_resizable(true);
    popup.show_all();
    let pop = popup.clone();
    close_button.connect_clicked(move |_| {
        pop.destroy();
    });

    if let Some(adjust) = scroll.get_vadjustment() {
        adjust.set_value(0.);
        scroll.set_vadjustment(&adjust);
    }
    ProcDialog {
        working_directory,
        memory_usage,
        cpu_usage,
        run_time,
        popup,
        pid: process.pid(),
    }
}
