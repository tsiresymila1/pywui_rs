use serde::Deserialize;

use crate::window::WindowAttributesConfig;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub build: Build,
    pub package: Package,
    pub pywui: Pywui,
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


