use directories::UserDirs;
use ini::ini;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

const DESTINATION_FOLDER: &str = "E:\\janitor\\";

fn main() {
    let user_dirs = UserDirs::new().unwrap();
    let downloads_path = user_dirs.download_dir().unwrap();
    let destination_path = Path::new(DESTINATION_FOLDER);

    let config = ini!(downloads_path.join("janitor.ini").to_str().unwrap());

    if !destination_path.exists() {
        fs::create_dir_all(destination_path).unwrap();
    }

    loop {
        // Check if the file is in the downloads folder

        for file in config["sources"]
            .values()
            .filter_map(|f| f.clone().filter(|f| f.len() > 0))
        {
            let file_str = file.to_owned();

            if downloads_path.join(&file_str).exists() {
                // Check if the file is still being written to
                fs::copy(
                    downloads_path.join(&file_str),
                    destination_path.join(&file_str),
                )
                .unwrap();

                fs::remove_file(downloads_path.join(&file_str)).unwrap();

                println!("Moved {} to {}.", &file_str, DESTINATION_FOLDER);
            }
        }

        // Sleep for 1 second before checking again
        thread::sleep(Duration::from_secs(1));
    }
}
