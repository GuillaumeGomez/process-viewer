use glib::object::Cast;
use glib::translate::ToGlibPtr;
use gtk::{self, BoxExt, ButtonExt, ContainerExt, DialogExt, ScrolledWindowExt};
use gtk::{WidgetExt, WindowExt};
use gtk_sys;
use pango_sys::PangoWrapMode;
use sysinfo;

fn fomat_time(t: u64) -> String {
    format!("{}{}{}{}s",
            {
                let days = t / 86400;
                if days > 0 {
                    format!("{}d ", days)
                } else {
                    "".to_owned()
                }
            },
            {
                let hours = t / 3600 % 60;
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

pub fn create_process_dialog(process: &sysinfo::Process, window: &gtk::Window,
                             start_time: u64, running_since: u64) {
    let flags = gtk::DIALOG_DESTROY_WITH_PARENT |
                gtk::DIALOG_USE_HEADER_BAR;
    let scroll = gtk::ScrolledWindow::new(None, None);
    let close_button = gtk::Button::new_with_label("Close");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    let popup = gtk::Dialog::new_with_buttons(Some(&format!("Information about {}", process.name)),
                                              Some(window),
                                              flags,
                                              &[]);
    let area = popup.get_content_area();
    let running_since = if start_time > process.start_time {
        start_time - process.start_time + running_since
    } else {
        start_time + running_since - process.start_time
    };
    let mut text = format!("name: {}\n\
                            pid: {}\n\
                            command: {:?}\n\
                            executable path: {}\n\
                            current working directory: {}\n\
                            root directory: {}\n\
                            memory usage: {} kB\n\
                            cpu usage: {}%\n\n\
                            Running since {}\n\n\
                            environment:",
                            process.name,
                            process.pid,
                            process.cmd,
                            process.exe,
                            process.cwd,
                            process.root,
                            process.memory,
                            process.cpu_usage,
                            fomat_time(running_since));
    for env in process.environ.iter() {
        text.push_str(&format!("\n{:?}", env));
    }
    let label = gtk::Label::new(text.as_str());
    label.set_selectable(true);
    label.set_line_wrap(true);
    //label.set_line_wrap_mode(gtk::WrapMode::Char);
    unsafe { gtk_sys::gtk_label_set_line_wrap_mode(label.to_glib_none().0, PangoWrapMode::Char) };
    scroll.add(&label);
    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);
    vertical_layout.set_spacing(10);
    area.pack_start(&vertical_layout, true, true, 0);
    popup.set_size_request(500, 700);
    let popup = popup.upcast::<gtk::Window>();
    popup.set_resizable(false);
    popup.show_all();
    close_button.connect_clicked(move |_| {
        popup.destroy();
    });
}
