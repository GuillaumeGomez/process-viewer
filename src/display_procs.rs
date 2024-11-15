use gtk::glib;
use gtk::prelude::*;

use sysinfo::{Pid, Process};

use crate::utils::format_number;

use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::rc::Rc;

#[allow(dead_code)]
pub struct Procs {
    pub left_tree: gtk::TreeView,
    pub scroll: gtk::ScrolledWindow,
    pub current_pid: Rc<Cell<Option<Pid>>>,
    pub kill_button: gtk::Button,
    pub info_button: gtk::Button,
    pub vertical_layout: gtk::Box,
    pub list_store: gtk::ListStore,
    pub columns: Vec<gtk::TreeViewColumn>,
    pub filter_entry: gtk::SearchEntry,
    pub search_bar: gtk::SearchBar,
}

impl Procs {
    pub fn new(proc_list: &HashMap<Pid, Process>, stack: &gtk::Stack) -> Procs {
        let left_tree = gtk::TreeView::builder().headers_visible(true).build();
        let scroll = gtk::ScrolledWindow::builder().child(&left_tree).build();
        let current_pid = Rc::new(Cell::new(None));
        let kill_button = gtk::Button::builder()
            .label("End task")
            .hexpand(true)
            .margin_top(6)
            .margin_bottom(6)
            .margin_end(6)
            .sensitive(false)
            .build();
        let info_button = gtk::Button::builder()
            .label("More information")
            .hexpand(true)
            .margin_top(6)
            .margin_bottom(6)
            .margin_end(6)
            .margin_start(6)
            .sensitive(false)
            .build();

        let overlay = gtk::Overlay::builder()
            .child(&scroll)
            .hexpand(true)
            .vexpand(true)
            .build();
        let filter_entry = gtk::SearchEntry::new();
        let search_bar = gtk::SearchBar::builder()
            .halign(gtk::Align::End)
            .valign(gtk::Align::End)
            .show_close_button(true)
            .child(&filter_entry)
            .build();

        // We put the filter entry at the right bottom.
        overlay.add_overlay(&search_bar);

        let mut columns: Vec<gtk::TreeViewColumn> = Vec::new();

        let list_store = gtk::ListStore::new(&[
            // The first four columns of the model are going to be visible in the view.
            glib::Type::U32,    // pid
            glib::Type::STRING, // name
            glib::Type::STRING, // CPU
            glib::Type::STRING, // mem
            glib::Type::STRING, // disk I/O
            // These two will serve as keys when sorting by process name and CPU usage.
            glib::Type::STRING, // name_lowercase
            glib::Type::F32,    // CPU_f32
            glib::Type::U64,    // mem
            glib::Type::U64,    // disk I/O
        ]);

        for pro in proc_list.values() {
            if let Some(exe) = pro
                .exe()
                .and_then(|exe| exe.file_name())
                .or_else(|| Some(pro.name()))
            {
                create_and_fill_model(
                    &list_store,
                    pro.pid().as_u32(),
                    pro.cmd(),
                    exe,
                    pro.cpu_usage(),
                    pro.memory(),
                );
            }
        }

        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 6);

        left_tree.connect_cursor_changed(
            glib::clone!(@strong current_pid, @weak kill_button, @weak info_button => move |tree_view| {
                let selection = tree_view.selection();
                let (pid, ret) = if let Some((model, iter)) = selection.selected() {
                    if let Ok(x) = model.get_value(&iter, 0).get::<u32>() {
                        (Some(Pid::from_u32(x)), true)
                    } else {
                        (None, false)
                    }
                } else {
                    (None, false)
                };
                current_pid.set(pid);
                kill_button.set_sensitive(ret);
                info_button.set_sensitive(ret);
            }),
        );
        vertical_layout.append(&overlay);
        horizontal_layout.append(&info_button);
        horizontal_layout.append(&kill_button);
        vertical_layout.append(&horizontal_layout);

        // The filter part.
        let filter_model = gtk::TreeModelFilter::new(&list_store, None);
        filter_model.set_visible_func(
            glib::clone!(@weak filter_entry => @default-return false, move |model, iter| {
                if !WidgetExt::is_visible(&filter_entry) {
                    return true;
                }
                let text = filter_entry.text();
                if text.is_empty() {
                    return true;
                }
                let text: &str = text.as_ref();
                // TODO: Maybe add an option to make searches case sensitive?
                let pid = model.get_value(iter, 0)
                               .get::<u32>()
                               .map(|p| p.to_string())
                               .ok()
                               .unwrap_or_default();
                let name = model.get_value(iter, 1)
                                .get::<String>()
                                .map(|s| s.to_lowercase())
                                .ok()
                                .unwrap_or_default();
                pid.contains(text) ||
                text.contains(&pid) ||
                name.contains(text) ||
                text.contains(&name)
            }),
        );
        // For the filtering to be taken into account, we need to add it directly into the
        // "global" model.
        let sort_model = gtk::TreeModelSort::with_model(&filter_model);
        left_tree.set_model(Some(&sort_model));
        left_tree.set_search_entry(Some(&filter_entry));

        append_column("pid", &mut columns, &left_tree, None);
        append_column("process name", &mut columns, &left_tree, Some(200));
        append_column("cpu usage", &mut columns, &left_tree, None);
        append_column("memory usage", &mut columns, &left_tree, None);
        #[cfg(not(windows))]
        {
            append_column("disk I/O usage", &mut columns, &left_tree, None);
        }
        #[cfg(windows)]
        {
            append_column("I/O usage", &mut columns, &left_tree, None);
        }

        // When we click the "name" column the order is defined by the
        // "name_lowercase" effectively making the built-in comparator ignore case.
        columns[1].set_sort_column_id(5);
        // Likewise clicking the "CPU" column sorts by the "CPU_f32" one because
        // we want the order to be numerical not lexicographical.
        columns[2].set_sort_column_id(6);
        // The memory usage display has been improved, so to make efficient sort,
        // we have to separate the display and the actual number.
        columns[3].set_sort_column_id(7);
        // The disk I/O usage display has been improved, so to make efficient sort,
        // we have to separate the display and the actual number.
        columns[4].set_sort_column_id(8);

        filter_entry.connect_search_changed(move |_| {
            filter_model.refilter();
        });

        // Sort by CPU usage by default.
        sort_model.set_sort_column_id(gtk::SortColumn::Index(6), gtk::SortType::Descending);

        stack.add_titled(&vertical_layout, Some("Processes"), "Processes");

        Procs {
            left_tree,
            scroll,
            current_pid,
            kill_button,
            info_button,
            vertical_layout: vertical_layout
                .downcast::<gtk::Box>()
                .expect("downcast failed"),
            list_store,
            columns,
            filter_entry,
            search_bar,
        }
    }
}

fn append_column(
    title: &str,
    v: &mut Vec<gtk::TreeViewColumn>,
    left_tree: &gtk::TreeView,
    max_width: Option<i32>,
) {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    if title != "process name" {
        renderer.set_xalign(1.0);
    }

    let column = gtk::TreeViewColumn::builder()
        .title(title)
        .resizable(true)
        .min_width(10)
        .clickable(true)
        .sort_column_id(id)
        .build();

    if let Some(max_width) = max_width {
        column.set_max_width(max_width);
        column.set_expand(true);
    }
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    left_tree.append_column(&column);
    v.push(column);
}

pub fn create_and_fill_model(
    list_store: &gtk::ListStore,
    pid: u32,
    cmdline: &[OsString],
    name: &OsStr,
    cpu: f32,
    memory: u64,
) {
    let name = if name.is_empty() {
        let Some(cmd) = cmdline
            .iter()
            .map(|c| c.to_string_lossy().to_string())
            .next()
        else {
            return;
        };
        cmd
    } else {
        name.to_string_lossy().to_string()
    };
    list_store.insert_with_values(
        None,
        &[
            (0, &pid),
            (1, &name),
            (2, &format!("{:.1}", cpu)),
            (3, &format_number(memory)),
            (4, &String::new()),
            (5, &name.to_lowercase()),
            (6, &cpu),
            (7, &memory),
            (8, &0),
        ],
    );
}
