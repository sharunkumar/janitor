use directories::UserDirs;
use ini::ini;
use notify_rust::Notification;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

fn main() {
    loop {
        let config = config_from_path(get_downloads_path().join("janitor.ini").to_str().unwrap());
        run_once(config); // todo: learn how to pipe this?

        // Sleep for 1 second before checking again
        thread::sleep(Duration::from_secs(1));
    }
}

fn get_downloads_path() -> PathBuf {
    return UserDirs::new().unwrap().download_dir().unwrap().to_owned();
}

fn app_message(summary: &str, message: &str) {
    println!("{}", message);
    Notification::new()
        .appname("Janitor")
        .summary(summary)
        .body(message)
        .show()
        .unwrap();
}

struct Configuration {
    sources: Vec<String>,
    destination: String,
}

fn config_from_path(path: &str) -> Configuration {
    let config = ini!(path);

    for (section, properties) in config.iter() {
        if section == "sources" {
            let mut sources = Vec::new();

            for (_, value) in properties.iter() {
                sources.push(value.clone().unwrap());
            }

            return Configuration {
                sources,
                destination: config["destination"]["default"].clone().unwrap(),
            };
        }
    }

    panic!("No sources found in config");
}

fn run_once(config: Configuration) {
    let downloads_path = get_downloads_path();

    let destination_path = Path::new(&config.destination);

    fs::create_dir_all(destination_path).unwrap();

    for file in config.sources {
        let file_str = file.to_owned();

        if downloads_path.join(&file_str).exists() {
            match fs::copy(
                downloads_path.join(&file_str),
                destination_path.join(&file_str),
            ) {
                Ok(_) => {
                    fs::remove_file(downloads_path.join(&file_str)).unwrap();
                    app_message(
                        "Moved",
                        format!("Moved {} to {}", &file_str, &config.destination).as_str(),
                    );
                }
                Err(_) => {
                    app_message(
                        "Failed",
                        format!("Failed to move {} to {}", &file_str, &config.destination).as_str(),
                    );
                    thread::sleep(Duration::from_secs(10));
                }
            }
        }
    }
}
