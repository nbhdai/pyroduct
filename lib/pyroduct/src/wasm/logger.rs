use std::io::Write;
use std::sync::Once;

use tracing_subscriber::{
    Registry, fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt,
};

static INIT: Once = Once::new();

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: usize);
}

// MOCK: If testing, provide a dummy implementation to satisfy the linker.
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn host_log(ptr: *const u8, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    if let Ok(s) = std::str::from_utf8(slice) {
        println!("[HOST LOG MOCK]: {}", s);
    }
}

pub fn init_logging() {
    INIT.call_once(|| {
        Registry::default()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(WasmWriterFactory)
                    .with_target(true)
                    .without_time()
                    .with_ansi(false),
            )
            .init();
    });
}

// ============================================================================
// MakeWriter
// ============================================================================

struct WasmWriterFactory;

impl<'a> MakeWriter<'a> for WasmWriterFactory {
    type Writer = WasmProxy;

    fn make_writer(&'a self) -> Self::Writer {
        WasmProxy
    }
}

// ============================================================================
// Writer proxy — performs the FFI call
// ============================================================================

struct WasmProxy;

impl Write for WasmProxy {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            host_log(buf.as_ptr(), buf.len());
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
