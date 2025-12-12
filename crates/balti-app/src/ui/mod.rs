use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Side, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    notification::Notification,
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    tab::{Tab, TabBar},
};

use crate::s3::S3;

pub struct Rooter {
    s3: Entity<S3>,
}

impl Rooter {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let s3 = cx.new(|_cx| S3::empty());

        cx.on_window_closed(move |cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        Self { s3 }
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
                    .child(
                        SidebarGroup::new("Remotes").child(SidebarMenu::new().children(
                            self.s3.read(cx).remotes().into_iter().map(|remote| {
                                SidebarMenuItem::new(remote)
                                    .icon(Icon::empty().path("icons/server.svg"))
                            }),
                        )),
                    )
                    .footer(
                        Button::new("create_remote")
                            .primary()
                            .label("Create remote")
                            .icon(IconName::Plus)
                            .small()
                            .w_full(),
                    ),
            )
            .child(
                TabBar::new("tabs")
                    .selected_index(0)
                    .on_click(|selected_index, _, _| {
                        println!("Tab {} selected", selected_index);
                    })
                    .child(Tab::new().label("Account"))
                    .child(Tab::new().label("Profile"))
                    .child(Tab::new().label("Settings")),
            )
            .when_some(notification_layer, |d, layer| d.child(layer))
    }
}
