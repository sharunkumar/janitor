use windres::Build;

fn main() {
    Build::new().compile("icons/app-resources.rc").unwrap();
    println!("cargo:rerun-if-changed=icons/app-resources.rc");
    println!("cargo:rerun-if-changed=icons/app-icon.ico");
}
