fn main() {
    #[cfg(any(target_os = "windows"))]
    {
        windres::Build::new()
            .compile("icons/app-resources.rc")
            .unwrap();
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=icons/");
    }
}
