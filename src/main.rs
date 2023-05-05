use directories::UserDirs;
use glob::{glob_with, MatchOptions};
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    example: Vec<(String, String)>,
    patterns: Vec<(String, String)>,
}

impl Default for Config {
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

fn main() {
    loop {
        let configuration = read_config();
        app_logic(configuration);
        thread::sleep(Duration::from_secs(1));
    }
}

fn read_config() -> Config {
    let config_path = get_config_path();

    if !(config_path.exists()) {
        app_message("No config file found", "writing default config");
        write_default_config(&config_path);
    }

    let configuration = toml::from_str::<Config>(&fs::read_to_string(&config_path).unwrap());

    match configuration {
        Ok(clean_config) => clean_config,
        Err(_) => {
            // in case of error, write default configuration
            app_message("Invalid config file", "writing default config");
            write_default_config(&config_path);
            read_config()
        }
    }
}

fn write_default_config(config_path: &PathBuf) {
    fs::write(&config_path, toml::to_string(&Config::default()).unwrap()).unwrap();
    app_message("Config file created", &config_path.to_str().unwrap());
}

fn get_downloads_path() -> PathBuf {
    UserDirs::new().unwrap().download_dir().unwrap().to_owned()
}

fn get_config_path() -> PathBuf {
    get_downloads_path().join("janitor.toml")
}

fn app_logic(configuration: Config) {
    for (pattern, destination) in configuration.patterns {
        let destination_path = Path::new(&destination);
        fs::create_dir_all(destination_path).unwrap();
        // get files from downloads directory that match pattern
        let options = MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        };

        let path_and_pattern = get_config_path().parent().unwrap().join(&pattern);

        for entry in glob_with(path_and_pattern.to_str().unwrap(), options).unwrap() {
            if let Ok(path) = entry {
                app_message("Moving", path.to_str().unwrap());
                match fs::copy(
                    &path,
                    destination_path.join(&path.file_name().unwrap().to_str().unwrap()),
                ) {
                    Ok(_) => fs::remove_file(path).unwrap(),
                    Err(_) => {
                        app_message(
                            "Move Failed",
                            format!(
                                "Could not move file: {}\nWill try again in next cycle",
                                path.to_str().unwrap()
                            )
                            .as_str(),
                        );
                    }
                }
            }
        }
    }
}

fn app_message(summary: &str, message: &str) {
    println!("{} {}", summary, message);
    Notification::new()
        .appname("Janitor")
        .summary(summary)
        .body(message)
        .show()
        .unwrap();
}
