use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Once, OnceLock, RwLock};

use tracing::Metadata;
use tracing_subscriber::{
    Registry, fmt::MakeWriter, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt,
};

use crate::ffi::interface::LogCallback;

/// Stored in a span's extensions to mark it as belonging to a specific object.
pub struct ObjectId(pub u64);

// Global storage for the log callback
static LOG_CALLBACK: OnceLock<(i64, LogCallback)> = OnceLock::new();
static CLIENT_SPAN: OnceLock<RwLock<HashMap<u64, tracing::Span>>> = OnceLock::new();

static INIT: Once = Once::new();

pub fn init_logging(id: i64, callback: LogCallback) {
    INIT.call_once(|| {
        let _ = LOG_CALLBACK.set((id, callback));
        let factory = FfiWriterFactory;

        Registry::default()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(factory)
                    .with_target(true)
                    .without_time()
                    .with_ansi(false),
            )
            .init();
    });
}

/// Create a tracing span for a specific object and attach the [`ObjectId`] to
/// its extensions. The returned span should be entered whenever work is done on
/// behalf of that object — all log lines emitted inside will carry the object
/// ID across the FFI boundary.
///
/// ```ignore
/// let span = pyroduct::ffi::guest::logger::object_span(42);
/// let _guard = span.enter();
/// tracing::info!("routed to object 42");
/// ```
pub fn object_span(object_id: u64) -> tracing::Span {
    let client_spans = CLIENT_SPAN.get_or_init(|| RwLock::new(HashMap::new()));
    {
        let cs = client_spans.read().unwrap();
        if let Some(span) = cs.get(&object_id) {
            return span.clone();
        }
    }
    {
        let mut cs = client_spans.write().unwrap();
        let span = tracing::span!(tracing::Level::INFO, "object", id = object_id);
        tracing::dispatcher::get_default(|dispatch| {
            if let Some(id) = span.id() {
                if let Some(reg) = dispatch.downcast_ref::<Registry>() {
                    if let Some(span_ref) = reg.span(&id) {
                        span_ref.extensions_mut().insert(ObjectId(object_id));
                    }
                }
            }
        });
        cs.insert(object_id, span.clone());
        span
    }
}
// ============================================================================
// MakeWriter
// ============================================================================

struct FfiWriterFactory;

impl<'a> MakeWriter<'a> for FfiWriterFactory {
    type Writer = FfiProxy;

    fn make_writer(&'a self) -> Self::Writer {
        FfiProxy { object_id: None }
    }

    fn make_writer_for(&'a self, _meta: &Metadata<'_>) -> Self::Writer {
        let mut target_id = None;

        let current = tracing::Span::current();
        if let Some(current_id) = current.id() {
            tracing::dispatcher::get_default(|dispatch| {
                if let Some(reg) = dispatch.downcast_ref::<Registry>() {
                    if let Some(span_ref) = reg.span(&current_id) {
                        for ancestor in span_ref.scope() {
                            if let Some(oid) = ancestor.extensions().get::<ObjectId>() {
                                target_id = Some(oid.0);
                                break;
                            }
                        }
                    }
                }
            });
        }

        FfiProxy {
            object_id: target_id,
        }
    }
}

// ============================================================================
// Writer proxy — performs the FFI call
// ============================================================================

struct FfiProxy {
    object_id: Option<u64>,
}

impl Write for FfiProxy {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let (lib_id, call) = match LOG_CALLBACK.get() {
            Some(cb) => cb,
            None => panic!("Logging Callback isn't set"),
        };

        unsafe {
            (call)(
                *lib_id,
                self.object_id.unwrap_or(0),
                buf.as_ptr(),
                buf.len(),
            );
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
