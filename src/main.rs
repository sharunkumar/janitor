#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod icons;
use config::*;
use glob::*;
use icons::get_app_icon;
use icons::get_blue_icon;
use lazy_static::lazy_static;
use notify_debouncer_mini::*;
use notify_rust::Notification;
use std::env;
use std::fs;
use std::path::*;
use std::sync::*;
use std::thread;
use std::time::Duration;
use tray_item::*;

lazy_static! {
    static ref CONFIG_PATH: PathBuf = get_config_path();
    static ref CONFIG: Mutex<JanitorConfig> = Mutex::new(read_config());
    static ref TRAY: Mutex<TrayItem> =
        Mutex::new(TrayItem::new("Janitor", get_app_icon()).unwrap());
}

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(arg1) = env::args().nth(1) {
            if arg1.eq_ignore_ascii_case("systemd") {
                let service = include_str!("systemd/janitor.service");
                println!("{}", service);
                std::process::exit(0);
            }
        }
    }

    if is_systemd() {
        println!(
            "{} v{} running under systemd",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
    }

    // check if its a single instance
    let instance = single_instance::SingleInstance::new(env!("CARGO_PKG_NAME")).unwrap();

    if !instance.is_single() {
        eprintln!("Another instance of janitor is already running");
        std::process::exit(1);
    }

    startup_run();

    let rx_tray = setup_tray();

    let mut debouncer = new_debouncer_opt::<_, notify::RecommendedWatcher>(
        notify_debouncer_mini::Config::default(),
        DownloadHandler,
    )
    .unwrap();

    debouncer
        .watcher()
        .watch(
            &CONFIG_PATH.parent().unwrap(),
            notify::RecursiveMode::NonRecursive,
        )
        .unwrap();

    loop {
        match rx_tray.recv() {
            Ok(TrayMessage::Quit) => {
                println!("Quit");
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

fn is_systemd() -> bool {
    env::var("SYSTEMD").is_ok()
}

fn refresh_config() {
    let new_config = read_config();
    let mut config = CONFIG.lock().unwrap();
    config.patterns = new_config.patterns;
    // println!("New config: {:?}", &config);
}

struct DownloadHandler;

impl DebounceEventHandler for DownloadHandler {
    fn handle_event(&mut self, event: DebounceEventResult) {
        let Ok(event) = event else { return };
        println!("Event: {:?}", event);

        // check if config changed
        if event
            .iter()
            .any(|e| e.path.as_os_str() == CONFIG_PATH.as_os_str())
        {
            // if config is changed, dry run
            println!("config changed!");
            refresh_config();
            startup_run();
            return;
        }

        // else lazer down the files changed, and move them if needed
        let any: Vec<&PathBuf> = event
            .iter()
            .filter(|e| e.kind == DebouncedEventKind::Any && e.path.exists() && e.path.is_file())
            .map(|e| &e.path)
            .collect();

        if any.len() > 0 {
            refresh_config();
            let config = CONFIG.lock().unwrap();
            let patterns: Vec<(Pattern, &String)> = config
                .patterns
                .iter()
                .filter_map(|collec| Pattern::new(&collec.0).ok().map(|p| (p, &collec.1)))
                .collect();

            let result = any
                .iter()
                .filter_map(|&path| {
                    // dbg!(&path);
                    let matching = &patterns
                        .iter()
                        .filter(|collec| collec.0.matches_path(&path.as_path()))
                        .nth(0);

                    // dbg!(&path);

                    matching.map(|collec| {
                        (
                            path,
                            PathBuf::from(collec.1.as_str()).join(&path.file_name().unwrap()),
                        )
                    })
                })
                .filter_map(|(from, to)| {
                    let movef = move_file(from.to_owned(), to.to_owned());
                    // dbg!(&movef);
                    movef.ok()
                })
                .count();

            report_files_moved(result);
        }
    }
}

fn blink_tray(n: usize) {
    thread::spawn(move || {
        let mut tray = TRAY.lock().unwrap();

        for _ in 0..n {
            tray.set_icon(get_blue_icon()).unwrap();
            thread::sleep(Duration::from_millis(250));
            tray.set_icon(get_app_icon()).unwrap();
            thread::sleep(Duration::from_millis(250));
        }
    });
}

fn startup_run() {
    let config = CONFIG.lock().unwrap();

    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let result: usize = config
        .patterns
        .iter()
        .map(|(pattern, destination)| (pattern, Path::new(&destination).to_owned()))
        .map(|(pattern, destination_path)| {
            fs::create_dir_all(&destination_path).unwrap();
            // get files from downloads directory that match pattern
            let path_and_pattern = CONFIG_PATH.parent().unwrap().join(&pattern);
            let paths = glob_with(path_and_pattern.to_str().unwrap(), options).unwrap();
            (paths, destination_path)
        })
        .map(|(p, d)| move_files(p, d))
        .sum();

    report_files_moved(result);
}

fn report_files_moved(result: usize) {
    if result > 0 {
        blink_tray(result);
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
        .map(|(from, to)| move_file(from, to))
        .filter_map(|f| f.ok())
        .count()
}

fn move_file(from: PathBuf, to: PathBuf) -> Result<(), std::io::Error> {
    // dbg!(&from, &to);s
    // thread::sleep(Duration::from_secs(2));
    fs::rename(&from, &to).or_else(|_| {
        // try copy and delete if that does not work
        fs::copy(&from, &to).and_then(|_| {
            fs::remove_file(&from).unwrap();
            Ok(())
        })
    })
}

fn setup_tray() -> std::sync::mpsc::Receiver<TrayMessage> {
    let mut tray = TRAY.lock().unwrap();

    if !is_systemd() {
        tray.add_label(
            format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).as_str(),
        )
        .unwrap();
    } else {
        tray.add_label(
            format!(
                "{} v{} - systemd",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )
            .as_str(),
        )
        .unwrap();
    }

    let (tx, rx) = mpsc::sync_channel(1);

    #[cfg(all(target_os = "windows"))]
    tray.inner_mut().add_separator().unwrap();

    if !is_systemd() {
        let quit_tx = tx.clone();
        tray.add_menu_item("Quit", move || {
            quit_tx.send(TrayMessage::Quit).unwrap();
        })
        .unwrap();
    }

    rx
}

fn read_config() -> JanitorConfig {
    let config_path = CONFIG_PATH.to_owned();

    if !(config_path.exists()) {
        app_message("No config file found", "writing default config");
        write_default_config(&config_path);
    }

    let configuration = toml::from_str::<JanitorConfig>(&fs::read_to_string(&config_path).unwrap());

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
    // if a file already exists at the path, rename it
    if config_path.exists() {
        println!("Renaming existing config");
        fs::rename(
            &config_path,
            &config_path.with_file_name("janitor-old.toml"),
        )
        .unwrap();
    }

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
