use std::collections::HashMap;

use gpui::*;

pub trait TabId: Render {
    fn id(&self) -> SharedString;
}

pub struct TabNav {
    active_index: Option<usize>,
    views: HashMap<SharedString, AnyView>,
    tabs: Vec<SharedString>,
}

impl TabNav {
    pub fn new() -> Self {
        Self {
            active_index: None,
            views: HashMap::new(),
            tabs: Vec::new(),
        }
    }

    pub fn active_index(&self) -> &Option<usize> {
        &self.active_index
    }

    pub fn tabs(&self) -> &Vec<SharedString> {
        &self.tabs
    }

    pub fn active_view(&self) -> Option<&AnyView> {
        self.active_index
            .and_then(|i| self.tabs.iter().nth(i))
            .and_then(|s| self.views.get(s))
    }

    pub fn select_tab<T: 'static>(&mut self, index: usize, cx: &mut Context<T>) {
        self.active_index = Some(index);
        cx.notify();
    }

    pub fn new_tab<N: TabId, T: 'static>(&mut self, view: Entity<N>, cx: &mut Context<T>) {
        let id = view.read(cx).id();

        match self.get_index_for_id(&id) {
            Some(index) => self.active_index = Some(index),
            None => {
                self.tabs.push(id.clone());
                self.active_index = Some(self.tabs.len() - 1);
                self.views.insert(id, view.into());
            }
        };
    }

    pub fn close_tab<T: 'static>(&mut self, id: SharedString, cx: &mut Context<T>) {
        match self.get_index_for_id(&id) {
            Some(index) => {
                if index < self.active_index.unwrap_or_default() {
                    self.active_index = self
                        .active_index
                        .map(|i| i.checked_sub(1).unwrap_or_default());
                }
                self.tabs.remove(index);
                self.views.remove(&id);
            }
            None => (),
        };

        cx.notify();
    }

    fn get_index_for_id(&self, id: &SharedString) -> Option<usize> {
        self.tabs
            .iter()
            .enumerate()
            .find(|(_, s)| s == &id)
            .map(|(i, _)| i)
    }
}
