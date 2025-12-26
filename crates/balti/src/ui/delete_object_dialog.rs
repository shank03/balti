use gpui::*;
use gpui_component::{
    Disableable, IconName, StyledExt, WindowExt,
    button::{Button, ButtonVariants},
    dialog::Dialog,
};

pub trait DeleteObjectDialog: Render {
    fn delete_objects(&mut self, window: &mut Window, cx: &mut Context<Self>);

    fn is_deleting(&self) -> bool;
}

pub fn dialog<T: DeleteObjectDialog>(
    dialog: Dialog,
    selected_objects_count: usize,
    entity: WeakEntity<T>,
) -> Dialog {
    dialog
        .alert()
        .keyboard(false)
        .overlay_closable(false)
        .rounded_lg()
        .title("Delete object(s)")
        .v_flex()
        .child(format!(
            "Delete selected {selected_objects_count} items ? This action cannot be UNDONE."
        ))
        .footer(move |_, _, _, cx| {
            let entity = entity.clone();

            let is_deleting = entity
                .read_with(cx, |this, _cx| this.is_deleting())
                .unwrap_or_default();

            let cancel = Button::new("cancel_dialog")
                .label("Cancel")
                .disabled(is_deleting)
                .on_click(|_, window, cx| {
                    window.close_dialog(cx);
                });

            let ok = Button::new("ok_dialog")
                .danger()
                .label("Delete")
                .disabled(is_deleting)
                .loading(is_deleting)
                .loading_icon(IconName::LoaderCircle)
                .on_click(move |_ev, window, cx| {
                    let _ = entity.update(cx, |this, cx| {
                        this.delete_objects(window, cx);
                        cx.notify();
                    });
                });

            vec![cancel, ok]
        })
}
