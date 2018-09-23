use std::cell::RefCell;
use std::rc::Rc;

use notebook::NoteBook;
use utils::format_number;

use gtk::{self, BoxExt, ButtonExt, ContainerExt, GridExt, LabelExt, ProgressBarExt, WidgetExt};

use sysinfo::{self, DiskExt, SystemExt};

fn update_disk(label: &gtk::Label, p: &gtk::ProgressBar, disk: &sysinfo::Disk) {
    label.set_text(format!("{} mounted at \"{}\"",
                           disk.get_name()
                               .to_str()
                               .unwrap_or_else(|| ""),
                           disk.get_mount_point()
                               .to_str()
                               .unwrap_or_else(|| "")).as_str());
    p.set_text(Some(format!("{} / {}",
                            format_number(disk.get_total_space() - disk.get_available_space()),
                            format_number(disk.get_total_space())).as_str()));
    p.set_fraction(
        (disk.get_total_space() - disk.get_available_space()) as f64 /
        disk.get_total_space() as f64);
}

fn refresh_disks(grid: &gtk::Grid, disks: &[sysinfo::Disk],
                 grid_elems: &mut Vec<(gtk::Label, gtk::ProgressBar)>) {
    let mut done = 0;
    for (pos, disk) in disks.iter().enumerate() {
        if pos <= grid_elems.len() {
            let name = gtk::Label::new(None);

            let p = gtk::ProgressBar::new();
            p.set_show_text(true);

            grid.attach(&name, 0, pos as i32, 1, 1);
            grid.attach(&p, 1, pos as i32, 2, 1);
            grid_elems.push((name, p));
        }
        update_disk(&grid_elems[pos].0, &grid_elems[pos].1, disk);
        done += 1;
    }
    // A disk was removed so we need to remove it from the list.
    while grid_elems.len() > done {
        if let Some(elem) = grid_elems.pop() {
            grid.remove(&elem.0);
            grid.remove(&elem.1);
        } else {
            break
        }
    }
}

pub fn create_disk_info(sys: &Rc<RefCell<sysinfo::System>>, note: &mut NoteBook) {
    let grid_elems: Rc<RefCell<Vec<(gtk::Label, gtk::ProgressBar)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let scroll = gtk::ScrolledWindow::new(None, None);

    let grid = gtk::Grid::new();
    grid.set_column_homogeneous(true);
    grid.set_margin_right(5);
    grid.set_margin_left(5);
    grid.set_margin_top(10);
    grid.set_margin_bottom(5);

    let refresh_but = gtk::Button::new_with_label("Refresh");

    refresh_but.connect_clicked(clone!(sys, grid, grid_elems => move |_| {
        sys.borrow_mut().refresh_disks();
        refresh_disks(&grid, sys.borrow().get_disks(), &mut *grid_elems.borrow_mut());
    }));

    scroll.add(&grid);
    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&refresh_but, false, true, 0);

    note.create_tab("Disk information", &vertical_layout);
    refresh_disks(&grid, sys.borrow().get_disks(), &mut *grid_elems.borrow_mut());
}
