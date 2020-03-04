use gtk::prelude::{
    CellLayoutExt, CellRendererTextExt, GtkListStoreExtManual, GtkWindowExt, TreeViewColumnExt,
    TreeViewExt, WidgetExt,
};
use gtk::{self, AdjustmentExt, BoxExt, ButtonExt, ContainerExt, LabelExt, ScrolledWindowExt};
use pango;
use sysinfo::{self, Pid, ProcessExt};

use std::cell::RefCell;
use std::fmt;
use std::iter;
use std::rc::Rc;

use graph::{Connecter, Graph};
use notebook::NoteBook;
use utils::{connect_graph, format_number, get_main_window, RotateVec};

#[allow(dead_code)]
pub struct ProcDialog {
    working_directory: gtk::Label,
    memory_usage: gtk::Label,
    cpu_usage: gtk::Label,
    run_time: gtk::Label,
    pub popup: gtk::Window,
    pub pid: Pid,
    notebook: NoteBook,
    ram_usage_history: Rc<RefCell<Graph>>,
    cpu_usage_history: Rc<RefCell<Graph>>,
    memory_peak: RefCell<u64>,
    memory_peak_label: gtk::Label,
    pub is_dead: bool,
}

impl fmt::Debug for ProcDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProcDialog {{ pid: {} }}", self.pid)
    }
}

impl ProcDialog {
    pub fn update(&self, process: &sysinfo::Process, start_time: u64) {
        if self.is_dead {
            return;
        }
        self.working_directory
            .set_text(&process.cwd().display().to_string());
        let memory = process.memory() << 10; // * 1024
        let memory_s = format_number(memory);
        self.memory_usage.set_text(&memory_s);
        if memory > *self.memory_peak.borrow() {
            *self.memory_peak.borrow_mut() = memory;
            self.memory_peak_label.set_text(&memory_s);
        }
        self.cpu_usage.set_text(&format!("{:.1}%", process.cpu_usage()));
        let running_since = compute_running_since(process, start_time);
        self.run_time.set_text(&format_time(running_since));

        let mut t = self.ram_usage_history.borrow_mut();
        t.data[0].move_start();
        *t.data[0].get_mut(0).expect("cannot get data 0") = process.memory() as f64;
        t.invalidate();
        let mut t = self.cpu_usage_history.borrow_mut();
        t.data[0].move_start();
        *t.data[0].get_mut(0).expect("cannot get data 0") = process.cpu_usage().into();
        t.invalidate();
    }

    pub fn set_dead(&mut self) {
        if self.is_dead {
            return;
        }
        self.is_dead = true;
        self.memory_usage.set_text("0");
        self.cpu_usage.set_text("0%");
        let s = format!("Ran for {}", self.run_time.get_text().unwrap_or("0s".into()));
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
    text.set_line_wrap(true);
    text.set_line_wrap_mode(pango::WrapMode::Char);

    horizontal_layout.add(&label);
    horizontal_layout.add(&text);
    scroll.add(&horizontal_layout);
    text
}

fn compute_running_since(process: &sysinfo::Process, running_since: u64) -> u64 {
    if running_since > process.start_time() {
        running_since - process.start_time()
    } else {
        process.start_time() - running_since
    }
}

fn append_text_column(tree: &gtk::TreeView, pos: i32) -> gtk::CellRendererText {
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();

    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", pos);
    if pos == 1 {
        cell.set_property_wrap_width(247);
        cell.set_property_wrap_mode(pango::WrapMode::Char);
        column.set_expand(true);
    }
    tree.append_column(&column);
    cell
}

pub fn create_process_dialog(
    process: &sysinfo::Process,
    start_time: u64,
    total_memory: u64,
) -> ProcDialog {
    let mut notebook = NoteBook::new();

    let popup = gtk::Window::new(gtk::WindowType::Toplevel);

    popup.set_title(&format!("Information about {}", process.name()));
    popup.set_transient_for(get_main_window().as_ref());
    popup.set_destroy_with_parent(true);

    //
    // PROCESS INFO TAB
    //
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    let close_button = gtk::Button::new_with_label("Close");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

    let running_since = compute_running_since(process, start_time);

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);

    create_and_add_new_label(&labels, "name", process.name());
    create_and_add_new_label(&labels, "pid", &process.pid().to_string());
    let memory_peak = process.memory() << 10; // * 1024
    let memory_usage =
        create_and_add_new_label(&labels, "memory usage", &format_number(memory_peak));
    let memory_peak_label =
        create_and_add_new_label(&labels, "memory usage peak", &format_number(memory_peak));
    let cpu_usage = create_and_add_new_label(
        &labels,
        "cpu usage",
        &format!("{:.1}%", process.cpu_usage()),
    );
    let run_time = create_and_add_new_label(&labels, "Running since", &format_time(running_since));
    create_and_add_new_label(&labels, "command", &format!("{:?}", process.cmd()));
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
    let list_store = gtk::ListStore::new(&[glib::Type::String, glib::Type::String]);

    env_tree.set_headers_visible(false);
    env_tree.set_model(Some(&list_store));

    append_text_column(&env_tree, 0);
    let cell = append_text_column(&env_tree, 1);

    env_tree.connect_size_allocate(move |tree, _| {
        if let Some(col) = tree.get_column(1) {
            cell.set_property_wrap_width(col.get_width() - 1);
        }
    });

    for env in process.environ() {
        let mut parts = env.splitn(2, '=');
        let name = match parts.next() {
            Some(n) => n,
            None => continue,
        };
        let value = parts.next().unwrap_or_else(|| "");
        list_store.insert_with_values(None, &[0, 1], &[&name, &value]);
    }

    let components = gtk::Box::new(gtk::Orientation::Vertical, 0);
    components.add(&labels);

    if !process.environ().is_empty() {
        let label = gtk::Label::new(None);
        label.set_markup("<b>Environment variables</b>");

        components.add(&label);
        components.pack_start(&env_tree, false, false, 0);
    }

    scroll.add(&components);

    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);

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
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    let mut cpu_usage_history = Graph::new(Some(100.), false); // In case a process uses more than 100%
    cpu_usage_history.set_display_labels(false);
    cpu_usage_history.set_minimum(Some(100.));

    let mut ram_usage_history = Graph::new(Some(total_memory as f64), false);
    ram_usage_history.set_display_labels(false);
    ram_usage_history.set_overhead(Some(20.));

    cpu_usage_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "",
        None,
    );
    cpu_usage_history.set_label_callbacks(Some(Box::new(|v| {
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
    vertical_layout.add(&gtk::Label::new(Some("Process usage")));
    cpu_usage_history.attach_to(&vertical_layout);
    cpu_usage_history.invalidate();
    let cpu_usage_history = connect_graph(cpu_usage_history);

    ram_usage_history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "",
        None,
    );
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
    vertical_layout.add(&gtk::Label::new(Some("Memory usage")));
    ram_usage_history.attach_to(&vertical_layout);
    ram_usage_history.invalidate();
    let ram_usage_history = connect_graph(ram_usage_history);

    scroll.add(&vertical_layout);
    scroll.connect_show(
        clone!(@weak ram_usage_history, @weak cpu_usage_history => move |_| {
            ram_usage_history.borrow().show_all();
            cpu_usage_history.borrow().show_all();
        }),
    );
    notebook.create_tab("Resources usage", &scroll);

    popup.add(&notebook.notebook);
    // To silence the annoying warning:
    // "(.:2257): Gtk-WARNING **: Allocating size to GtkWindow 0x7f8a31038290 without
    // calling gtk_widget_get_preferred_width/height(). How does the code know the size to
    // allocate?"
    popup.get_preferred_width();
    popup.set_size_request(500, 600);

    close_button.connect_clicked(clone!(@weak popup => move |_| {
        popup.destroy();
    }));
    popup.set_resizable(true);
    popup.show_all();

    if let Some(adjust) = scroll.get_vadjustment() {
        adjust.set_value(0.);
        scroll.set_vadjustment(Some(&adjust));
    }
    ram_usage_history.connect_to_window_events();
    cpu_usage_history.connect_to_window_events();

    ProcDialog {
        working_directory,
        memory_usage,
        cpu_usage,
        run_time,
        popup,
        pid: process.pid(),
        notebook,
        ram_usage_history,
        cpu_usage_history,
        memory_peak: RefCell::new(memory_peak),
        memory_peak_label,
        is_dead: false,
    }
}
