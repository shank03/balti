use gpui::*;
use gpui_component::{
    Disableable, IconName, StyledExt, WindowExt,
    button::{Button, ButtonVariants},
    dialog::Dialog,
    form::{field, v_form},
    input::{Input, InputState},
};

use crate::config::S3Config;

pub trait CreateRemoteDialog: Render {
    fn create_remote(
        &mut self,
        name: SharedString,
        config: S3Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    );

    fn test(&mut self, config: S3Config, window: &mut Window, cx: &mut Context<Self>);

    fn is_testing(&self) -> bool;
}

pub fn trigger<T: CreateRemoteDialog>(entity: WeakEntity<T>) -> Button {
    Button::new("create_remote")
        .primary()
        .icon(IconName::Plus)
        .text_color(black())
        .label("Create remote")
        .on_click(move |_ev, window, cx| {
            let entity = entity.clone();

            let remote_name_input_state =
                cx.new(|cx| InputState::new(window, cx).placeholder("cooler_remote"));
            let access_key_id_input_state =
                cx.new(|cx| InputState::new(window, cx).placeholder("ABCD1234"));
            let secret_access_key_input_state =
                cx.new(|cx| InputState::new(window, cx).placeholder("secret-abcd-xyz-123"));
            let region_input_state = cx.new(|cx| InputState::new(window, cx).placeholder("auto"));
            let endpoint_input_state =
                cx.new(|cx| InputState::new(window, cx).placeholder("https://endpoint.com"));
            let bucket_name_input_state =
                cx.new(|cx| InputState::new(window, cx).placeholder("acme-bucket"));

            window.open_dialog(cx, move |dialog, _window, cx| {
                comp(
                    dialog,
                    entity.clone(),
                    remote_name_input_state.clone(),
                    access_key_id_input_state.clone(),
                    secret_access_key_input_state.clone(),
                    region_input_state.clone(),
                    endpoint_input_state.clone(),
                    bucket_name_input_state.clone(),
                    cx,
                )
            });
        })
}

fn comp<T: CreateRemoteDialog>(
    dialog: Dialog,
    entity: WeakEntity<T>,
    remote_name_input_state: Entity<InputState>,
    access_key_id_input_state: Entity<InputState>,
    secret_access_key_input_state: Entity<InputState>,
    region_input_state: Entity<InputState>,
    endpoint_input_state: Entity<InputState>,
    bucket_name_input_state: Entity<InputState>,
    cx: &mut App,
) -> Dialog {
    let invalid_fields = remote_name_input_state.read(cx).value().is_empty()
        || access_key_id_input_state.read(cx).value().is_empty()
        || secret_access_key_input_state.read(cx).value().is_empty()
        || bucket_name_input_state.read(cx).value().is_empty()
        || endpoint_input_state.read(cx).value().is_empty();

    let _entity = entity.clone();

    dialog
        .alert()
        .keyboard(false)
        .overlay_closable(false)
        .rounded_lg()
        .title("Create new remote")
        .v_flex()
        .child(
            v_form()
                .child(
                    field()
                        .label("Remote Name")
                        .child(Input::new(&remote_name_input_state).cleanable(true)),
                )
                .child(
                    field()
                        .label("Access Key")
                        .child(Input::new(&access_key_id_input_state).cleanable(true)),
                )
                .child(
                    field()
                        .label("Secret Access Key")
                        .child(Input::new(&secret_access_key_input_state).cleanable(true)),
                )
                .child(
                    field()
                        .label("Region (default: auto)")
                        .child(Input::new(&region_input_state).cleanable(true)),
                )
                .child(
                    field()
                        .label("Endpoint")
                        .child(Input::new(&endpoint_input_state).cleanable(true)),
                )
                .child(
                    field()
                        .label("Bucket name")
                        .child(Input::new(&bucket_name_input_state).cleanable(true)),
                ),
        )
        .footer(move |_, _, _, cx| {
            let _remote_name_input_state = remote_name_input_state.clone();
            let _access_key_id_input_state = access_key_id_input_state.clone();
            let _secret_access_key_input_state = secret_access_key_input_state.clone();
            let _region_input_state = region_input_state.clone();
            let _endpoint_input_state = endpoint_input_state.clone();
            let _bucket_name_input_state = bucket_name_input_state.clone();

            let remote_name_input_state = remote_name_input_state.clone();
            let access_key_id_input_state = access_key_id_input_state.clone();
            let secret_access_key_input_state = secret_access_key_input_state.clone();
            let region_input_state = region_input_state.clone();
            let endpoint_input_state = endpoint_input_state.clone();
            let bucket_name_input_state = bucket_name_input_state.clone();

            let entity = _entity.clone();
            let _entity = _entity.clone();

            let is_testing = entity
                .read_with(cx, |this, _cx| this.is_testing())
                .unwrap_or_default();

            let test = Button::new("test_dialog")
                .ghost()
                .label("Test")
                .disabled(invalid_fields || is_testing)
                .loading_icon(IconName::LoaderCircle)
                .loading(is_testing)
                .on_click(move |_, window, cx| {
                    let access_key_id = _access_key_id_input_state.read(cx).value();
                    let secret_access_key = _secret_access_key_input_state.read(cx).value();
                    let region = _region_input_state.read(cx).value();
                    let region = if region.trim().is_empty() {
                        SharedString::new_static("auto")
                    } else {
                        region
                    };
                    let endpoint = _endpoint_input_state.read(cx).value();
                    let bucket_name = _bucket_name_input_state.read(cx).value();

                    let config = S3Config {
                        access_key_id,
                        secret_access_key,
                        region,
                        endpoint,
                        bucket_name,
                    };

                    let _ = _entity.update(cx, |this, cx| {
                        this.test(config, window, cx);
                        cx.notify();
                    });
                });

            let cancel = Button::new("cancel_dialog")
                .label("Cancel")
                .disabled(is_testing)
                .on_click(|_, window, cx| {
                    window.close_dialog(cx);
                });

            let ok = Button::new("ok_dialog")
                .primary()
                .label("Save")
                .disabled(invalid_fields)
                .on_click(move |_ev, window, cx| {
                    let remote_name = remote_name_input_state.read(cx).value();
                    let access_key_id = access_key_id_input_state.read(cx).value();
                    let secret_access_key = secret_access_key_input_state.read(cx).value();
                    let region = region_input_state.read(cx).value();
                    let region = if region.trim().is_empty() {
                        SharedString::new_static("auto")
                    } else {
                        region
                    };
                    let endpoint = endpoint_input_state.read(cx).value();
                    let bucket_name = bucket_name_input_state.read(cx).value();

                    let config = S3Config {
                        access_key_id,
                        secret_access_key,
                        region,
                        endpoint,
                        bucket_name,
                    };

                    let _ = entity.update(cx, |this, cx| {
                        this.create_remote(remote_name, config, window, cx);
                        cx.notify();
                    });
                });

            vec![test, cancel, ok]
        })
}
