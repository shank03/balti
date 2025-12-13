use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, WindowExt, notification::Notification, scroll::ScrollableElement,
};

use crate::{
    err::AppError,
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
    error: Option<AppError>,
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
            error: None,
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
                    Err(err) => {
                        window.push_notification(
                            Notification::error(&err.message).title("Failed to fetch objects"),
                            cx,
                        );
                        this.error = Some(err);
                    }
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

    fn prefix(&self) -> SharedString {
        self.prefix.clone()
    }
}

impl Render for BrowseUi {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.prefix.clone())
            .size_full()
            .mt_11()
            .overflow_scroll()
            .when_some(self.error.clone(), |this, error| {
                this.child(self.render_error(error.message, cx))
            })
            .when_none(&self.error.clone(), |this| {
                this.when_else(
                    self.loading,
                    |this| {
                        this.flex()
                            .size_full()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::LoaderCircle).size_8().with_animation(
                                ElementId::CodeLocation(*std::panic::Location::caller()),
                                Animation::new(std::time::Duration::from_secs(1)).repeat(),
                                |el, delta| el.transform(Transformation::rotate(percentage(delta))),
                            ))
                    },
                    |this| this.child(self.render_object_item_list(cx)),
                )
                .vertical_scrollbar(&self.objects_scroll_handle)
                .horizontal_scrollbar(&self.objects_scroll_handle)
            })
    }
}

impl BrowseUi {
    fn render_error(&mut self, message: String, cx: &mut Context<Self>) -> impl IntoElement {
        div().p_2().child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .items_center()
                .justify_center()
                .border_color(cx.theme().sidebar_border)
                .border_1()
                .border_dashed()
                .rounded_lg()
                .p_4()
                .gap_2()
                .child(
                    div().rounded_md().p_2().bg(cx.theme().danger).child(
                        Icon::new(IconName::TriangleAlert)
                            .size_5()
                            .text_color(cx.theme().danger_foreground),
                    ),
                )
                .child(div().text_lg().child("Failed to fetch objects"))
                .child(div().child(message)),
        )
    }

    fn render_object_item_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .gap_4()
                    .items_center()
                    .justify_between()
                    .rounded_md()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().sidebar_border)
                    .text_sm()
                    .hover(|this| this.bg(cx.theme().secondary_hover))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(match object {
                                s3::Object::Folder(_) => Icon::new(IconName::Folder),
                                s3::Object::File { .. } => {
                                    Icon::empty().path("icons/file-digit.svg")
                                }
                            })
                            .text_sm()
                            .child(match object {
                                s3::Object::Folder(prefix) => {
                                    prefix.replace(self.prefix.as_str(), "")
                                }
                                s3::Object::File { key, .. } => {
                                    key.replace(self.prefix.as_str(), "")
                                }
                            }),
                    )
                    .child(
                        div().flex().flex_shrink_0().gap_4().items_center().map(
                            |this| match object {
                                s3::Object::Folder(_) => this,
                                s3::Object::File {
                                    size,
                                    last_modified,
                                    ..
                                } => this
                                    .text_color(cx.theme().muted_foreground)
                                    .child(
                                        div().font_family("JetBrains Mono").child(size.to_string()),
                                    )
                                    .child(last_modified.clone().unwrap_or_default()),
                            },
                        ),
                    )
                    .map(|this| match object {
                        s3::Object::Folder(shared_string) => {
                            let prefix = shared_string.clone();
                            this.on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.browse_nav.update(cx, |_nav, cx| {
                                    cx.emit(BrowseNavEvent(prefix.clone()));
                                });
                            }))
                        }
                        s3::Object::File { .. } => this,
                    })
            }))
            .overflow_y_scroll()
            .track_scroll(&self.objects_scroll_handle)
    }
}
