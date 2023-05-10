#![windows_subsystem = "windows"]

mod config;
use config::*;
use glob::*;
use lazy_static::lazy_static;
use notify_debouncer_mini::*;
use notify_rust::Notification;
use std::fs;
use std::path::*;
use std::sync::*;
use std::thread;
use std::time::Duration;
use tray_item::*;

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

    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let result: usize = config
        .patterns
        .to_owned()
        .into_iter()
        .map(|(pattern, destination)| (pattern, Path::new(&destination).to_owned()))
        .map(|(pattern, destination_path)| {
            fs::create_dir_all(&destination_path).unwrap();
            // get files from downloads directory that match pattern
            let path_and_pattern = get_config_path().parent().unwrap().join(&pattern);
            let paths = glob_with(path_and_pattern.to_str().unwrap(), options).unwrap();
            (paths, destination_path)
        })
        .map(|(p, d)| move_files(p, d))
        .sum();

    if result > 0 {
        app_message(
            "Moved",
            format!("{} {}", result, if result > 1 { "files" } else { "file" }).as_str(),
        );
    }
}

/// moves the files from the `paths` to `destination_path`
fn move_files(paths: Paths, destination_path: PathBuf) -> usize {
    paths
        .filter_map(|f| f.ok())
        .map(|path| {
            (
                path.to_owned(),
                destination_path.join(&path.file_name().unwrap()),
            )
        })
        .map(|(from, to)| {
            fs::rename(&from, &to).or_else(|_| {
                // try copy and delete if that does not work
                fs::copy(&from, &to).and_then(|_| {
                    fs::remove_file(&from).unwrap();
                    Ok(())
                })
            })
        })
        .filter_map(|f| f.ok())
        .count()
}

fn setup_tray() -> (std::sync::mpsc::Receiver<TrayMessage>, TrayItem) {
    let mut tray = TrayItem::new("Janitor", IconSource::Resource("aa-exe-icon")).unwrap();

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
