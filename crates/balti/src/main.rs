use gpui::*;
use gpui_component::{ActiveTheme, TitleBar};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod assets;
mod config;
mod err;
mod nav;
mod rt;
mod s3;
mod theme;
mod ui;

fn get_window_options(cx: &mut App) -> WindowOptions {
    let mut window_size = size(px(1600.0), px(1200.0));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.8);
        window_size.height = window_size.height.min(display_size.height * 0.8);
    }
    let bounds = Bounds::centered(None, window_size, cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitleBar::title_bar_options()),
        window_min_size: Some(size(px(800.0), px(600.0))),
        kind: WindowKind::Normal,
        window_decorations: Some(WindowDecorations::Client),
        tabbing_identifier: Some("Balti".into()),
        ..Default::default()
    }
}

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with(tracing_subscriber::fmt::layer().with_thread_ids(true))
        .init();

    Application::new()
        .with_assets(assets::AppAssets)
        .run(|cx: &mut App| {
            let window_options = get_window_options(cx);

            rt::init(cx);

            cx.activate(true);
            cx.open_window(window_options, |win, cx| {
                gpui_component::init(cx);
                gpui_component::theme::init(cx);
                theme::change_color_mode(cx.theme().mode, cx);

                let root_view = ui::Rooter::view(win, cx);
                cx.new(|cx| gpui_component::Root::new(root_view, win, cx))
            })
            .unwrap();
        });
}
