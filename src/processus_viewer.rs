#![crate_type = "bin"]

#![feature(convert)]

extern crate rgtk;
extern crate sysinfo;

use rgtk::*;
use rgtk::gtk::signals::DeleteEvent;
use rgtk::glib::Type;
use sysinfo::*;

fn append_column(title: &str, v: &mut Vec<gtk::TreeViewColumn>) {
    let l = v.len();
    let renderer = gtk::CellRendererText::new().unwrap();

    v.push(gtk::TreeViewColumn::new().unwrap());
    let tmp = v.get_mut(l).unwrap();

    tmp.set_title(title);
    tmp.pack_start(&renderer, true);
    tmp.add_attribute(&renderer, "text", l as i32);
}

fn create_and_fill_model(tree_store: &mut gtk::TreeStore, pid: i64, name: &str, cpu: f32, memory: u64) {
    let mut top_level = gtk::TreeIter::new().unwrap();

    tree_store.append(&mut top_level, None);
    tree_store.set_string(&top_level, 0, &format!("{}", pid));
    tree_store.set_string(&top_level, 1, name);
    tree_store.set_string(&top_level, 2, &format!("{}", cpu));
    tree_store.set_string(&top_level, 3, &format!("{}", memory));
}

fn main() {
    gtk::init();

    let mut window = gtk::Window::new(gtk::WindowType::TopLevel).unwrap();
    let mut sys = sysinfo::System::new();

    window.set_title("TreeView Sample");
    window.set_window_position(gtk::WindowPosition::Center);

    Connect::connect(&window, DeleteEvent::new(&mut |_| {
        gtk::main_quit();
        true
    }));

    let mut left_tree = gtk::TreeView::new().unwrap();
    let mut scroll = gtk::ScrolledWindow::new(None, None).unwrap();

    scroll.set_min_content_height(800);
    scroll.set_min_content_width(600);

    let mut columns : Vec<gtk::TreeViewColumn> = Vec::new();

    append_column("pid", &mut columns);
    append_column("process name", &mut columns);
    append_column("cpu usage", &mut columns);
    append_column("memory usage", &mut columns);

    for i in columns {
        left_tree.append_column(&i);
    }

    let mut tree_store = gtk::TreeStore::new(&[Type::String, Type::String, Type::String, Type::String]).unwrap();
    sys.refresh();
    for (_, pro) in sys.get_processus_list() {
        create_and_fill_model(&mut tree_store, pro.pid, &pro.name, pro.cpu_usage, pro.memory);
    }

    left_tree.set_model(&tree_store.get_model().unwrap());
    left_tree.set_headers_visible(true);
    scroll.add(&left_tree);

    /*for _ in 0..10 {
        let mut iter = gtk::TreeIter::new().unwrap();
        left_store.append(&mut iter);
        left_store.set_string(&iter, 0, "I'm in a list");
    }*/

    // display the panes

    //let mut split_pane = gtk::Box::new(gtk::Orientation::Horizontal, 10).unwrap();

    //split_pane.set_size_request(-1, -1);
    //split_pane.add(&left_tree);

    window.add(&scroll);
    window.show_all();
    gtk::main();
}
