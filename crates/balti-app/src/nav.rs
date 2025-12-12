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

pub trait BrowsePrefix: Render {
    fn name(&self) -> SharedString;
}

pub struct BucketNav {
    ptr: usize,
    views: HashMap<SharedString, AnyView>,
    stack: Vec<SharedString>,
}

impl BucketNav {
    pub fn new<N: BrowsePrefix, T: 'static>(view: Entity<N>, cx: &mut Context<T>) -> Self {
        let id = view.read(cx).name();

        let mut views = HashMap::new();
        views.insert(id.clone(), view.into());

        Self {
            ptr: 0,
            views: views,
            stack: vec![id],
        }
    }

    pub fn current_view(&self) -> Option<&AnyView> {
        self.stack
            .iter()
            .nth(self.ptr)
            .and_then(|s| self.views.get(s))
    }

    pub fn stack(&self) -> &Vec<SharedString> {
        &self.stack
    }

    pub fn push<N: BrowsePrefix, T: 'static>(&mut self, view: Entity<N>, cx: &mut Context<T>) {
        let id = view.read(cx).name();

        self.drop_later_and_views();

        self.stack.push(id.clone());
        self.views.insert(id, view.into());
        self.ptr = self.stack().len() - 1;

        cx.notify();
    }

    pub fn trim(&mut self, index: usize) {
        self.ptr = index;
        self.drop_later_and_views();
    }

    fn drop_later_and_views(&mut self) {
        // trim stack
        self.stack = self.stack.drain(..=self.ptr).collect();

        // drop views who's state not in stack
        let mut views = HashMap::new();
        for state in self.stack.iter() {
            if let Some(_) = views.get(state) {
                // we already have it
                continue;
            }

            // else save if it has view
            if let Some(view) = self.views.get(state) {
                views.insert(state.clone(), view.clone());
            }
        }

        self.views = views;
    }
}
