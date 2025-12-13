use std::{collections::BTreeMap, rc::Rc, sync::LazyLock};

use gpui::{App, SharedString};
use gpui_component::{Theme, ThemeConfig, ThemeMode, ThemeSet};

pub static THEMES: LazyLock<BTreeMap<SharedString, ThemeConfig>> = LazyLock::new(|| {
    let mut themes = BTreeMap::new();
    for source in [include_str!("./balti.json")].into_iter() {
        let theme_set = serde_json::from_str::<ThemeSet>(source).unwrap();
        for theme in theme_set.themes.into_iter() {
            themes.insert(theme.name.clone(), theme);
        }
    }

    themes
});

pub fn change_color_mode(mode: ThemeMode, cx: &mut App) {
    let theme = Theme::global_mut(cx);

    let theme_name = match mode {
        ThemeMode::Light => "Balti Light",
        ThemeMode::Dark => "Balti Dark",
    };

    if let Some(config) = THEMES.get(theme_name) {
        let theme_config = Rc::new(config.clone());
        theme.mode = mode;
        theme.apply_config(&theme_config);
    }
}
