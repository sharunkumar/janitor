use directories::UserDirs;
use serde::*;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub patterns: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExampleConfig {
    pub example: Vec<(String, String)>,
    pub patterns: Vec<(String, String)>,
}

impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            example: vec![(
                "file*.pdf".to_string(),
                "C:\\Path\\To\\Folder\\".to_string(),
            )],
            patterns: Default::default(),
        }
    }
}

pub fn get_downloads_path() -> PathBuf {
    UserDirs::new().unwrap().download_dir().unwrap().to_owned()
}

pub fn get_config_path() -> PathBuf {
    get_downloads_path().join("janitor.toml")
}
