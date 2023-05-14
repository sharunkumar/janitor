use std::process::Command;

use windres::Build;

fn main() {
    #[cfg(any(target_os = "windows"))]
    {
        Command::new("taskkill")
            .args(&["/F", "/IM", "janitor.exe"])
            .spawn()
            .unwrap();
    }
    Build::new().compile("icons/app-resources.rc").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icons/");
}
