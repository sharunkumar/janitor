use std::process::Command;

use windres::Build;

fn main() {
    #[cfg(any(target_os = "windows"))]
    {
        Command::new("taskkill")
            .args(&["/F", "/IM", "janitor.exe"])
            .spawn()
            .unwrap(); // replace with your command and arguments
    }
    Build::new().compile("icons/app-resources.rc").unwrap();
    println!("cargo:rerun-if-changed=icons/app-resources.rc");
    println!("cargo:rerun-if-changed=icons/app-icon.ico");
}
