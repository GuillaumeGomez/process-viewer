use display_sysinfo::{self, show_if_necessary};
use glib::Cast;
use graph::Graph;
use gtk::{
    self, AdjustmentExt, BoxExt, ContainerExt, LabelExt, ScrolledWindowExt, ToggleButtonExt,
    WidgetExt,
};
use notebook::NoteBook;
use settings::Settings;
use sysinfo::{NetworkExt, SystemExt};
use utils::{connect_graph, format_number, RotateVec};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

struct NetworkData {
    name: String,
    history: Rc<RefCell<Graph>>,
    check_box: gtk::CheckButton,
    non_graph_layout: gtk::Box,
    updated: bool,
    // network in usage
    in_usage: gtk::Label,
    // network out usage
    out_usage: gtk::Label,
}

pub struct Network {
    elems: Rc<RefCell<Vec<NetworkData>>>,
    layout: gtk::Box,
}

impl Network {
    pub fn new(sys: &sysinfo::System, note: &mut NoteBook, settings: &Settings) -> Network {
        let mut elems = Vec::new();
        let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        for (interface_name, _) in sys.get_networks() {
            elems.push(create_network_interface(&layout, interface_name, &settings));
        }
        scroll.add(&layout);
        note.create_tab("Networks", &scroll);

        let elems = Rc::new(RefCell::new(elems));
        scroll.connect_show(clone!(@weak elems => move |_| {
                let elems = elems.borrow();
                for elem in elems.iter() {
                    show_if_necessary(
                        &elem.check_box.clone().upcast::<gtk::ToggleButton>(),
                        &elem.history.borrow(),
                        &elem.non_graph_layout,
                    );
                }
            }
        ));
        // It greatly improves the scrolling on the system information tab. No more clipping.
        if let Some(adjustment) = scroll.get_vadjustment() {
            adjustment.connect_value_changed(clone!(@weak elems => move |_| {
                let elems = elems.borrow();
                for elem in elems.iter() {
                    elem.history.borrow().invalidate();
                }
            }));
        }
        Network { elems, layout }
        // add button "refresh networks list"
    }

    // Maybe move the caller to a higher level?
    pub fn set_size_request(&self, width: i32, height: i32) {
        let elems = self.elems.borrow();
        for elem in elems.iter() {
            let history = elem.history.borrow();
            history.area.set_size_request(width, height);
        }
    }

    pub fn update_network(&mut self, sys: &sysinfo::System) {
        let mut networks = self.elems.borrow_mut();
        for (name, data) in sys.get_networks() {
            for network in networks.iter_mut().filter(|x| x.name == *name) {
                network
                    .in_usage
                    .set_text(format_number(data.get_income()).as_str());
                network
                    .out_usage
                    .set_text(format_number(data.get_outcome()).as_str());

                let mut history = network.history.borrow_mut();
                history.data[0].move_start();
                *history.data[0].get_mut(0).expect("cannot get data 0") = data.get_income() as f64;
                history.data[1].move_start();
                *history.data[1].get_mut(0).expect("cannot get data 1") = data.get_outcome() as f64;
                history.invalidate();
            }
        }
    }

    pub fn set_checkboxes_state(&self, active: bool) {
        let elems = self.elems.borrow();
        for elem in elems.iter() {
            elem.check_box.set_active(active);
        }
    }
}

fn create_network_interface(layout: &gtk::Box, name: &str, settings: &Settings) -> NetworkData {
    let mut history = Graph::new(Some(1.), false);
    history.set_overhead(Some(20.));
    history.set_label_callbacks(Some(Box::new(|v| {
        let v = v as u64;
        if v < 1000 {
            return [
                v.to_string(),
                (v >> 1).to_string(),
                "0".to_string(),
                "B/sec".to_string(),
            ];
        }
        let nb = v / 1000;
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
    history.set_labels_width(70);

    let non_graph_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let check_box = display_sysinfo::create_header(&name, &layout, settings.display_graph);
    // input data
    let in_usage = gtk::Label::new(Some(&format_number(0)));
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    horizontal_layout.pack_start(&gtk::Label::new(Some("Input data")), true, false, 0);
    horizontal_layout.pack_start(&in_usage, true, false, 0);
    horizontal_layout.set_homogeneous(true);
    non_graph_layout.add(&horizontal_layout);
    history.push(
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
    non_graph_layout.add(&horizontal_layout);
    history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "Output data",
        None,
    );
    layout.add(&non_graph_layout);
    history.attach_to(&layout);
    history.area.set_margin_bottom(20);

    let history = connect_graph(history);

    check_box
        .clone()
        .upcast::<gtk::ToggleButton>()
        .connect_toggled(clone!(@weak non_graph_layout, @weak history => move |c| {
            show_if_necessary(c, &history.borrow(), &non_graph_layout);
        }));
    NetworkData {
        name: name.to_owned(),
        history,
        check_box,
        non_graph_layout,
        in_usage,
        out_usage,
        updated: false,
    }
}
