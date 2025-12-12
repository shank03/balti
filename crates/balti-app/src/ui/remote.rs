use gpui::*;
use gpui_component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

use crate::{nav::TabId, s3::S3Remote};

pub struct RemoteUi {
    s3_remote: S3Remote,
}

impl RemoteUi {
    fn new(s3_remote: S3Remote, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { s3_remote }
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
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .w_full()
                    .border_b_1()
                    .text_sm()
                    .border_color(cx.theme().sidebar_border)
                    .child(
                        Button::new(SharedString::new(self.s3_remote.bucket_name.clone()))
                            .label(&self.s3_remote.bucket_name)
                            .ghost()
                            .small(),
                    ),
            )
            .child(self.s3_remote.remote_name.clone())
    }
}
