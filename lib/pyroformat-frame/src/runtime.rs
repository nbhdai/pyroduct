//! A CPU runtime for compression and decoding beside compio's IO threads.

use vortex_io::runtime::BlockingRuntime;
use vortex_io::runtime::Handle;
use vortex_io::runtime::current::CurrentThreadRuntime;
use vortex_io::runtime::current::CurrentThreadWorkerPool;
use vortex_io::session::RuntimeSessionExt;
use vortex_session::VortexSession;

/// Worker threads that run a session's compression and decoding while compio threads do IO.
///
/// Encoding and scanning spawn CPU work on the session's runtime handle. A compio thread never
/// drives a Vortex runtime itself, so that work needs threads of its own. The handle only holds a
/// weak reference, so keep this value alive for as long as sessions using it run.
pub struct CpuRuntime {
    runtime: CurrentThreadRuntime,
    pool: CurrentThreadWorkerPool,
}

impl CpuRuntime {
    /// A runtime with `workers` threads.
    pub fn new(workers: usize) -> Self {
        let runtime = CurrentThreadRuntime::new();
        let pool = runtime.new_pool();
        pool.set_workers(workers.max(1));
        Self { runtime, pool }
    }

    /// A runtime with one thread per available core.
    pub fn with_available_parallelism() -> Self {
        let runtime = CurrentThreadRuntime::new();
        let pool = runtime.new_pool();
        pool.set_workers_to_available_parallelism();
        Self { runtime, pool }
    }

    /// The number of worker threads.
    pub fn workers(&self) -> usize {
        self.pool.worker_count()
    }

    /// A handle that spawns onto this runtime.
    pub fn handle(&self) -> Handle {
        self.runtime.handle()
    }

    /// `session`, configured to spawn its CPU work onto this runtime.
    pub fn configure(&self, session: VortexSession) -> VortexSession {
        session.with_handle(self.handle())
    }
}
