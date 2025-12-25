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

    pub fn new_tab<N: TabId>(&mut self, view: Entity<N>, cx: &mut Context<Self>) {
        let id = view.read(cx).id();

        match self.get_index_for_id(&id) {
            Some(index) => self.active_index = Some(index),
            None => {
                self.tabs.push(id.clone());
                self.views.insert(id, view.into());
                self.active_index = Some(self.tabs.len() - 1);
            }
        };
        cx.notify();
    }

    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }

        if let Some(active_index) = self.active_index {
            let id = self.tabs.remove(index);
            self.views.remove(&id);
            if index <= active_index {
                self.active_index = active_index.checked_sub(1);
            }
        }

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
    fn prefix(&self) -> SharedString;
}

pub struct BucketNav {
    ptr: usize,
    views: HashMap<SharedString, AnyView>,
    stack: Vec<(SharedString, SharedString)>,
}

impl BucketNav {
    pub fn new<N: BrowsePrefix, T: 'static>(view: Entity<N>, cx: &mut Context<T>) -> Self {
        let (name, prefix) = view.read_with(cx, |this, _cx| (this.name(), this.prefix()));

        let mut views = HashMap::new();
        views.insert(prefix.clone(), view.into());

        Self {
            ptr: 0,
            views: views,
            stack: vec![(name, prefix)],
        }
    }

    pub fn refresh_active_view<N: BrowsePrefix>(
        &mut self,
        mut f: impl FnMut(&SharedString) -> Entity<N>,
    ) {
        let prefix = self.stack.iter().nth(self.ptr);
        if let Some((_, prefix)) = prefix {
            let view = f(prefix);
            self.views.insert(prefix.clone(), view.into());
        }
    }

    pub fn current_view(&self) -> Option<&AnyView> {
        self.stack
            .iter()
            .nth(self.ptr)
            .and_then(|(_, prefix)| self.views.get(prefix))
    }

    pub fn stack(&self) -> &Vec<(SharedString, SharedString)> {
        &self.stack
    }

    pub fn push<N: BrowsePrefix, T: 'static>(&mut self, view: Entity<N>, cx: &mut Context<T>) {
        let (name, prefix) = view.read_with(cx, |this, _cx| (this.name(), this.prefix()));

        self.drop_later_and_views();

        self.stack.push((name, prefix.clone()));
        self.views.insert(prefix, view.into());
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
        for (_, prefix) in self.stack.iter() {
            if let Some(_) = views.get(prefix) {
                // we already have it
                continue;
            }

            // else save if it has view
            if let Some(view) = self.views.get(prefix) {
                views.insert(prefix.clone(), view.clone());
            }
        }

        self.views = views;
    }
}
