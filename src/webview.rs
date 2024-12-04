use std::collections::HashMap;
use serde::Deserialize;
use wry::{WebViewAttributes, WebViewId};

#[derive(Deserialize, Debug, Default, Clone)]
pub struct WebViewAttributesConfig {
    pub user_agent: Option<String>,
    pub visible: Option<bool>,
    pub transparent: Option<bool>,
    pub background_color: Option<(u8, u8, u8, u8)>,
    pub url: Option<String>,
    pub zoom_hotkeys_enabled: Option<bool>,
    pub html: Option<String>,
    pub initialization_scripts: Option<Vec<(String, bool)>>,
    pub clipboard: Option<bool>,
    pub devtools: Option<bool>,
    pub accept_first_mouse: Option<bool>,
    pub back_forward_navigation_gestures: Option<bool>,
    pub incognito: Option<bool>,
    pub autoplay: Option<bool>,
    pub focused: Option<bool>,
}

fn ensure_valid_url(url: Option<String>) -> Option<String> {
    url.map(|mut u| {
        let valid_protocols = ["http://", "https://", "ftp://", "file://", "ws://", "wss://"];
        if !valid_protocols.iter().any(|protocol| u.starts_with(protocol)) {
            u = format!("pywui://pywui/{}", u);
        }
        u
    })
}

impl From<WebViewAttributesConfig> for WebViewAttributes<'static> {
    fn from(config: WebViewAttributesConfig) -> Self {
        WebViewAttributes {
            user_agent: config.user_agent,
            visible: config.visible.unwrap_or(true),
            transparent: config.transparent.unwrap_or(false),
            background_color: config.background_color,
            url: ensure_valid_url(config.url),
            zoom_hotkeys_enabled: config.zoom_hotkeys_enabled.unwrap_or(true),
            html: config.html,
            initialization_scripts: config.initialization_scripts.unwrap_or_default(),
            custom_protocols: HashMap::new(),
            clipboard: config.clipboard.unwrap_or(false),
            devtools: config.devtools.unwrap_or(true),
            accept_first_mouse: config.accept_first_mouse.unwrap_or(false),
            back_forward_navigation_gestures: config.back_forward_navigation_gestures.unwrap_or(false),
            document_title_changed_handler: None, // Cannot deserialize, set as None
            incognito: config.incognito.unwrap_or(false),
            autoplay: config.autoplay.unwrap_or(false),
            focused: config.focused.unwrap_or(false),
            ..WebViewAttributes::default()
        }
    }
}
