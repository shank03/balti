use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

use crate::{
    nav::{BucketNav, TabId},
    s3::S3Remote,
    ui::browse::BrowseUi,
};

pub struct BrowseNavEvent(pub SharedString);
pub struct BrowseNav;
impl EventEmitter<BrowseNavEvent> for BrowseNav {}

pub struct RemoteUi {
    s3_remote: S3Remote,
    nav: Entity<BucketNav>,
    browse_nav: Entity<BrowseNav>,
    _subcriptions: Vec<Subscription>,
}

impl RemoteUi {
    fn new(s3_remote: S3Remote, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let browse_nav = cx.new(|_| BrowseNav {});

        let nav_sub = cx.subscribe_in(&browse_nav, window, |this, _entity, event, window, cx| {
            this.nav.update(cx, |nav, cx| {
                nav.push(
                    BrowseUi::view(
                        this.browse_nav.clone(),
                        this.s3_remote.clone(),
                        event.0.clone(),
                        window,
                        cx,
                    ),
                    cx,
                );
            });
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stack = self.nav.read(cx).stack();
        let len = stack.len();

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .w_full()
                    .border_b_1()
                    .text_sm()
                    .border_color(cx.theme().sidebar_border)
                    .children(stack.into_iter().cloned().enumerate().map(|(i, s)| {
                        h_flex()
                            .child(
                                Button::new(SharedString::new(i.to_string()))
                                    .label(s.trim_matches('/').to_owned())
                                    .ghost()
                                    .small()
                                    .px_1()
                                    .map(|this| {
                                        if i == len - 1 {
                                            this.text_color(cx.theme().primary)
                                        } else {
                                            this
                                        }
                                    })
                                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                                        this.nav.update(cx, |nav, cx| {
                                            nav.trim(i);
                                            cx.notify();
                                        });
                                    })),
                            )
                            .child(div().text_color(cx.theme().muted_foreground).child("/"))
                    })),
            )
            .when_some(self.nav.read(cx).current_view().cloned(), |this, view| {
                this.child(view)
            })
    }
}
