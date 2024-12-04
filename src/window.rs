use serde::Deserialize;
use tao::dpi::Size;
use tao::window::WindowAttributes;

use crate::webview::WebViewAttributesConfig;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct WindowAttributesConfig {
    pub label: Option<String>,
    pub inner_size: Option<Size>,
    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,
    pub title: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub maximized: Option<bool>,
    pub visible: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,
    pub always_on_top: Option<bool>,
    pub always_on_bottom: Option<bool>,
    pub focused: Option<bool>,
    pub content_protection: Option<bool>,
    pub visible_on_all_workspaces: Option<bool>,
    pub background_color: Option<(u8, u8, u8, u8)>,
    pub webview: Option<WebViewAttributesConfig>,
}

impl From<WindowAttributesConfig> for WindowAttributes {
    fn from(config: WindowAttributesConfig) -> Self {
        WindowAttributes {
            inner_size: config.inner_size,
            resizable: config.resizable.unwrap_or(true),
            minimizable: config.minimizable.unwrap_or(true),
            maximizable: config.maximizable.unwrap_or(true),
            closable: config.closable.unwrap_or(true),
            title: config.title.unwrap_or_else(|| "PyWui window".to_string()),
            maximized: config.maximized.unwrap_or(false),
            visible: config.visible.unwrap_or(true),
            transparent: config.transparent.unwrap_or(false),
            decorations: config.decorations.unwrap_or(true),
            always_on_top: config.always_on_top.unwrap_or(false),
            always_on_bottom: config.always_on_bottom.unwrap_or(false),
            focused: config.focused.unwrap_or(true),
            content_protection: config.content_protection.unwrap_or(false),
            visible_on_all_workspaces: config.visible_on_all_workspaces.unwrap_or(false),
            background_color: config.background_color,
            ..WindowAttributes::default()
        }
    }
}
