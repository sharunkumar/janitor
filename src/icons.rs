use std::io::Cursor;

use tray_item::IconSource;

pub fn get_app_icon() -> IconSource {
    #[cfg(all(target_os = "windows"))]
    return IconSource::Resource("aa-exe-icon");

    #[cfg(not(target_os = "windows"))]
    return {
        let cursor_red = Cursor::new(include_bytes!("../icons/png/app-icon-32.png"));
        let decoder_red = png::Decoder::new(cursor_red);
        let (info_red, mut reader_red) = decoder_red.read_info().unwrap();
        let mut buf_red = vec![0; info_red.buffer_size()];
        reader_red.next_frame(&mut buf_red).unwrap();
        IconSource::Data {
            data: buf_red,
            height: 32,
            width: 32,
        }
    };
}

pub fn get_blue_icon() -> IconSource {
    #[cfg(all(target_os = "windows"))]
    return IconSource::Resource("fire-blue");

    #[cfg(not(target_os = "windows"))]
    return {
        let cursor_red = Cursor::new(include_bytes!("../icons/png/fire-blue-32.png"));
        let decoder_red = png::Decoder::new(cursor_red);
        let (info_red, mut reader_red) = decoder_red.read_info().unwrap();
        let mut buf_red = vec![0; info_red.buffer_size()];
        reader_red.next_frame(&mut buf_red).unwrap();
        IconSource::Data {
            data: buf_red,
            height: 32,
            width: 32,
        }
    };
}
