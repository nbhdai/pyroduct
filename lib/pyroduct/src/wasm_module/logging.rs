use std::sync::Once;

#[cfg(not(test))]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: usize);
}

// MOCK: If testing, provide a dummy implementation to satisfy the linker.
#[cfg(test)]
unsafe extern "C" fn host_log(ptr: *const u8, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    if let Ok(s) = std::str::from_utf8(slice) {
        println!("[HOST LOG MOCK]: {}", s);
    }
}

struct HostLogger;
impl std::io::Write for HostLogger {
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

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for HostLogger {
    type Writer = HostLogger;
    fn make_writer(&'a self) -> Self::Writer {
        HostLogger
    }
}

static INIT: Once = Once::new();
#[cfg(not(test))]
pub fn init_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_writer(HostLogger)
            .without_time()
            .with_ansi(false)
            .init();
    });
}
#[cfg(test)]
pub fn init_logging() {}