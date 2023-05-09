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
    std::thread::spawn(|| loop {
        app_logic();
        thread::sleep(Duration::from_secs(1));
    });

    std::thread::spawn(|| {
        let (rx_tray, mut tray) = setup_tray();
        loop {
            match rx_tray.recv() {
                Ok(Message::Quit) => {
                    println!("Quit");
                    std::process::exit(0);
                }
                Ok(Message::Red) => {
                    println!("Red");
                    tray.set_icon(IconSource::Resource("another-name-from-rc-file"))
                        .unwrap();
                }
                Ok(Message::Green) => {
                    println!("Green");
                    tray.set_icon(IconSource::Resource("name-of-icon-in-rc-file"))
                        .unwrap()
                }
                _ => {}
            }
        }
    });

    let rx_config = setup_config_watcher();

    loop {
        match rx_config.recv() {
            Ok(_event) => {
                println!("Config changed");
                let new_config = read_config();
                let mut config = CONFIG.lock().unwrap();
                config.patterns = new_config.patterns;
            }
            _ => {}
        }
    }
}

fn setup_config_watcher(
) -> mpsc::Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, Vec<notify::Error>>> {
    let (tx, rx) = mpsc::channel();
    // setup_tray(tx);
    let mut debouncer = new_debouncer_opt::<_, notify::RecommendedWatcher>(
        Duration::from_millis(500),
        None,
        tx,
        notify::Config::default(),
    )
    .unwrap();

    debouncer
        .watcher()
        .watch(
            get_config_path().as_path(),
            notify::RecursiveMode::NonRecursive,
        )
        .unwrap();
    rx
}

fn setup_tray() -> (std::sync::mpsc::Receiver<Message>, TrayItem) {
    let mut tray = TrayItem::new(
        "Tray Example",
        IconSource::Resource("name-of-icon-in-rc-file"),
    )
    .unwrap();

    tray.add_label("Tray Label").unwrap();

    tray.add_menu_item("Hello", || {
        println!("Hello!");
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    let (tx, rx) = mpsc::channel();

    let red_tx = get_thread_sender(&tx);
    tray.add_menu_item("Red", move || {
        red_tx.lock().unwrap().send(Message::Red).unwrap();
    })
    .unwrap();

    let green_tx = get_thread_sender(&tx);
    tray.add_menu_item("Green", move || {
        green_tx.lock().unwrap().send(Message::Green).unwrap();
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    let quit_tx = get_thread_sender(&tx);
    tray.add_menu_item("Quit", move || {
        quit_tx.lock().unwrap().send(Message::Quit).unwrap();
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

fn app_logic() {
    let config = CONFIG.lock().unwrap();
    for (pattern, destination) in config.patterns.to_owned() {
        let destination_path = Path::new(&destination);
        fs::create_dir_all(destination_path).unwrap();

        // get files from downloads directory that match pattern
        let path_and_pattern = get_config_path().parent().unwrap().join(&pattern);

        for entry in glob_with(
            path_and_pattern.to_str().unwrap(),
            MatchOptions {
                case_sensitive: false,
                require_literal_separator: false,
                require_literal_leading_dot: false,
            },
        )
        .unwrap()
        {
            if let Ok(path) = entry {
                app_message(
                    "Moving",
                    format!("{} to {}", &path.display(), &destination_path.display()).as_str(),
                );
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
                    Ok(_) => (),
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

enum Message {
    Quit,
    Green,
    Red,
}

fn get_thread_sender(sender: &mpsc::Sender<Message>) -> Arc<Mutex<mpsc::Sender<Message>>> {
    let tx = sender.clone();
    let sender = Arc::new(Mutex::new(tx));
    let thread_sender = sender.clone();
    thread_sender
}
