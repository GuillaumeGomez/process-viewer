use std::cell::RefCell;
use std::rc::Rc;

use notebook::NoteBook;
use utils::format_number;

use gtk::{self, BoxExt, ButtonExt, ContainerExt, LabelExt, ProgressBarExt};

use sysinfo::{self, DiskExt, SystemExt};

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
            disk.get_name().to_str().unwrap_or_else(|| ""),
            &info.mount_point,
        )
        .as_str(),
    );
    info.progress.set_text(Some(
        format!(
            "{} / {}",
            format_number(disk.get_total_space() - disk.get_available_space()),
            format_number(disk.get_total_space())
        )
        .as_str(),
    ));
    info.progress.set_fraction(
        (disk.get_total_space() - disk.get_available_space()) as f64
            / disk.get_total_space() as f64,
    );
    info.updated = true;
}

fn refresh_disks(container: &gtk::Box, disks: &[sysinfo::Disk], elems: &mut Vec<DiskInfo>) {
    for disk in disks.iter() {
        let mount_point = disk.get_mount_point().to_str().unwrap_or_else(|| "");
        update_disk(
            if let Some(entry) = elems.iter_mut().find(|e| e.mount_point == mount_point) {
                entry
            } else {
                let label = gtk::LabelBuilder::new()
                    .margin_top(if elems.is_empty() { 8 } else { 20 })
                    .build();

                let progress = gtk::ProgressBar::new();
                progress.set_show_text(true);

                container.add(&label);
                container.add(&progress);
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

pub fn create_disk_info(sys: &Rc<RefCell<sysinfo::System>>, note: &mut NoteBook) {
    let elems: Rc<RefCell<Vec<DiskInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let refresh_but = gtk::Button::new_with_label("Refresh disks");

    refresh_but.connect_clicked(
        clone!(@weak sys, @weak container, @strong elems => move |_| {
            sys.borrow_mut().refresh_disks();
            refresh_disks(&container, sys.borrow().get_disks(), &mut *elems.borrow_mut());
        }),
    );

    scroll.add(&container);
    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&refresh_but, false, true, 0);

    note.create_tab("Disk information", &vertical_layout);
    refresh_disks(
        &container,
        sys.borrow().get_disks(),
        &mut *elems.borrow_mut(),
    );
}
