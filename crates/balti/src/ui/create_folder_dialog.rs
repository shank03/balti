use gpui::*;
use gpui_component::{
    Disableable, IconName, StyledExt, WindowExt,
    button::{Button, ButtonVariants},
    dialog::Dialog,
    form::{field, v_form},
    input::{Input, InputState},
};

pub trait CreateFolderDialog: Render {
    fn create_folder(
        &mut self,
        folder_name: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    );

    fn is_creating(&self) -> bool;
}

pub fn dialog<T: CreateFolderDialog>(
    dialog: Dialog,
    entity: WeakEntity<T>,
    prefix: SharedString,
    folder_name_input_state: Entity<InputState>,
) -> Dialog {
    dialog
        .alert()
        .keyboard(false)
        .overlay_closable(false)
        .rounded_lg()
        .title("Create new folder")
        .v_flex()
        .child(
            v_form().child(
                field()
                    .label("Folder Name")
                    .child(Input::new(&folder_name_input_state).cleanable(true))
                    .description(format!("Path: {prefix}")),
            ),
        )
        .footer(move |_, _, _, cx| {
            let folder_name_input_state = folder_name_input_state.clone();

            let entity = entity.clone();
            let _entity = entity.clone();

            let is_creating = entity
                .read_with(cx, |this, _cx| this.is_creating())
                .unwrap_or_default();

            let cancel = Button::new("cancel_dialog")
                .label("Cancel")
                .disabled(is_creating)
                .on_click(|_, window, cx| {
                    window.close_dialog(cx);
                });

            let ok = Button::new("ok_dialog")
                .primary()
                .label("Create")
                .disabled(is_creating)
                .loading(is_creating)
                .loading_icon(IconName::LoaderCircle)
                .on_click(move |_ev, window, cx| {
                    let folder_name = folder_name_input_state.read(cx).value();

                    let _ = entity.update(cx, |this, cx| {
                        this.create_folder(folder_name, window, cx);
                        cx.notify();
                    });
                });

            vec![cancel, ok]
        })
}
