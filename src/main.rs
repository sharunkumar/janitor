use directories::UserDirs;
use ini::ini;
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
                        println!("Moved {} to {}", &file_str, &desination);
                    }
                    Err(_) => {
                        println!(
                            "Failed to copy {} to {}. Waiting for 10 seconds...",
                            &file_str, &desination
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
