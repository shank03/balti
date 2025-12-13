use std::collections::BTreeMap;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Side, Sizable, StyledExt, ThemeMode, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    menu::DropdownMenu,
    notification::Notification,
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    tab::{Tab, TabBar},
};

use crate::{
    config::S3Config,
    nav::TabNav,
    rt,
    s3::{self, S3, S3Remote},
    ui::remote::RemoteUi,
};

mod browse;
mod remote;
mod remote_dialog;

actions!([EmptyAction]);

pub struct Rooter {
    s3: Entity<S3>,
    tab_nav: Entity<TabNav>,
    remotes: BTreeMap<SharedString, S3Remote>,
    is_testing: bool,
}

impl Rooter {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let s3 = cx.new(|_cx| S3::empty());
        let tab_nav = cx.new(|_cx| TabNav::new());

        let win_s3 = s3.clone();
        cx.on_window_closed(move |cx| {
            win_s3.read(cx).save_remotes();

            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        Self {
            s3,
            tab_nav,
            remotes: BTreeMap::new(),
            is_testing: false,
        }
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
                    } else {
                        this.remotes = s3.remotes().clone();
                    }
                })
            });
        })
        .detach();
    }
}

impl remote_dialog::CreateRemoteDialog for Rooter {
    fn create_remote(
        &mut self,
        name: SharedString,
        config: S3Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.s3.read(cx).has_remote(name.clone()) {
            window.push_notification(
                Notification::warning(format!("Remote with name \"{name}\" already exists")),
                cx,
            );
            return;
        }

        self.s3.update(cx, |s3, cx| {
            s3.add_remote(name, config);
            s3.save_remotes();
            cx.notify();
        });
        self.remotes = self.s3.read(cx).remotes().clone();
        window.close_all_dialogs(cx);
    }

    fn test(
        &mut self,
        config: crate::config::S3Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let remote = self.s3.read(cx).dummy_remote(config);
        let task = rt::spawn(cx, async move { s3::list_objects(remote, "").await });

        cx.spawn_in(window, async move |this, cx| {
            let _ = this.update(cx, |this, cx| {
                this.is_testing = true;
                cx.notify();
            });

            let result = task.await.flatten();

            let _ = this.update_in(cx, |this, window, cx| {
                this.is_testing = false;

                match result {
                    Ok(objects) => window.push_notification(
                        Notification::new()
                            .message(format!("Listed {} objects at root", objects.len()))
                            .title("Test success")
                            .icon(Icon::new(IconName::CircleCheck).text_color(green())),
                        cx,
                    ),
                    Err(err) => window.push_notification(
                        Notification::error(err.message).title("Test failed"),
                        cx,
                    ),
                };

                cx.notify();
            });
        })
        .detach();
    }

    fn is_testing(&self) -> bool {
        self.is_testing
    }
}

impl Render for Rooter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .flex()
            .size_full()
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .size_full()
                    .map(|this| match self.tab_nav.read(cx).active_index() {
                        Some(index) => this.child(self.render_tabs(*index, cx)),
                        None => this.child(self.render_empty_tab(cx)),
                    }),
            )
            .when_some(notification_layer, |d, layer| d.child(layer))
            .when_some(dialog_layer, |d, layer| d.child(layer))
    }
}

impl Rooter {
    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                SidebarGroup::new("Remotes").child(
                    SidebarMenu::new().children(self.s3.read(cx).remotes().into_iter().map(
                        |(remote, s3_remote)| {
                            let entity = cx.weak_entity();
                            let s3_remote = s3_remote.clone();

                            SidebarMenuItem::new(remote)
                                .icon(Icon::empty().path("icons/server.svg"))
                                .suffix(
                                    Button::new(SharedString::new(format!("btn-{remote}")))
                                        .icon(IconName::EllipsisVertical)
                                        .small()
                                        .ghost()
                                        .on_click(move |_ev, _window, cx| {
                                            cx.stop_propagation();
                                        })
                                        .dropdown_menu(move |menu, _window, _cx| {
                                            let entity = entity.clone();
                                            let _entity = entity.clone();

                                            menu.menu_element(
                                                Box::new(EmptyAction),
                                                move |_window, cx| {
                                                    let entity = _entity.clone();

                                                    div()
                                                        .id("")
                                                        .flex()
                                                        .gap_2()
                                                        .items_center()
                                                        .child(
                                                            Icon::empty()
                                                                .path("icons/pencil.svg")
                                                                .small(),
                                                        )
                                                        .child(div().child("Edit remote").text_sm())
                                                        .on_click(move |_ev, window, cx| {
                                                            let entity = entity.clone();

                                                            // window.open_dialog(cx, move |dialog, _window, cx| {
                                                            //     delete_dialog::comp(dialog, entity.clone(), delete_dialog::DeleteType::Folder(folder_id.clone()), folder_path.clone(), cx)
                                                            // });
                                                        })
                                                },
                                            )
                                            .separator()
                                            .menu_element(
                                                Box::new(EmptyAction),
                                                move |_window, cx| {
                                                    let entity = entity.clone();

                                                    div()
                                                        .id("")
                                                        .flex()
                                                        .gap_2()
                                                        .items_center()
                                                        .text_color(cx.theme().danger)
                                                        .child(Icon::new(IconName::Delete).small())
                                                        .child(
                                                            div().child("Delete remote").text_sm(),
                                                        )
                                                        .on_click(move |_ev, window, cx| {
                                                            let entity = entity.clone();

                                                            // window.open_dialog(cx, move |dialog, _window, cx| {
                                                            //     delete_dialog::comp(dialog, entity.clone(), delete_dialog::DeleteType::Folder(folder_id.clone()), folder_path.clone(), cx)
                                                            // });
                                                        })
                                                },
                                            )
                                        }),
                                )
                                .on_click(cx.listener(move |this, _ev, window, cx| {
                                    cx.stop_propagation();
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
                ),
            )
            .footer(
                div()
                    .flex()
                    .w_full()
                    .gap_2()
                    .child(remote_dialog::trigger(cx.weak_entity()).small().flex_1())
                    .child(
                        Button::new("theme-mode")
                            .icon(Icon::empty().path("icons/circle-shade.svg"))
                            .small()
                            .ghost()
                            .on_click(cx.listener(|_this, _ev, _window, cx| {
                                cx.stop_propagation();

                                let new_mode = if cx.theme().mode.is_dark() {
                                    ThemeMode::Light
                                } else {
                                    ThemeMode::Dark
                                };
                                crate::theme::change_color_mode(new_mode, cx);
                            })),
                    ),
            )
    }

    fn render_empty_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div().p_2().size_full().child(
            div()
                .flex()
                .flex_col()
                .size_full()
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
                .child(div().child("Select or create remote to start browsing"))
                .child(remote_dialog::trigger(cx.weak_entity())),
        )
    }

    fn render_tabs(&mut self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                TabBar::new("remote_tabs")
                    // .suffix(Button::new("suffix").label("Suffix"))
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
                                        ),
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
