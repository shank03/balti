use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, WindowExt, notification::Notification, scroll::ScrollableElement,
};

use crate::{
    nav::BrowsePrefix,
    rt,
    s3::{self, S3Remote},
    ui::remote::{BrowseNav, BrowseNavEvent},
};

pub struct BrowseUi {
    browse_nav: Entity<BrowseNav>,
    s3_remote: S3Remote,
    prefix: SharedString,
    objects: Vec<s3::Object>,
    objects_scroll_handle: ScrollHandle,
    loading: bool,
}

impl BrowseUi {
    fn new(
        browse_nav: Entity<BrowseNav>,
        s3_remote: S3Remote,
        prefix: SharedString,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            browse_nav,
            s3_remote,
            prefix,
            objects: Vec::new(),
            objects_scroll_handle: ScrollHandle::new(),
            loading: false,
        }
    }

    pub fn view(
        browse_nav: Entity<BrowseNav>,
        s3_remote: S3Remote,
        prefix: SharedString,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let mut view = Self::new(browse_nav, s3_remote, prefix, window, cx);
            view.list_objects(window, cx);
            view
        })
    }

    fn list_objects(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.s3_remote.clone();
        let prefix = self.prefix.clone();

        let task = rt::spawn(cx, async move {
            s3::list_objects(remote, prefix.trim_start_matches('/')).await
        });

        cx.spawn_in(window, async move |this, cx| {
            let _ = this.update(cx, |this, cx| {
                this.loading = true;
                cx.notify();
            });

            let result = task.await.flatten();

            let _ = this.update_in(cx, |this, window, cx| {
                this.loading = false;

                match result {
                    Ok(objects) => this.objects = objects,
                    Err(err) => window.push_notification(
                        Notification::error(err.message).title("Failed to fetch objects"),
                        cx,
                    ),
                };

                cx.notify();
            });
        })
        .detach();
    }
}

impl BrowsePrefix for BrowseUi {
    fn name(&self) -> SharedString {
        if self.prefix == "/" {
            self.s3_remote.bucket_name.clone()
        } else {
            self.prefix
                .trim_matches('/')
                .split('/')
                .last()
                .map(|s| s.to_owned())
                .unwrap_or_default()
                .into()
        }
    }
}

impl Render for BrowseUi {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.prefix.clone())
            .size_full()
            .overflow_y_scroll()
            .when_else(
                self.loading,
                |this| {
                    this.flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(IconName::LoaderCircle).with_animation(
                            ElementId::CodeLocation(*std::panic::Location::caller()),
                            Animation::new(std::time::Duration::from_secs(2)).repeat(),
                            |el, delta| el.transform(Transformation::rotate(percentage(delta))),
                        ))
                },
                |this| {
                    this.child(
                        div()
                            .id(self.prefix.clone())
                            .p_2()
                            .flex()
                            .flex_col()
                            .size_full()
                            .pb_12()
                            .children(self.objects.iter().enumerate().map(|(i, object)| {
                                div()
                                    .id(SharedString::new(i.to_string()))
                                    .flex()
                                    .items_center()
                                    .w_full()
                                    .rounded_md()
                                    .p_2()
                                    .gap_4()
                                    .hover(|this| this.bg(cx.theme().secondary_hover))
                                    .child(match object {
                                        s3::Object::Folder(_) => Icon::new(IconName::Folder),
                                        s3::Object::File(_) => Icon::new(IconName::File),
                                    })
                                    .child(match object {
                                        s3::Object::Folder(prefix) => {
                                            prefix.replace(self.prefix.as_str(), "")
                                        }
                                        s3::Object::File(prefix) => {
                                            prefix.replace(self.prefix.as_str(), "")
                                        }
                                    })
                                    .map(|this| match object {
                                        s3::Object::Folder(shared_string) => {
                                            let prefix = shared_string.clone();
                                            this.on_click(cx.listener(
                                                move |this, _ev, _window, cx| {
                                                    this.browse_nav.update(cx, |_nav, cx| {
                                                        cx.emit(BrowseNavEvent(prefix.clone()));
                                                    });
                                                },
                                            ))
                                        }
                                        s3::Object::File(_) => this,
                                    })
                            }))
                            .overflow_y_scroll()
                            .track_scroll(&self.objects_scroll_handle),
                    )
                },
            )
            .vertical_scrollbar(&self.objects_scroll_handle)
    }
}
