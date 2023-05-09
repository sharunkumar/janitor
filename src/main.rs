#![windows_subsystem = "windows"]

mod config;
use config::{Config, ExampleConfig};
use directories::UserDirs;
use glob::{glob_with, MatchOptions};
use lazy_static::lazy_static;
use notify_debouncer_mini::new_debouncer_opt;
use notify_rust::Notification;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray_item::{IconSource, TrayItem};

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(read_config());
}

fn main() {
    // check if its a single instance
    let instance = single_instance::SingleInstance::new("janitor").unwrap();

    if !instance.is_single() {
        println!("Another instance of janitor is already running");
        std::process::exit(1);
    }

    std::thread::spawn(|| loop {
        app_logic();
        thread::sleep(Duration::from_secs(1));
    });

    std::thread::spawn(|| {
        let (rx_tray, mut _tray) = setup_tray();
        loop {
            match rx_tray.recv() {
                Ok(TrayMessage::Quit) => {
                    println!("Quit");
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer_opt::<_, notify::RecommendedWatcher>(
        Duration::from_millis(500),
        None,
        tx,
        notify::Config::default(),
    )
    .unwrap();

    debouncer
        .watcher()
        .watch(&get_config_path(), notify::RecursiveMode::NonRecursive)
        .unwrap();

    for events in rx {
        events.into_iter().for_each(|e| {
            println!("{:?}", e);
            let new_config = read_config();
            let mut config = CONFIG.lock().unwrap();
            config.patterns = new_config.patterns;
        });
    }
}

fn app_logic() {
    let config = CONFIG.lock().unwrap();
    for (pattern, destination) in config.patterns.to_owned() {
        let destination_path = Path::new(&destination);
        fs::create_dir_all(destination_path).unwrap();

        // get files from downloads directory that match pattern
        let path_and_pattern = get_config_path().parent().unwrap().join(&pattern);

        let glob = glob_with(
            path_and_pattern.to_str().unwrap(),
            MatchOptions {
                case_sensitive: false,
                require_literal_separator: false,
                require_literal_leading_dot: false,
            },
        )
        .unwrap();

        let mut count = 0;

        for entry in glob {
            if let Ok(path) = entry {
                // try with rename first
                match fs::rename(
                    &path,
                    destination_path.join(&path.file_name().unwrap().to_str().unwrap()),
                ) {
                    Err(_) => {
                        // try copy and delete if that does not work
                        match fs::copy(
                            &path,
                            destination_path.join(&path.file_name().unwrap().to_str().unwrap()),
                        ) {
                            Ok(_) => {
                                fs::remove_file(path).unwrap();
                                count += 1;
                            }
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
                    Ok(_) => {
                        count += 1;
                    }
                }
            }
        }

        if count > 0 {
            app_message("Moved", format!("{} files", count).as_str());
        }
    }
}

fn setup_tray() -> (std::sync::mpsc::Receiver<TrayMessage>, TrayItem) {
    let mut tray =
        TrayItem::new("Janitor", IconSource::Resource("name-of-icon-in-rc-file")).unwrap();

    tray.add_label("Janitor is running...").unwrap();

    let (tx, rx) = mpsc::channel();

    tray.inner_mut().add_separator().unwrap();

    let quit_tx = get_thread_sender(&tx);
    tray.add_menu_item("Quit", move || {
        quit_tx.lock().unwrap().send(TrayMessage::Quit).unwrap();
    })
    .unwrap();

    return (rx, tray);
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
    fs::write(
        &config_path,
        toml::to_string(&ExampleConfig::default()).unwrap(),
    )
    .unwrap();
    app_message("Config file created", &config_path.to_str().unwrap());
}

fn get_downloads_path() -> PathBuf {
    UserDirs::new().unwrap().download_dir().unwrap().to_owned()
}

fn get_config_path() -> PathBuf {
    get_downloads_path().join("janitor.toml")
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

enum TrayMessage {
    Quit,
}

fn get_thread_sender(sender: &mpsc::Sender<TrayMessage>) -> Arc<Mutex<mpsc::Sender<TrayMessage>>> {
    let tx = sender.clone();
    let sender = Arc::new(Mutex::new(tx));
    let thread_sender = sender.clone();
    thread_sender
}
