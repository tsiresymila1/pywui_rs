use std::env::consts::OS;
use serde::Deserialize;

use crate::window::WindowAttributesConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Icons {
    pub linux: Option<String>,
    pub macos: Option<String>,
    pub windows: Option<String>
}

impl Icons {
    pub fn get_for_current_os(&self) -> Option<String> {
        match OS {
            "linux" => self.linux.clone(),
            "macos" => self.macos.clone(),
            "windows" => self.windows.clone(),
            _ => None,
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub build: Build,
    pub package: Package,
    pub pywui: Pywui,
    pub icon: Icons
}

#[derive(Debug, Deserialize, Clone)]
pub struct Build {
    #[serde(rename = "beforeBuildCommand")]
    pub before_build_command: String,

    #[serde(rename = "beforeDevCommand")]
    pub before_dev_command: String,

    #[serde(rename = "devPath")]
    pub dev_path: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct Package {
    #[serde(rename = "productName")]
    pub product_name: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pywui {
    pub windows: Vec<WindowAttributesConfig>,
}


