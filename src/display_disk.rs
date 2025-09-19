use std::cell::RefCell;
use std::rc::Rc;

use crate::utils::format_number;

use gtk::glib;
use gtk::prelude::*;

struct DiskInfo {
    label: gtk::Label,
    progress: gtk::ProgressBar,
    mount_point: String,
    updated: bool,
}

fn update_disk(info: &mut DiskInfo, disk: &sysinfo::Disk) {
    info.label.set_text(
        format!(
            "{} mounted on \"{}\"",
            disk.name().to_str().unwrap_or(""),
            &info.mount_point,
        )
        .as_str(),
    );
    info.progress.set_text(Some(
        format!(
            "{} / {}",
            format_number(disk.total_space() - disk.available_space()),
            format_number(disk.total_space())
        )
        .as_str(),
    ));
    info.progress.set_fraction(
        (disk.total_space() - disk.available_space()) as f64 / disk.total_space() as f64,
    );
    info.updated = true;
}

fn refresh_disks(container: &gtk::Box, disks: &[sysinfo::Disk], elems: &mut Vec<DiskInfo>) {
    for disk in disks.iter() {
        let mount_point = disk.mount_point().to_str().unwrap_or("");
        update_disk(
            if let Some(entry) = elems.iter_mut().find(|e| e.mount_point == mount_point) {
                entry
            } else {
                let label = gtk::Label::builder()
                    .margin_top(if elems.is_empty() { 8 } else { 20 })
                    .build();

                let progress = gtk::ProgressBar::new();
                progress.set_show_text(true);

                container.append(&label);
                container.append(&progress);
                elems.push(DiskInfo {
                    label,
                    progress,
                    mount_point: mount_point.to_owned(),
                    updated: false,
                });
                elems.last_mut().unwrap()
            },
            disk,
        );
    }
    for entry in elems.iter().filter(|e| !e.updated) {
        container.remove(&entry.label);
        container.remove(&entry.progress);
    }
    elems.retain(|e| e.updated);
    for entry in elems.iter_mut() {
        entry.updated = false;
    }
}

pub fn create_disk_info(stack: &gtk::Stack) {
    let elems: Rc<RefCell<Vec<DiskInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&container)
        .build();

    let disks = RefCell::new(sysinfo::Disks::new_with_refreshed_list());

    refresh_disks(&container, &disks.borrow(), &mut elems.borrow_mut());

    let refresh_but = gtk::Button::with_label("Refresh disks");

    refresh_but.connect_clicked(glib::clone!(
        #[weak]
        container,
        #[strong]
        elems,
        move |_| {
            disks.borrow_mut().refresh(true);
            refresh_disks(&container, &disks.borrow(), &mut elems.borrow_mut());
        }
    ));

    vertical_layout.append(&scroll);
    vertical_layout.append(&refresh_but);

    stack.add_titled(&vertical_layout, Some("Disks"), "Disks");
}
