use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Side, Sizable, StyledExt, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    notification::Notification,
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{nav::TabNav, s3::S3, ui::remote::RemoteUi};

mod remote;

pub struct Rooter {
    s3: Entity<S3>,
    tab_nav: Entity<TabNav>,
}

impl Rooter {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let s3 = cx.new(|_cx| S3::empty());
        let tab_nav = cx.new(|_cx| TabNav::new());

        cx.on_window_closed(move |cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        Self { s3, tab_nav }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut view = Self::new(window, cx);
            view.init_remotes(window, cx);
            view
        })
    }

    fn init_remotes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let _ = this.update_in(cx, |this, window, cx| {
                this.s3.update(cx, |s3, cx| {
                    if let Err(err) = s3.parse() {
                        window.push_notification(
                            Notification::error(err.message)
                                .title("Failed to init s3 remotes")
                                .autohide(false),
                            cx,
                        );
                    }
                })
            });
        })
        .detach();
    }
}

impl Render for Rooter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .flex()
            .size_full()
            .child(
                Sidebar::new(Side::Left)
                    .header(
                        SidebarHeader::new()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(cx.theme().primary)
                                            .rounded_md()
                                            .p_0p5()
                                            .size_6()
                                            .child(
                                                Icon::empty()
                                                    .path("icons/monitor-cloud.svg")
                                                    .text_color(black()),
                                            ),
                                    )
                                    .child("Balti"),
                            )
                            .mt(px(32.)),
                    )
                    .child(SidebarGroup::new("Remotes").child(
                        SidebarMenu::new().children(self.s3.read(cx).remotes().into_iter().map(
                            |(remote, s3_remote)| {
                                let s3_remote = s3_remote.clone();

                                SidebarMenuItem::new(remote)
                                    .icon(Icon::empty().path("icons/server.svg"))
                                    .on_click(cx.listener(move |this, _ev, window, cx| {
                                        let s3_remote = s3_remote.clone();

                                        this.tab_nav.update(cx, move |tab_nav, cx| {
                                            tab_nav.new_tab(
                                                RemoteUi::view(s3_remote.clone(), window, cx),
                                                cx,
                                            );
                                        });
                                    }))
                            },
                        )),
                    ))
                    .footer(
                        Button::new("create_remote")
                            .primary()
                            .label("Create remote")
                            .icon(IconName::Plus)
                            .small()
                            .w_full(),
                    ),
            )
            .child(div().size_full().map(|this| {
                match self.tab_nav.read(cx).active_index() {
                    Some(index) => this.child(self.render_tabs(*index, cx)),
                    None => this.child(
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
                                    div()
                                        .rounded_md()
                                        .p_2()
                                        .bg(cx.theme().muted)
                                        .child(Icon::empty().path("icons/server.svg").size_5()),
                                )
                                .child(div().text_lg().child("Select remote"))
                                .child(div().child("Select remote to start browsing")),
                        ),
                    ),
                }
            }))
            .when_some(notification_layer, |d, layer| d.child(layer))
    }
}

impl Rooter {
    fn render_tabs(&mut self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .child(
                TabBar::new("remote_tabs")
                    .bg(cx.theme().sidebar)
                    .selected_index(index)
                    .on_click(cx.listener(|this, index, _window, cx| {
                        this.tab_nav.update(cx, move |tab_nav, cx| {
                            tab_nav.select_tab(*index, cx);
                            cx.notify();
                        });
                    }))
                    .children(
                        self.tab_nav
                            .read(cx)
                            .tabs()
                            .into_iter()
                            .cloned()
                            .map(|remote| {
                                Tab::new()
                                    .child(
                                        h_flex().px_2().gap_3().items_center().child(
                                            div().font_medium().text_sm().child(remote.clone()),
                                        ), // .child(
                                           //     Button::new(remote.clone())
                                           //         .icon(IconName::Close)
                                           //         .xsmall()
                                           //         .ghost()
                                           //         .on_click(cx.listener(
                                           //             move |this, _ev, _window, cx| {
                                           //                 this.tab_nav.update(cx, |tab_nav, cx| {
                                           //                     tab_nav.close_tab(remote.clone(), cx);
                                           //                     cx.notify();
                                           //                 });
                                           //             },
                                           //         )),
                                           // ),
                                    )
                                    .suffix(
                                        Button::new(remote.clone())
                                            .mr_2()
                                            .icon(IconName::Close)
                                            .xsmall()
                                            .ghost()
                                            .on_click(cx.listener(
                                                move |this, _ev, _window, cx| {
                                                    this.tab_nav.update(cx, |tab_nav, cx| {
                                                        tab_nav.close_tab(remote.clone(), cx);
                                                        cx.notify();
                                                    });
                                                },
                                            )),
                                    )
                            }),
                    ),
            )
            .when_some(
                self.tab_nav.read(cx).active_view().cloned(),
                |this, view| this.child(view),
            )
    }
}
