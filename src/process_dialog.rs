use glib::object::Cast;
use gtk::{self, BoxExt, ContainerExt, DialogExt, DialogSignals, WidgetExt, WindowExt};
use gtk_sys;
use sysinfo;

pub fn create_process_dialog(process: &sysinfo::Process, window: &gtk::Window) {
      let flags = gtk::DIALOG_MODAL | gtk_sys::GTK_DIALOG_DESTROY_WITH_PARENT |
                  gtk_sys::GTK_DIALOG_USE_HEADER_BAR;
      let scroll = gtk::ScrolledWindow::new(None, None);
      let popup = gtk::Dialog::new_with_buttons(Some(&format!("Information about {}",
                                                              process.name)),
                                                Some(window),
                                                flags,
                                                &[("Close", gtk_sys::GTK_RESPONSE_CLOSE as i32)]);
      let area = popup.get_content_area();
      let mut text = format!("name: {}\n\
                              pid: {}\n\
                              command: {}\n\
                              executable path: {}\n\
                              current working directory: {}\n\
                              root directory: {}\n\
                              memory usage: {} kB\n\
                              cpu usage: {}%\n\
                              Started at {} seconds (yes, not very useful for the moment...)\n\n\
                              environment:\n",
                              process.name,
                              process.pid,
                              process.cmd,
                              process.exe,
                              process.cwd,
                              process.root,
                              process.memory,
                              process.cpu_usage,
                              process.start_time);
      for env in process.environ.iter() {
            text.push_str(&format!("{}\n", env));
      }
      let label = gtk::Label::new(Some(&text));
      label.set_selectable(true);
      scroll.add(&label);
      area.pack_start(&scroll, true, true, 0);
      popup.set_size_request(400, 600);
      popup.clone().upcast::<gtk::Window>().set_resizable(false);
      popup.show_all();
      popup.connect_response(|w, _| w.destroy());
}
