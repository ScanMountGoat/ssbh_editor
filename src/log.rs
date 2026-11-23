use std::sync::{Arc, Mutex};

use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

// Store formatted log messages using the formatting from tracing_subscriber::fmt.
#[derive(Clone)]
pub struct AppLogger {
    pub messages: Arc<Mutex<Vec<(tracing::Level, String)>>>,
}

impl AppLogger {
    pub fn new() -> Self {
        Self {
            messages: Default::default(),
        }
    }
}

impl std::io::Write for AppLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let line = String::from_utf8_lossy(buf).trim().to_string();
        if !line.is_empty() {
            // TODO: Is there a way to use the existing formatting but store the level separately?
            // Just strip the level for now.
            let (level, message) = line
                .strip_prefix("ERROR ")
                .map(|l| (Level::ERROR, l))
                .or_else(|| line.strip_prefix("WARN ").map(|l| (Level::WARN, l)))
                .or_else(|| line.strip_prefix("TRACE ").map(|l| (Level::TRACE, l)))
                .or_else(|| line.strip_prefix("INFO ").map(|l| (Level::INFO, l)))
                .or_else(|| line.strip_prefix("DEBUG ").map(|l| (Level::DEBUG, l)))
                .unwrap_or((Level::INFO, &line));

            self.messages
                .lock()
                .unwrap()
                .push((level, message.to_string()));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for AppLogger {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

// impl<S> Layer<S> for AppLogger
// where
//     S: Subscriber,
//     S: for<'a> LookupSpan<'a>,
// {
//     fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
//         let level = *event.metadata().level();
//         if level < Level::WARN {
//             return;
//         }

//         let mut buf = String::new();
//         let writer = Writer::new(&mut buf);

//         DefaultFields::new().make_visitor(target)
//         let test = ctx.field_format();
//         if self
//             .formatter
//             .format_event(
//                 &ctx.fmt_context(DefaultFields::default(), ()),
//                 writer,
//                 event,
//             )
//             .is_ok()
//         {
//             // Trim trailing newline added by formatting.
//             let message = buf.trim_end_matches('\n').to_owned();
//             if let Ok(mut guard) = self.messages.lock() {
//                 guard.push((level, message));
//             }
//         }
//     }
// }
