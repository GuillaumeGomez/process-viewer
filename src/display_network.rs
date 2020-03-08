use display_sysinfo::show_if_necessary;
use graph::Graph;
use gtk::{
    self, AdjustmentExt, BoxExt, ButtonExt, ContainerExt, GridExt, LabelExt, ScrolledWindowExt,
    ToggleButtonExt, WidgetExt,
};
use notebook::NoteBook;
use settings::Settings;
use sysinfo::{NetworkExt, SystemExt};
use utils::{connect_graph, format_number, RotateVec};

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

// TODO:
// change this entire tab into a treeview (to allow to sort columns and filter)
// double-clicking on a network will open a dialog specifically for it

struct NetworkData {
    name: String,
    history: Rc<RefCell<Graph>>,
    check_box: gtk::CheckButton,
    non_graph_layout: gtk::Box,
    updated: bool,
    container: gtk::Box,
    // network in usage
    in_usage: gtk::Label,
    // network out usage
    out_usage: gtk::Label,
    // network income packets
    income_packets: gtk::Label,
    // network outcome packets
    outcome_packets: gtk::Label,
    // network income errors
    income_errors: gtk::Label,
    // network outcome errors
    outcome_errors: gtk::Label,
}

pub struct Network {
    elems: Rc<RefCell<Vec<NetworkData>>>,
}

impl Network {
    pub fn new(
        sys: &Rc<RefCell<sysinfo::System>>,
        note: &mut NoteBook,
        settings: &Rc<RefCell<Settings>>,
    ) -> Network {
        let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let mut elems = Vec::new();
        update_network(&mut elems, &sys.borrow(), &layout, &settings.borrow());
        let elems = Rc::new(RefCell::new(elems));
        scroll.connect_show(clone!(@weak elems => move |_| {
                let elems = elems.borrow();
                for elem in elems.iter() {
                    show_if_necessary(
                        &elem.check_box,
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
        let refresh_but = gtk::Button::new_with_label("Refresh network interfaces list");

        refresh_but.connect_clicked(
            clone!(@weak sys, @weak elems, @weak layout, @weak settings => move |_| {
                sys.borrow_mut().refresh_networks_list();
                update_network(&mut elems.borrow_mut(), &sys.borrow(), &layout, &settings.borrow());
                // refresh_networks(&container, sys.borrow().get_disks(), &mut *elems.borrow_mut());
            }),
        );

        scroll.add(&layout);

        let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

        vertical_layout.pack_start(&scroll, true, true, 0);
        vertical_layout.pack_start(&refresh_but, false, true, 0);

        note.create_tab("Networks", &vertical_layout);

        Network { elems }
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

                network
                    .income_packets
                    .set_text(&better_number(data.get_total_packets_income()));
                network
                    .outcome_packets
                    .set_text(&better_number(data.get_total_packets_outcome()));
                network
                    .income_errors
                    .set_text(&better_number(data.get_total_errors_income()));
                network
                    .outcome_errors
                    .set_text(&better_number(data.get_total_errors_outcome()));
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

fn update_network(
    interfaces: &mut Vec<NetworkData>,
    sys: &sysinfo::System,
    layout: &gtk::Box,
    settings: &Settings,
) {
    for (interface_name, _) in sys.get_networks() {
        if let Some(item) = interfaces.iter_mut().find(|x| x.name == *interface_name) {
            item.updated = true;
        } else {
            interfaces.push(create_network_interface(
                &layout,
                &interface_name,
                &settings,
            ));
        }
    }
    interfaces.retain(|x| {
        if !x.updated {
            layout.remove(&x.container);
        }
        x.updated
    });
    interfaces.sort_unstable_by(|a, b| {
        a.name
            .partial_cmp(&b.name)
            .expect("string comparison failed")
    });
    for (pos, interface) in interfaces.iter_mut().enumerate() {
        interface.updated = false;
        layout.reorder_child(&interface.container, pos as _);
    }
}

fn create_non_graph_labels(
    label_text: &str,
    text: &str,
    non_graph_layout: &gtk::Box,
) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    horizontal_layout.pack_start(&gtk::Label::new(Some(label_text)), true, false, 0);
    horizontal_layout.pack_start(&label, true, false, 0);
    horizontal_layout.set_homogeneous(true);
    non_graph_layout.add(&horizontal_layout);
    label
}

fn better_number(mut f: u64) -> String {
    if f < 1000 {
        f.to_string()
    } else {
        let mut s = String::new();
        let mut count = 0;
        while f > 0 {
            if !s.is_empty() && count % 3 == 0 {
                s.push(' ');
            }
            s.push((((f % 10) as u8) + b'0') as char);
            f /= 10;
            count += 1;
        }
        {
            let vec = unsafe { s.as_mut_vec() };
            vec.reverse();
        }
        s
    }
}

fn create_header(
    label_text: &str,
    parent_layout: &gtk::Box,
    display_graph: bool,
    hide_everything: Rc<RefCell<bool>>,
    history: &Rc<RefCell<Graph>>,
    non_graph_layout: &gtk::Box,
) -> gtk::CheckButton {
    let check_box = gtk::CheckButton::new_with_label("Graph view");
    check_box.set_active(display_graph);

    let label = gtk::Label::new(None);
    label.set_markup(label_text);

    let empty = gtk::Label::new(None);
    let grid = gtk::Grid::new();
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    horizontal_layout.pack_start(&gtk::Label::new(None), true, true, 0);
    horizontal_layout.pack_start(&check_box, false, false, 0);
    
    let hide_button = gtk::Button::new_with_label("-");

    hide_button.connect_clicked(clone!(@weak hide_everything, @weak check_box, @weak history, @weak non_graph_layout => move |hide_button| {
        let show_necessary = if hide_button.get_label().map(|x| x == "+").unwrap_or(false) {
            hide_button.set_label("-");
            *hide_everything.borrow_mut() = false;
            true
        } else {
            hide_button.set_label("+");
            *hide_everything.borrow_mut() = true;
            false
        };
        if show_necessary {
            show_if_necessary(&check_box, &history.borrow(), &non_graph_layout);
        } else {
            non_graph_layout.hide();
            history.borrow().hide();
        }
    }));

    grid.attach(&hide_button, 0, 0, 1, 1);
    grid.attach_next_to(&empty, Some(&hide_button), gtk::PositionType::Right, 2, 1);
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
        .connect_toggled(clone!(@weak non_graph_layout, @weak history => move |c| {
            if !*hide_everything.borrow() {
                show_if_necessary(c, &history.borrow(), &non_graph_layout);
            }
        }));
    check_box
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

    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let non_graph_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

    history.area.set_margin_bottom(20);

    // input data
    let in_usage = create_non_graph_labels("Input data", &format_number(0), &non_graph_layout);
    history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "Input data",
        None,
    );
    // output data
    let out_usage = create_non_graph_labels("Output data", &format_number(0), &non_graph_layout);
    history.push(
        RotateVec::new(iter::repeat(0f64).take(61).collect()),
        "Output data",
        None,
    );

    let history = connect_graph(history);
    let hide_everything = Rc::new(RefCell::new(false));

    let check_box = create_header(
        &format!("<b>{}</b>", name),
        &container,
        settings.display_graph,
        hide_everything,
        &history,
        &non_graph_layout,
    );

    container.add(&non_graph_layout);
    {
        history.borrow().attach_to(&container);
    }

    layout.add(&container);

    // packets
    let income_packets = create_non_graph_labels("Total income packets", "0", &non_graph_layout);
    let outcome_packets = create_non_graph_labels("Total outcome packets", "0", &non_graph_layout);
    // errors
    let income_errors = create_non_graph_labels("Total income errors", "0", &non_graph_layout);
    let outcome_errors = create_non_graph_labels("Total outcome errors", "0", &non_graph_layout);

    NetworkData {
        name: name.to_owned(),
        history,
        check_box,
        non_graph_layout,
        in_usage,
        out_usage,
        updated: true,
        income_packets,
        outcome_packets,
        income_errors,
        outcome_errors,
        container,
    }
}
