use glib::IsA;
use gtk::prelude::{BoxExt, NotebookExtManual, WidgetExt};
use gtk::{Box, Label, Notebook, Orientation, Widget};

pub struct NoteBook {
    pub notebook: Notebook,
    pub tabs: Vec<Box>,
}

impl NoteBook {
    pub fn new() -> NoteBook {
        NoteBook {
            notebook: Notebook::new(),
            tabs: Vec::new(),
        }
    }

    pub fn create_tab<T: IsA<Widget>>(&mut self, title: &str, widget: &T) -> Option<u32> {
        let label = Label::new(Some(title));
        let tab = Box::new(Orientation::Horizontal, 0);

        tab.pack_start(&label, true, true, 0);
        tab.show_all();

        let index = self.notebook.append_page(widget, Some(&tab));
        self.tabs.push(tab);
        Some(index)
    }
}
