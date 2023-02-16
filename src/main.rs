use directories::UserDirs;
use ini::ini;
use notify_rust::Notification;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() {
    loop {
        let user_dirs = UserDirs::new().unwrap();
        let downloads_path = user_dirs.download_dir().unwrap();
        let config = ini!(downloads_path.join("janitor.ini").to_str().unwrap());

        let desination = config["destination"]["default"].clone().unwrap();
        let destination_path = Path::new(&desination);

        fs::create_dir_all(destination_path).unwrap();

        for file in config["sources"]
            .values()
            .filter_map(|f| f.clone().filter(|f| f.len() > 0))
        {
            let file_str = file.to_owned();

            if downloads_path.join(&file_str).exists() {
                match fs::copy(
                    downloads_path.join(&file_str),
                    destination_path.join(&file_str),
                ) {
                    Ok(_) => {
                        fs::remove_file(downloads_path.join(&file_str)).unwrap();
                        notify(
                            "Moved",
                            format!("Moved {} to {}", &file_str, &desination).as_str(),
                        );
                    }
                    Err(_) => {
                        notify(
                            "Failed",
                            format!("Failed to move {} to {}", &file_str, &desination).as_str(),
                        );
                        thread::sleep(Duration::from_secs(10));
                    }
                }
            }
        }

        // Sleep for 1 second before checking again
        thread::sleep(Duration::from_secs(1));
    }
}

fn notify(summary: &str, message: &str) {
    println!("{}", message);
    Notification::new()
        .appname("Janitor")
        .summary(summary)
        .body(message)
        .show()
        .unwrap();
}
