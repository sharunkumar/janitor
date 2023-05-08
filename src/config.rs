use serde::{Deserialize, Serialize};

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
