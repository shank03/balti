use std::{collections::HashMap, rc::Rc, sync::Arc};

use futures::StreamExt;
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt, VirtualListScrollHandle, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::InputState,
    notification::Notification,
    scroll::ScrollableElement,
    skeleton::Skeleton,
    v_virtual_list,
};

use crate::{
    err::AppError,
    nav::BrowsePrefix,
    rt,
    s3::{self, S3Object, S3Remote, TrimPrefix},
    ui::{
        create_folder_dialog, delete_object_dialog,
        remote::{BrowseNav, BrowseNavEvent},
    },
};

pub struct BrowseUi {
    browse_nav: Entity<BrowseNav>,
    s3_remote: S3Remote,
    prefix: SharedString,

    objects: Vec<Arc<S3Object>>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    objects_scroll_handle: VirtualListScrollHandle,
    checked_objects: HashMap<SharedString, Arc<S3Object>>,

    loading: bool,
    creating_folder: bool,
    deleting_objects: bool,
    error: Option<AppError>,
    _subscriptions: Vec<Subscription>,
}

impl BrowseUi {
    fn new(
        browse_nav: Entity<BrowseNav>,
        s3_remote: S3Remote,
        prefix: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let nav_sub = cx.subscribe_in(&browse_nav, window, |this, _entity, event, window, cx| {
            match event {
                BrowseNavEvent::CreateFolder(prefix) => {
                    if &this.prefix == prefix {
                        this.new_folder_dialog(window, cx)
                    }
                }
                BrowseNavEvent::UploadFiles(prefix) => {
                    if &this.prefix == prefix {
                        // let a = 0;
                    }
                }
                _ => (),
            };
        });

        Self {
            browse_nav,
            s3_remote,
            prefix,
            objects: Vec::new(),
            item_sizes: Rc::new(Vec::new()),
            objects_scroll_handle: VirtualListScrollHandle::new(),
            checked_objects: HashMap::new(),
            loading: false,
            creating_folder: false,
            deleting_objects: false,
            error: None,
            _subscriptions: vec![nav_sub],
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
                    Ok(objects) => {
                        this.checked_objects.clear();

                        let item_sizes = objects.iter().map(|_| size(px(256.), px(40.))).collect();
                        this.item_sizes = Rc::new(item_sizes);
                        this.objects = objects
                    }
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

    fn new_folder_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let folder_name_input_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("CoolFolder"));

        let prefix = self.prefix.clone();
        let entity = cx.weak_entity();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            create_folder_dialog::dialog(
                dialog,
                entity.clone(),
                prefix.clone(),
                folder_name_input_state.clone(),
            )
        });
    }
}

impl create_folder_dialog::CreateFolderDialog for BrowseUi {
    fn create_folder(
        &mut self,
        folder_name: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let remote = self.s3_remote.clone();
        let key = format!(
            "{}/{}",
            self.prefix.trim_matches('/'),
            folder_name
                .trim()
                .trim_matches('/')
                .replace(".", "")
                .replace("..", "")
        );

        let task = rt::spawn(
            cx,
            async move { s3::create_folder(remote, key.as_str()).await },
        );

        cx.spawn_in(window, async move |this, cx| {
            let _ = this.update(cx, |this, cx| {
                this.creating_folder = true;
                cx.notify();
            });

            let result = task.await.flatten();

            let _ = this.update_in(cx, |this, window, cx| {
                this.creating_folder = false;

                match result {
                    Ok(_) => {
                        this.list_objects(window, cx);
                        window.close_dialog(cx);
                    }
                    Err(err) => window.push_notification(
                        Notification::error(err.message).title("Error creating folder"),
                        cx,
                    ),
                };

                cx.notify();
            });
        })
        .detach();
    }

    fn is_creating(&self) -> bool {
        self.creating_folder
    }
}

impl delete_object_dialog::DeleteObjectDialog for BrowseUi {
    fn delete_objects(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let remote = self.s3_remote.clone();

        let objects = self
            .checked_objects
            .iter()
            .map(|(_, obj)| obj.clone())
            .collect::<Vec<_>>();

        let task = rt::spawn(cx, async move {
            let tasks = objects.into_iter().map(|obj| {
                let remote = remote.clone();
                async move {
                    match obj.as_ref() {
                        S3Object::Folder(key) => s3::delete_folder(remote, key.as_str()).await,
                        S3Object::File { key, .. } => s3::delete_file(remote, key.as_str()).await,
                    }
                }
            });

            let results = futures::stream::iter(tasks)
                .buffer_unordered(8)
                .collect::<Vec<_>>()
                .await;

            let mut err_message = String::from("");
            for result in results.into_iter() {
                if let Err(err) = result {
                    err_message.push_str(&err.message);
                    err_message.push_str(";\n");
                }
            }

            if err_message.is_empty() {
                Ok(())
            } else {
                Err(AppError::message(err_message))
            }
        });

        cx.spawn_in(window, async move |this, cx| {
            let _ = this.update(cx, |this, cx| {
                this.deleting_objects = true;
                cx.notify();
            });

            let result = task.await.flatten();

            let _ = this.update_in(cx, |this, window, cx| {
                this.deleting_objects = false;

                match result {
                    Ok(_) => {
                        this.list_objects(window, cx);

                        window.close_all_dialogs(cx);
                        window.push_notification(
                            Notification::success("Objects deleted")
                                .icon(Icon::new(IconName::CircleCheck).text_color(green())),
                            cx,
                        );
                    }
                    Err(err) => window.push_notification(
                        Notification::error(err.message).title("Failed to delete object(s)"),
                        cx,
                    ),
                };

                cx.notify();
            });
        })
        .detach();
    }

    fn is_deleting(&self) -> bool {
        self.deleting_objects
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
            .child(deferred(self.render_browse_status(cx)).with_priority(999))
            .when_some(self.error.clone(), |this, error| {
                this.child(self.render_error(error.message, cx))
            })
            .when_none(&self.error.clone(), |this| {
                this.when_else(
                    self.loading,
                    |this| {
                        this.child(
                            div()
                                .p_2()
                                .pb_10()
                                .flex()
                                .flex_col()
                                .size_full()
                                .gap_0p5()
                                .children(
                                    (0..7)
                                        .map(|_| Skeleton::new().w_full().h(px(40.)).rounded_md()),
                                ),
                        )
                    },
                    |this| this.child(self.render_object_list(cx)),
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

    fn render_browse_status(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .bottom_0()
            .flex()
            .flex_shrink_0()
            .items_center()
            .bg(cx.theme().sidebar)
            .border_t_1()
            .border_color(cx.theme().sidebar_border)
            .px_2()
            .py_1p5()
            .w_full()
            .justify_between()
            .text_sm()
            .map(|this| {
                if self.checked_objects.is_empty() {
                    this.child(div().map(|this| {
                        if self.loading {
                            this.child("...")
                        } else {
                            this.child(format!("Total: {} item(s)", self.objects.len()))
                        }
                    }))
                } else {
                    this.bg(cx.theme().primary)
                        .text_color(cx.theme().primary_foreground)
                        .child(
                            Button::new("select-all")
                                .small()
                                .outline()
                                .icon(IconName::Asterisk)
                                .label("Select all")
                                .on_click(cx.listener(|this, _ev, _window, cx| {
                                    this.objects.iter().for_each(|remote| {
                                        this.checked_objects
                                            .insert(remote.key().clone(), remote.clone());
                                    });
                                    cx.notify();
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_4()
                                .child(div().text_sm().font_medium().child(format!(
                                    "{} item(s) selected",
                                    self.checked_objects.len()
                                )))
                                .child(
                                    Button::new("clear")
                                        .small()
                                        .outline()
                                        .icon(IconName::Close)
                                        .label("Clear all")
                                        .on_click(cx.listener(|this, _ev, _window, cx| {
                                            this.checked_objects.clear();
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            Button::new("delete")
                                .small()
                                .danger()
                                .icon(IconName::Delete)
                                .label("Delete items")
                                .on_click(cx.listener(|this, _ev, window, cx| {
                                    let count = this.checked_objects.len();
                                    let entity = cx.weak_entity();

                                    window.open_dialog(cx, move |dialog, _window, _cx| {
                                        delete_object_dialog::dialog(dialog, count, entity.clone())
                                    });
                                })),
                        )
                }
            })
    }

    fn render_object_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.prefix.clone())
            .p_2()
            .pb_10()
            .flex()
            .flex_col()
            .size_full()
            .gap_0p5()
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "browse-list",
                    self.item_sizes.clone(),
                    |this, range, _window, cx| {
                        range
                            .map(|i| match this.objects.get(i) {
                                Some(object) => this.render_object_item(i, object.clone(), cx),
                                None => div().id("i").child("whoops ??"),
                            })
                            .collect()
                    },
                )
                .w_full()
                .track_scroll(&self.objects_scroll_handle),
            )
    }

    fn render_object_item(
        &self,
        i: usize,
        object: Arc<S3Object>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let _object = object.clone();

        div()
            .id(SharedString::new(i.to_string()))
            .flex()
            .w_full()
            .h(px(40.))
            .gap_4()
            .items_center()
            .justify_between()
            .rounded_md()
            .p_2()
            .pr_8()
            .text_sm()
            .map(|this| {
                if self.checked_objects.contains_key(object.key()) {
                    this.border_1().border_color(cx.theme().primary)
                } else {
                    this.border_b_1().border_color(cx.theme().sidebar_border)
                }
            })
            .group(i.to_string())
            .hover(|this| this.bg(cx.theme().secondary_hover.opacity(0.4)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        Checkbox::new(SharedString::new(format!("chk-{i}")))
                            .checked(self.checked_objects.contains_key(object.key()))
                            .map(|this| {
                                if !self.checked_objects.contains_key(object.key()) {
                                    this.opacity(0.)
                                        .group_hover(SharedString::new(i.to_string()), |el| {
                                            el.opacity(100.)
                                        })
                                } else {
                                    this
                                }
                            })
                            .on_click(cx.listener(move |this, checked, _window, cx| {
                                cx.stop_propagation();

                                let object = _object.clone();
                                let key = object.key();
                                if *checked {
                                    this.checked_objects.insert(key.clone(), object.clone());
                                } else {
                                    this.checked_objects.remove(key);
                                }
                                cx.notify();
                            })),
                    )
                    .child(match object.as_ref() {
                        s3::S3Object::Folder(_) => Icon::new(IconName::Folder),
                        s3::S3Object::File { .. } => Icon::empty().path("icons/file-digit.svg"),
                    })
                    .text_sm()
                    .child(object.key().trim_key_prefix(self.prefix.as_str())),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .gap_4()
                    .items_center()
                    .map(|this| match object.as_ref() {
                        s3::S3Object::Folder(_) => this,
                        s3::S3Object::File {
                            size,
                            last_modified,
                            ..
                        } => this
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                div()
                                    // i know this font won't exist for everyone
                                    .font_family("JetBrains Mono")
                                    .child(size.to_string()),
                            )
                            .child(last_modified.clone().unwrap_or_default()),
                    }),
            )
            .map(|this| match object.as_ref() {
                s3::S3Object::Folder(key) => {
                    let prefix = key.clone();
                    this.on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.browse_nav.update(cx, |_nav, cx| {
                            cx.emit(BrowseNavEvent::NewView(prefix.clone()));
                        });
                    }))
                }
                s3::S3Object::File { .. } => this,
            })
    }
}
