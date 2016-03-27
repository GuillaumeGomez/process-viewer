use gtk;
use gtk::prelude::{BoxExt, WidgetExt};

pub struct NoteBook {
    pub notebook: gtk::Notebook,
    pub tabs: Vec<gtk::Box>,
}

impl NoteBook {
    pub fn new() -> NoteBook {
        NoteBook {
            notebook: gtk::Notebook::new(),
            tabs: Vec::new(),
        }
    }

    pub fn create_tab(&mut self, title: &str, widget: &gtk::Widget) -> Option<u32> {
        let label = gtk::Label::new(Some(title));
        let tab = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        tab.pack_start(&label, true, true, 0);
        tab.show_all();

        let index = self.notebook.append_page(widget, Some(&tab));
        self.tabs.push(tab);
        Some(index)
    }
}