use gtk::prelude::*;
use gtk::{glib, pango, EventControllerKey, Inhibit};
use sysinfo::{self, Pid, ProcessExt};

use std::cell::{Cell, RefCell};
use std::fmt;
use std::iter;
use std::rc::Rc;

use crate::graph::GraphWidget;
use crate::notebook::NoteBook;
use crate::utils::{format_number, get_main_window, graph_label_units, RotateVec};

#[allow(dead_code)]
pub struct ProcDialog {
    working_directory: gtk::Label,
    memory_usage: gtk::Label,
    disk_usage: gtk::Label,
    cpu_usage: gtk::Label,
    run_time: gtk::Label,
    pub popup: gtk::Window,
    pub pid: Pid,
    notebook: NoteBook,
    ram_usage_history: Rc<RefCell<GraphWidget>>,
    cpu_usage_history: Rc<RefCell<GraphWidget>>,
    disk_usage_history: Rc<RefCell<GraphWidget>>,
    memory_peak: RefCell<u64>,
    memory_peak_label: gtk::Label,
    disk_peak: RefCell<u64>,
    disk_peak_label: gtk::Label,
    pub is_dead: bool,
    pub to_be_removed: Rc<Cell<bool>>,
}

impl fmt::Debug for ProcDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProcDialog {{ pid: {} }}", self.pid)
    }
}

impl ProcDialog {
    pub fn update(&self, process: &sysinfo::Process) {
        if self.is_dead {
            return;
        }
        self.working_directory
            .set_text(&process.cwd().display().to_string());
        let memory = process.memory() * 1_000; // It returns in kB so we have to convert it to B
        let memory_s = format_number(memory);
        self.memory_usage.set_text(&memory_s);
        if memory > *self.memory_peak.borrow() {
            *self.memory_peak.borrow_mut() = memory;
            self.memory_peak_label.set_text(&memory_s);
        }
        let disk_usage = process.disk_usage();
        let disk_usage = disk_usage.written_bytes + disk_usage.read_bytes;
        let disk_usage_s = format_number(disk_usage);
        self.disk_usage.set_text(&disk_usage_s);
        if disk_usage > *self.disk_peak.borrow() {
            *self.disk_peak.borrow_mut() = disk_usage;
            self.disk_peak_label.set_text(&disk_usage_s);
        }
        self.cpu_usage
            .set_text(&format!("{:.1}%", process.cpu_usage()));
        self.run_time.set_text(&format_time(process.run_time()));

        let t = self.ram_usage_history.borrow_mut();
        t.data(0, |d| {
            d.move_start();
            *d.get_mut(0).expect("cannot get data 0") = memory as f32;
        });
        t.queue_draw();
        let t = self.cpu_usage_history.borrow_mut();
        t.data(0, |d| {
            d.move_start();
            *d.get_mut(0).expect("cannot get data 0") = process.cpu_usage().into();
        });
        t.queue_draw();
        let t = self.disk_usage_history.borrow_mut();
        t.data(0, |d| {
            d.move_start();
            *d.get_mut(0).expect("cannot get data 0") = disk_usage as f32;
        });
        t.queue_draw();
    }

    pub fn need_remove(&self) -> bool {
        self.to_be_removed.get()
    }

    pub fn set_dead(&mut self) {
        if self.is_dead {
            return;
        }
        self.is_dead = true;
        self.memory_usage.set_text("0");
        self.disk_usage.set_text("0");
        self.cpu_usage.set_text("0%");
        let time = self.run_time.text();
        let s = format!("Ran for {}", if time.is_empty() { "0s" } else { &time },);
        self.run_time.set_text(&s);
    }
}

fn format_time(t: u64) -> String {
    format!(
        "{}{}{}{}s",
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
        t % 60
    )
}

fn create_and_add_new_label(scroll: &gtk::Box, title: &str, text: &str) -> gtk::Label {
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    horizontal_layout.set_margin_top(5);
    horizontal_layout.set_margin_bottom(5);
    horizontal_layout.set_margin_end(5);
    horizontal_layout.set_margin_start(5);

    let label = gtk::Label::new(None);
    label.set_justify(gtk::Justification::Left);
    label.set_markup(&format!("<b>{}:</b> ", title));

    let text = gtk::Label::new(Some(text));
    text.set_selectable(true);
    text.set_justify(gtk::Justification::Left);
    text.set_wrap(true);
    text.set_wrap_mode(pango::WrapMode::Char);

    horizontal_layout.append(&label);
    horizontal_layout.append(&text);
    scroll.append(&horizontal_layout);
    text
}

fn append_text_column(tree: &gtk::TreeView, pos: i32) {
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();

    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", pos);
    if pos == 1 {
        cell.set_wrap_width(247);
        cell.set_wrap_mode(pango::WrapMode::Char);
        column.set_expand(true);
    }
    tree.append_column(&column);
}

pub fn create_process_dialog(process: &sysinfo::Process, total_memory: u64) -> ProcDialog {
    let mut notebook = NoteBook::new();

    let popup = gtk::Window::new();

    popup.set_title(Some(&format!("Information about {}", process.name())));
    popup.set_transient_for(get_main_window().as_ref());
    popup.set_destroy_with_parent(true);

    //
    // PROCESS INFO TAB
    //
    let scroll = gtk::ScrolledWindow::new();
    let close_button = gtk::Button::with_label("Close");
    close_button.add_css_class("button-with-margin");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

    let running_since = process.run_time();

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);

    create_and_add_new_label(&labels, "name", process.name());
    create_and_add_new_label(&labels, "pid", &process.pid().to_string());
    let memory_peak = process.memory() * 1_000;
    let memory_usage =
        create_and_add_new_label(&labels, "memory usage", &format_number(memory_peak));
    let memory_peak_label =
        create_and_add_new_label(&labels, "memory usage peak", &format_number(memory_peak));
    let disk_peak = process.disk_usage();
    let disk_peak = disk_peak.written_bytes + disk_peak.read_bytes;
    let s;
    #[cfg(not(any(windows, target_os = "freebsd")))]
    {
        s = "disk I/O usage";
    }
    #[cfg(any(windows, target_os = "freebsd"))]
    {
        s = "I/O usage";
    }
    let disk_usage = create_and_add_new_label(&labels, s, &format_number(disk_peak));
    let disk_peak_label =
        create_and_add_new_label(&labels, &format!("{} peak", s), &format_number(disk_peak));
    let cpu_usage = create_and_add_new_label(
        &labels,
        "cpu usage",
        &format!("{:.1}%", process.cpu_usage()),
    );
    let run_time = create_and_add_new_label(&labels, "Running since", &format_time(running_since));
    create_and_add_new_label(
        &labels,
        "command",
        &format!(
            "[{}]",
            process
                .cmd()
                .iter()
                .map(|x| format!("\"{}\"", x))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    create_and_add_new_label(
        &labels,
        "executable path",
        &process.exe().display().to_string(),
    );
    let working_directory = create_and_add_new_label(
        &labels,
        "current working directory",
        &process.cwd().display().to_string(),
    );
    create_and_add_new_label(
        &labels,
        "root directory",
        &process.root().display().to_string(),
    );

    let env_tree = gtk::TreeView::new();
    let list_store = gtk::ListStore::new(&[glib::Type::STRING, glib::Type::STRING]);

    env_tree.set_headers_visible(false);
    env_tree.set_model(Some(&list_store));

    append_text_column(&env_tree, 0);
    append_text_column(&env_tree, 1);

    for env in process.environ() {
        let mut parts = env.splitn(2, '=');
        let name = match parts.next() {
            Some(n) => n,
            None => continue,
        };
        let value = parts.next().unwrap_or("");
        list_store.insert_with_values(None, &[(0, &name), (1, &value)]);
    }

    let components = gtk::Box::new(gtk::Orientation::Vertical, 0);
    components.append(&labels);

    if !process.environ().is_empty() {
        let label = gtk::Label::new(None);
        label.set_markup("<b>Environment variables</b>");

        components.append(&label);
        components.append(&env_tree);
    }

    scroll.set_child(Some(&components));
    scroll.set_hexpand(true);
    scroll.set_vexpand(true);

    vertical_layout.append(&scroll);
    vertical_layout.append(&close_button);

    notebook.create_tab("Information", &vertical_layout);

    //
    // GRAPH TAB
    //
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vertical_layout.set_spacing(5);
    vertical_layout.set_margin_top(10);
    vertical_layout.set_margin_bottom(10);
    vertical_layout.set_margin_start(5);
    vertical_layout.set_margin_end(5);
    let scroll = gtk::ScrolledWindow::new();
    let cpu_usage_history = GraphWidget::new(Some(100.), false); // In case a process uses more than 100%
    cpu_usage_history.set_display_labels(false);
    cpu_usage_history.set_minimum(Some(100.));

    let ram_usage_history = GraphWidget::new(Some(total_memory as f32), false);
    ram_usage_history.set_display_labels(false);
    ram_usage_history.set_overhead(Some(20.));

    let disk_usage_history = GraphWidget::new(Some(0f32), false);
    disk_usage_history.set_display_labels(false);
    disk_usage_history.set_overhead(Some(20.));

    cpu_usage_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "",
        None,
    );
    cpu_usage_history.set_labels_callback(Some(Box::new(|v| {
        if v > 100. {
            let nb = v.ceil() as u64;
            [
                nb.to_string(),
                (nb / 2).to_string(),
                "0".to_string(),
                "%".to_string(),
            ]
        } else {
            [
                "100".to_string(),
                "50".to_string(),
                "0".to_string(),
                "%".to_string(),
            ]
        }
    })));
    vertical_layout.append(&gtk::Label::new(Some("Process usage")));
    vertical_layout.append(&cpu_usage_history);
    cpu_usage_history.queue_draw();
    // let cpu_usage_history = connect_graph(cpu_usage_history);
    let cpu_usage_history = Rc::new(RefCell::new(cpu_usage_history));

    ram_usage_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "",
        None,
    );

    disk_usage_history.push(
        RotateVec::new(iter::repeat(0f32).take(61).collect()),
        "",
        None,
    );

    ram_usage_history.set_labels_callback(Some(Box::new(graph_label_units)));
    disk_usage_history.set_labels_callback(Some(Box::new(graph_label_units)));

    vertical_layout.append(&gtk::Label::new(Some("Memory usage")));
    vertical_layout.append(&ram_usage_history);
    ram_usage_history.queue_draw();
    // let ram_usage_history = connect_graph(ram_usage_history);
    let ram_usage_history = Rc::new(RefCell::new(ram_usage_history));

    #[cfg(not(windows))]
    {
        vertical_layout.append(&gtk::Label::new(Some("Disk I/O usage")));
    }
    #[cfg(windows)]
    {
        vertical_layout.append(&gtk::Label::new(Some("I/O usage")));
    }
    vertical_layout.append(&disk_usage_history);
    disk_usage_history.queue_draw();
    let disk_usage_history = Rc::new(RefCell::new(disk_usage_history));

    scroll.set_child(Some(&vertical_layout));
    scroll.connect_show(
        glib::clone!(@weak ram_usage_history, @weak cpu_usage_history, @weak disk_usage_history => move |_| {
            ram_usage_history.borrow().show();
            cpu_usage_history.borrow().show();
            disk_usage_history.borrow().show();
        }),
    );
    notebook.create_tab("Resources usage", &scroll);

    popup.set_child(Some(&notebook.notebook));
    popup.set_size_request(500, 600);

    close_button.connect_clicked(glib::clone!(@weak popup => move |_| {
        popup.close();
    }));
    let to_be_removed = Rc::new(Cell::new(false));
    popup.connect_destroy(glib::clone!(@weak to_be_removed => move |_| {
        to_be_removed.set(true);
    }));
    popup.connect_close_request(
        glib::clone!(@weak to_be_removed => @default-return Inhibit(false), move |_| {
            to_be_removed.set(true);
            Inhibit(false)
        }),
    );
    let event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(
        glib::clone!(@weak popup, @weak to_be_removed => @default-return Inhibit(false), move |_, key, _, _modifier| {
            if key == gtk::gdk::Key::Escape {
                popup.close();
                to_be_removed.set(true);
            }
            Inhibit(false)
        }),
    );
    popup.add_controller(&event_controller);
    popup.set_resizable(true);
    popup.show();

    let adjust = scroll.vadjustment();
    adjust.set_value(0.);
    scroll.set_vadjustment(Some(&adjust));

    ProcDialog {
        working_directory,
        memory_usage,
        disk_usage,
        cpu_usage,
        run_time,
        popup,
        pid: process.pid(),
        notebook,
        ram_usage_history,
        cpu_usage_history,
        disk_usage_history,
        memory_peak: RefCell::new(memory_peak),
        memory_peak_label,
        disk_peak: RefCell::new(disk_peak),
        disk_peak_label,
        is_dead: false,
        to_be_removed,
    }
}
