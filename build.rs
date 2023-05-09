use windres::Build;

fn main() {
    Build::new().compile("icons/app-resources.rc").unwrap();
}
