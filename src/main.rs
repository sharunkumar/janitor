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
    static ref CONFIG_PATH: PathBuf = get_config_path();
    static ref CONFIG: Mutex<Config> = Mutex::new(read_config());
    static ref TRAY: Mutex<TrayItem> =
        Mutex::new(TrayItem::new("Janitor", IconSource::Resource("aa-exe-icon")).unwrap());
}

fn main() {
    // check if its a single instance
    let instance = single_instance::SingleInstance::new("janitor").unwrap();

    if !instance.is_single() {
        println!("Another instance of janitor is already running");
        std::process::exit(1);
    }

    startup_run();

    let rx_tray = setup_tray();

    let mut debouncer = new_debouncer_opt::<_, notify::RecommendedWatcher>(
        Duration::from_millis(500),
        None,
        DownloadHandler,
        notify::Config::default(),
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
        // println!("Event: {:?}", event);
        let any: Vec<PathBuf> = event
            .into_iter()
            .filter(|e| e.kind == DebouncedEventKind::Any && e.path.exists() && e.path.is_file())
            .map(|e| e.path)
            .collect();

        if &any
            .clone()
            .into_iter()
            .any(|f| f.as_os_str() == CONFIG_PATH.as_os_str())
            == &true
        {
            println!("config changed!");
            refresh_config();
            startup_run();
            return;
        }

        if &any.len() > &0 {
            refresh_config();
            let config = CONFIG.lock().unwrap();
            let patterns: Vec<(Pattern, String)> = config
                .patterns
                .clone()
                .into_iter()
                .filter_map(|collec| Pattern::new(&collec.0).ok().map(|p| (p, collec.1)))
                .collect();

            let result = any
                .into_iter()
                .filter_map(|path| {
                    // dbg!(&path);
                    let matching = &patterns
                        .clone()
                        .into_iter()
                        .filter(|collec| collec.0.matches_path(&path.as_path()))
                        .nth(0);

                    // dbg!(&path);

                    matching.to_owned().map(|collec| {
                        (
                            path.to_owned(),
                            PathBuf::from(collec.1.as_str()).join(&path.file_name().unwrap()),
                        )
                    })
                })
                .filter_map(|(from, to)| {
                    let movef = move_file(from, to);
                    // dbg!(&movef);
                    movef.ok()
                })
                .count();

            blink_tray(result);
        }
    }
}

fn blink_tray(n: usize) {
    thread::spawn(move || {
        let mut tray = TRAY.lock().unwrap();

        for _ in 0..n {
            tray.set_icon(IconSource::Resource("fire-blue")).unwrap();
            thread::sleep(Duration::from_millis(250));
            tray.set_icon(IconSource::Resource("aa-exe-icon")).unwrap();
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
        .to_owned()
        .into_iter()
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

    tray.add_label("Janitor is running...").unwrap();

    let (tx, rx) = mpsc::channel();

    tray.inner_mut().add_separator().unwrap();

    let quit_tx = get_thread_sender(&tx);
    tray.add_menu_item("Quit", move || {
        quit_tx.lock().unwrap().send(TrayMessage::Quit).unwrap();
    })
    .unwrap();

    rx
}

fn read_config() -> Config {
    let config_path = CONFIG_PATH.to_owned();

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
