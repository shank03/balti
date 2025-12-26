use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
};

use crate::{
    nav::{BucketNav, TabId},
    s3::S3Remote,
    ui::browse::BrowseUi,
};

pub enum BrowseNavEvent {
    CreateFolder(SharedString),
    UploadFiles(SharedString),
    NewView(SharedString),
}
pub struct BrowseNav;
impl EventEmitter<BrowseNavEvent> for BrowseNav {}

pub struct RemoteUi {
    s3_remote: S3Remote,
    nav: Entity<BucketNav>,
    browse_nav: Entity<BrowseNav>,
    header_scroll_handle: ScrollHandle,
    _subcriptions: Vec<Subscription>,
}

impl RemoteUi {
    fn new(s3_remote: S3Remote, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let browse_nav = cx.new(|_| BrowseNav {});

        let nav_sub = cx.subscribe_in(&browse_nav, window, |this, _entity, event, window, cx| {
            if let BrowseNavEvent::NewView(prefix) = event {
                this.nav.update(cx, |nav, cx| {
                    nav.push(
                        BrowseUi::view(
                            this.browse_nav.clone(),
                            this.s3_remote.clone(),
                            prefix.clone(),
                            window,
                            cx,
                        ),
                        cx,
                    );
                });
            }
        });

        let nav = cx.new(|cx| {
            BucketNav::new(
                BrowseUi::view(
                    browse_nav.clone(),
                    s3_remote.clone(),
                    SharedString::new_static("/"),
                    window,
                    cx,
                ),
                cx,
            )
        });

        Self {
            s3_remote,
            nav,
            browse_nav,
            header_scroll_handle: ScrollHandle::new(),
            _subcriptions: vec![nav_sub],
        }
    }

    pub fn view(s3_remote: S3Remote, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(s3_remote, window, cx))
    }
}

impl TabId for RemoteUi {
    fn id(&self) -> SharedString {
        self.s3_remote.remote_name.clone()
    }
}

impl Render for RemoteUi {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stack = self.nav.read(cx).stack();
        let len = stack.len();

        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .flex()
                    .items_center()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .border_b_1()
                    .text_sm()
                    .border_color(cx.theme().sidebar_border)
                    .child(
                        Button::new("refresh")
                            .icon(Icon::empty().path("icons/rotate-ccw.svg"))
                            .small()
                            .ghost()
                            .on_click(cx.listener(move |this, _ev, window, cx| {
                                this.nav.update(cx, |nav, cx| {
                                    nav.refresh_active_view(|prefix| {
                                        BrowseUi::view(
                                            this.browse_nav.clone(),
                                            this.s3_remote.clone(),
                                            prefix.clone(),
                                            window,
                                            cx,
                                        )
                                    });
                                    cx.notify();
                                });
                            })),
                    )
                    .child(
                        div()
                            .id("header")
                            .flex()
                            .w_full()
                            .overflow_x_scroll()
                            .pr(px(56.))
                            .track_scroll(&self.header_scroll_handle)
                            .gap_1()
                            .children(stack.into_iter().cloned().enumerate().map(
                                |(i, (name, _))| {
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new(SharedString::new(i.to_string()))
                                                .label(name.trim_matches('/').to_owned())
                                                .ghost()
                                                .small()
                                                .px_1()
                                                .map(|this| {
                                                    if i == len - 1 {
                                                        this.bg(cx.theme().primary.opacity(0.2))
                                                            .border_1()
                                                            .border_color(cx.theme().primary)
                                                            .font_medium()
                                                    } else {
                                                        this
                                                    }
                                                })
                                                .on_click(cx.listener(
                                                    move |this, _ev, _window, cx| {
                                                        this.nav.update(cx, |nav, cx| {
                                                            nav.trim(i);
                                                            cx.notify();
                                                        });
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("/"),
                                        )
                                },
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Button::new("new_folder")
                                    .icon(Icon::empty().path("icons/folder-plus.svg"))
                                    .small()
                                    .border_color(cx.theme().sidebar_border)
                                    .outline()
                                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                                        if let Some(prefix) =
                                            this.nav.read(cx).active_view().cloned()
                                        {
                                            this.browse_nav.update(cx, |_nav, cx| {
                                                cx.emit(BrowseNavEvent::CreateFolder(prefix));
                                            });
                                        }
                                    })),
                            )
                            .child(
                                Button::new("upload")
                                    .icon(Icon::empty().path("icons/upload.svg"))
                                    .small()
                                    .primary()
                                    .on_click(cx.listener(move |this, _ev, window, cx| {
                                        todo!();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .when_some(self.nav.read(cx).current_view().cloned(), |this, view| {
                this.child(view)
            })
    }
}
