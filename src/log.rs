use std::sync::Mutex;

use log::Log;

pub struct AppLogger {
    pub messages: Mutex<Vec<(log::Level, String)>>,
}

impl Log for AppLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // TODO: Investigate why wgpu_text warns about cache resizing.
        // TODO: Use an RGBA8Unorm framebuffer for compatibility with egui_wgpu?
        // Silence this error for now.
        metadata.level() <= log::Level::Warn
            && !metadata.target().starts_with("wgpu_text")
            && !metadata.target().starts_with("egui_wgpu")
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.messages
                .lock()
                .unwrap()
                .push((record.level(), format!("{}", record.args())));
        }
    }

    fn flush(&self) {}
}
