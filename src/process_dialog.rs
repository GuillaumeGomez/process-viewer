use glib::object::Cast;
use glib::translate::ToGlibPtr;
use gtk::{self, BoxExt, ButtonSignals, ContainerExt, DialogExt, ScrolledWindowExt};
use gtk::{WidgetExt, WindowExt};
use gtk_sys;
use pango_sys::PangoWrapMode;
use sysinfo;

use std::time::{UNIX_EPOCH, Duration, SystemTime};

pub fn create_process_dialog(process: &sysinfo::Process, window: &gtk::Window) {
    let flags = gtk_sys::GTK_DIALOG_DESTROY_WITH_PARENT |
                gtk_sys::GTK_DIALOG_USE_HEADER_BAR;
    let scroll = gtk::ScrolledWindow::new(None, None);
    let close_button = gtk::Button::new_with_label("Close");
    let vertical_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    let popup = gtk::Dialog::new_with_buttons(Some(&format!("Information about {}", process.name)),
                                              Some(window),
                                              flags,
                                              &[]);
    let area = popup.get_content_area();
    let mut text = format!("name: {}\n\
                            pid: {}\n\
                            command: {}\n\
                            executable path: {}\n\
                            current working directory: {}\n\
                            root directory: {}\n\
                            memory usage: {} kB\n\
                            cpu usage: {}%\n\n\
                            Running since {} seconds\n\n\
                            environment:",
                            process.name,
                            process.pid,
                            process.cmd,
                            process.exe,
                            process.cwd,
                            process.root,
                            process.memory,
                            process.cpu_usage,
                            SystemTime::now().duration_since(UNIX_EPOCH + Duration::from_secs(process.start_time)).unwrap_or(Duration::from_secs(0)).as_secs());
    for env in process.environ.iter() {
        text.push_str(&format!("\n{:?}", env));
    }
    let label = gtk::Label::new(Some(&text));
    label.set_selectable(true);
    label.set_line_wrap(true);
    //label.set_line_wrap_mode(gtk::WrapMode::Char);
    unsafe { gtk_sys::gtk_label_set_line_wrap_mode(label.to_glib_none().0, PangoWrapMode::Char) };
    scroll.add(&label);
    vertical_layout.pack_start(&scroll, true, true, 0);
    vertical_layout.pack_start(&close_button, false, true, 0);
    vertical_layout.set_spacing(10);
    area.pack_start(&vertical_layout, true, true, 0);
    popup.set_size_request(400, 600);
    let popup = popup.upcast::<gtk::Window>();
    popup.set_resizable(false);
    popup.show_all();
    close_button.connect_clicked(move |_| {
        popup.destroy();
    });
}
