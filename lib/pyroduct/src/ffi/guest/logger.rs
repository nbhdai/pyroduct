use std::sync::{Mutex, Once, OnceLock};

use crate::ffi::interface::LogCallback;

// Global storage for the log callback - shared across all plugins
static LOG_CALLBACK: OnceLock<(i64, LogCallback)> = OnceLock::new();

pub struct PyroSpanLayer;

impl<S> tracing_subscriber::Layer<S> for PyroSpanLayer 
where S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> 
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.lookup_current() {
            let id = span.id().into_u64(); // Convert Id to FFI-safe u64
            
            if let Some((library_id, writer)) = LOG_CALLBACK.get() {
                
                    let mut msg = String::new();
                    event.record(&mut StringVisitor(&mut msg));
                    queue.push(msg);
            }
        }
    }
}

static INIT: Once = Once::new();
pub fn init_logging(id: i64, callback: LogCallback) {
    if LOG_CALLBACK.set((id, callback)).is_err() {
        tracing::error!("Double set tracing callback, plugin double imported");
    }
    let host_logger = HostLogger;

    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_writer(host_logger)
            .without_time()
            .with_ansi(false)
            .init();
    });
}

struct HostLogger;

impl std::io::Write for HostLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let callback = match LOG_CALLBACK.get() {
            Some(callback) => callback,
            None => panic!("Logging Callback isn't set"),
        };
        let callback_mut = callback.lock().unwrap();
        unsafe {
            (callback_mut.1)(callback_mut.0, buf.as_ptr(), buf.len());
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for HostLogger {
    type Writer = HostLogger;
    fn make_writer(&'a self) -> Self::Writer {
        HostLogger
    }
}