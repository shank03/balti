use std::sync::Arc;

use balti_err::AppError;
use balti_s3::{S3Config, S3Remote};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Side, Sizable, ThemeMode, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    menu::DropdownMenu,
    notification::Notification,
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    tab::{Tab, TabBar},
};

use crate::{config, nav::TabNav, rt, s3::S3RemoteManager, ui::remote::RemoteUi};

mod browse;
mod create_folder_dialog;
mod delete_object_dialog;
mod remote;
mod remote_dialog;

actions!([EmptyAction]);

actions!(window, [CloseWindow, Quit, About, CheckForUpdates]);
pub const APP_CONTEXT: &str = "Rooter";

fn init_kb(cx: &mut App) {
    #[cfg(target_os = "macos")]
    cx.bind_keys([KeyBinding::new("cmd-w", CloseWindow, Some(APP_CONTEXT))]);

    #[cfg(target_os = "macos")]
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, Some(APP_CONTEXT))]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([KeyBinding::new("ctrl-w", CloseWindow, Some(APP_CONTEXT))]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([KeyBinding::new("alt-f4", Quit, Some(APP_CONTEXT))]);
}

pub struct Rooter {
    s3_remote_manager: Entity<S3RemoteManager>,
    tab_nav: TabNav,

    focus_handle: FocusHandle,
    is_testing: bool,
}

impl Rooter {
    fn new(focus_handle: FocusHandle, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let s3_remote_manager = cx.new(|_cx| S3RemoteManager::empty());
        let tab_nav = TabNav::new();

        let win_s3 = s3_remote_manager.clone();
        cx.on_window_closed(move |cx| {
            win_s3.read(cx).save_remotes();

            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        Self {
            s3_remote_manager,
            tab_nav,
            focus_handle,
            is_testing: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            init_kb(cx);

            let focus_handle = cx.focus_handle();
            focus_handle.focus(window);

            let mut view = Self::new(focus_handle, window, cx);
            view.init_remotes(window, cx);
            view
        })
    }

    fn init_remotes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let _ = this.update_in(cx, |this, window, cx| {
                this.s3_remote_manager.update(cx, |s3, cx| {
                    if let Err(err) = s3.parse() {
                        window.push_notification(
                            Notification::error(err.message)
                                .title("Failed to init s3 remotes")
                                .autohide(false),
                            cx,
                        );
                    }
                    cx.notify();
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn open_about_dialog(&mut self, _: &About, window: &mut Window, cx: &mut Context<Self>) {
        let message = format!("Balti {}", config::BALTI_VERSION);
        let detail = config::BALTI_COMMIT_SHA;

        let task = window.prompt(
            PromptLevel::Info,
            message.as_str(),
            Some(detail),
            &[PromptButton::Ok(SharedString::new_static("Ok"))],
            cx,
        );

        cx.spawn(async move |_this, _cx| {
            let _ = task.await;
        })
        .detach();
    }

    fn check_for_updates(
        &mut self,
        _: &CheckForUpdates,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let task = rt::spawn(cx, async move {
            println!("todo: {}", config::BALTI_COMMIT_SHA);
        });

        cx.spawn_in(window, async move |_this, _cx| {
            let _result = task.await;
        })
        .detach();
    }

    fn on_theme_change(&mut self, _ev: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        let new_mode = if cx.theme().mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        crate::theme::change_color_mode(new_mode, cx);
    }

    fn delete_remote(
        &mut self,
        remote_name: Arc<str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let task = window.prompt(
            PromptLevel::Critical,
            format!("Delete '{}' remote ?", remote_name).as_str(),
            None,
            &[
                PromptButton::Cancel(SharedString::new_static("Cancel")),
                PromptButton::Ok(SharedString::new_static("Delete")),
            ],
            cx,
        );

        cx.spawn(async move |this, cx| {
            let result = task.await;
            match result {
                Ok(index) => {
                    if index == 1 {
                        let _ = this.update(cx, |this, cx| {
                            this.tab_nav
                                .close_tab_by_remote(SharedString::new(remote_name.clone()), cx);
                            this.s3_remote_manager.update(cx, |s3, cx| {
                                s3.remove_remote(remote_name.into());
                                s3.save_remotes();
                                cx.notify();
                            });

                            cx.notify();
                        });
                    }
                }
                Err(err) => {
                    let _ = AppError::err(err);
                }
            };
        })
        .detach();
    }

    fn new_tab(&mut self, s3_remote: S3Remote, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        let view = RemoteUi::view(s3_remote, window, cx);
        self.tab_nav.new_tab(view, cx);
        cx.notify();
    }

    fn select_tab(&mut self, index: &usize, _window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.tab_nav.select_tab(*index);
        cx.notify();
    }

    fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.tab_nav.close_tab(index);
        cx.notify();
    }

    fn close_active_tab(&mut self) -> bool {
        self.tab_nav.close_active_tab()
    }
}

impl remote_dialog::RemoteDialog for Rooter {
    fn create_remote(
        &mut self,
        name: SharedString,
        config: S3Config,
        old_remote: Option<Arc<str>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name: Arc<str> = name.into();

        match old_remote {
            Some(old_remote) => {
                self.tab_nav
                    .close_tab_by_remote(old_remote.clone().into(), cx);
                self.s3_remote_manager.update(cx, |s3, cx| {
                    s3.remove_remote(old_remote);
                    s3.save_remotes();
                    cx.notify();
                });
            }
            None => {
                if self.s3_remote_manager.read(cx).has_remote(name.clone()) {
                    window.push_notification(
                        Notification::warning(format!(
                            "Remote with name \"{name}\" already exists"
                        )),
                        cx,
                    );
                    return;
                }
            }
        };

        self.s3_remote_manager.update(cx, |s3, cx| {
            s3.add_remote(name, config);
            s3.save_remotes();
            cx.notify();
        });
        window.close_all_dialogs(cx);
    }

    fn test_config(&mut self, config: S3Config, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.s3_remote_manager.read(cx).dummy_remote(config);
        let task = rt::spawn(cx, async move { balti_s3::list_objects(remote, "").await });

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
            .id("rooter")
            .key_context(APP_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::open_about_dialog))
            .on_action(cx.listener(Self::check_for_updates))
            .on_action(cx.listener(|this, _: &CloseWindow, window, cx| {
                let closed = this.close_active_tab();
                cx.notify();
                if !closed {
                    window.remove_window();
                }
            }))
            .on_action(cx.listener(|_this, _: &Quit, window, cx| {
                window.remove_window();
                cx.quit();
            }))
            .flex()
            .size_full()
            .child(self.render_sidebar(cx))
            .child(div().size_full().map(|this| {
                if self.tab_nav.tabs().is_empty() {
                    this.child(self.render_empty_tab(cx))
                } else {
                    this.child(self.render_tabs(cx))
                }
            }))
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
                                        Icon::empty().path("icons/bucket.svg").text_color(black()),
                                    ),
                            )
                            .child("Balti"),
                    )
                    .mt(px(32.)),
            )
            .child(SidebarGroup::new("Remotes").child(
                SidebarMenu::new().children(
                    self.s3_remote_manager.read(cx).remotes().into_iter().map(
                        |(remote, s3_remote)| {
                            let entity = cx.weak_entity();
                            let _s3_remote = s3_remote.clone();
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
                                            let s3_remote = _s3_remote.clone();
                                            let _s3_remote = _s3_remote.clone();
                                            let entity = entity.clone();
                                            let _entity = entity.clone();

                                            menu.menu_element(
                                                Box::new(EmptyAction),
                                                move |_window, _cx| {
                                                    let s3_remote = _s3_remote.clone();
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
                                                            remote_dialog::open_dialog(
                                                                Some(s3_remote.clone()),
                                                                entity.clone(),
                                                                window,
                                                                cx,
                                                            );
                                                        })
                                                },
                                            )
                                            .separator()
                                            .menu_element(
                                                Box::new(EmptyAction),
                                                move |_window, cx| {
                                                    let remote_name = s3_remote.remote_name.clone();
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
                                                            let _ = entity.clone().update(
                                                                cx,
                                                                |this, cx| {
                                                                    this.delete_remote(
                                                                        remote_name.clone(),
                                                                        window,
                                                                        cx,
                                                                    );
                                                                    cx.notify();
                                                                },
                                                            );
                                                        })
                                                },
                                            )
                                        }),
                                )
                                .on_click(cx.listener(move |this, _ev, window, cx| {
                                    this.new_tab(s3_remote.clone(), window, cx);
                                }))
                        },
                    ),
                ),
            ))
            .footer(
                div()
                    .flex()
                    .w_full()
                    .gap_2()
                    .child(
                        remote_dialog::trigger(cx.weak_entity(), None)
                            .small()
                            .flex_1(),
                    )
                    .child(
                        Button::new("theme-mode")
                            .icon(Icon::empty().path("icons/circle-shade.svg"))
                            .small()
                            .ghost()
                            .on_click(cx.listener(Self::on_theme_change)),
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
                .child(remote_dialog::trigger(cx.weak_entity(), None)),
        )
    }

    fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                TabBar::new("remote_tabs")
                    .menu(true)
                    // .suffix(Button::new("suffix").label("Suffix"))
                    .bg(cx.theme().sidebar)
                    .selected_index(*self.tab_nav.active_index())
                    .on_click(cx.listener(Self::select_tab))
                    .children(
                        self.tab_nav
                            .tabs()
                            .iter()
                            .enumerate()
                            .map(|(index, remote)| {
                                let remote = remote.clone();

                                Tab::new().label(remote.clone()).suffix(
                                    Button::new(remote.clone())
                                        .mr_2()
                                        .icon(IconName::Close)
                                        .xsmall()
                                        .ghost()
                                        .on_click(cx.listener(move |this, _ev, _window, cx| {
                                            this.close_tab(index, cx);
                                        })),
                                )
                            }),
                    ),
            )
            .when_some(self.tab_nav.active_view().cloned(), |this, view| {
                this.child(view)
            })
    }
}
