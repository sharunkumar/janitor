use directories::UserDirs;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

const FILE_NAME: &str = "example.txt";
const DESTINATION_FOLDER: &str = "E:\\janitor\\";

fn main() {
    let user_dirs = UserDirs::new().unwrap();
    let downloads_path = user_dirs.download_dir().unwrap();
    let destination_path = Path::new(DESTINATION_FOLDER);

    if !destination_path.exists() {
        fs::create_dir_all(destination_path).unwrap();
    }

    loop {
        // Check if the file is in the downloads folder

        if downloads_path.join(FILE_NAME).exists() {
            // Check if the file is still being written to
            fs::copy(
                downloads_path.join(FILE_NAME),
                destination_path.join(FILE_NAME),
            )
            .unwrap();

            fs::remove_file(downloads_path.join(FILE_NAME)).unwrap();

            println!("Moved {} to {}.", FILE_NAME, DESTINATION_FOLDER);
        }

        // Sleep for 1 second before checking again
        thread::sleep(Duration::from_secs(1));
    }
}
