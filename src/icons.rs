use tray_item::IconSource;

pub fn get_app_icon() -> IconSource {
    #[cfg(all(target_os = "windows"))]
    return IconSource::Resource("aa-exe-icon");

    #[cfg(not(target_os = "windows"))]
    return IconSource::Resource("user-trash");
}

pub fn get_blue_icon() -> IconSource {
    #[cfg(all(target_os = "windows"))]
    return IconSource::Resource("fire-blue");

    #[cfg(not(target_os = "windows"))]
    return IconSource::Resource("user-trash-full");
}
