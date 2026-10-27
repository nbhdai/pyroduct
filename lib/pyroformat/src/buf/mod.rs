//! The buffer family: one 16-aligned layout, five handle types, a type-state
//! split between *building* and *sending*.
//!
//! ```text
//!       base (16-aligned, allocation start)
//!    -32 ..-16   LOCAL PREFIX A — release vtable (§4)
//!                  ctx *mut c_void | vtable *const PyroReleaseVTable
//!    -16 .. 0    LOCAL PREFIX B — bookkeeping
//!                  ref_count u32 | capacity u32 | pool_class u16 |
//!                  local_flags u16 | generation u32
//!   ┌─ 0 ..16    MESSAGE HEADER — the transmitted 16 bytes (§3)
//!   │ 16 ..      payload — 16-aligned, exactly what the wire reader sees
//!   └─ this pointer is what crosses FFI / goes on the socket
//! ```
//!
//! The prefix is wire-invisible: 32 bytes of local bookkeeping at a negative
//! offset from the message pointer, costing not one transmitted byte and not
//! disturbing payload alignment (32 and 16 are both multiples of 16).
//! On-wire and across-FFI overhead is 16 bytes.
//!
//! All prefix and header access goes through raw pointer arithmetic off the
//! *message* pointer, which carries provenance for the whole allocation. No
//! reference is ever created that covers less than the allocation and then used
//! to reach past itself — that is fix (4) of [docs/vec-plan.md §2].
//!
//! The module is split by *role*, not by type count: the layout is one thing,
//! the release seam another, and each handle owns its own file so the unsafe
//! blocks that belong to it stay next to the invariant that justifies them.
//!
//! | file | contents |
//! | --- | --- |
//! | [`header`] | the 16-byte transmitted header, `DataStatus`, `schema_hash` |
//! | [`error`] | [`PyroError`], [`CapturedError`] |
//! | [`encoding`] | error-payload encoding and status parsing |
//! | [`layout`] | allocation geometry — offsets, `layout_for`, alignment |
//! | [`release`] | the release seam: `PyroReleaseVTable`, the host releaser |
//! | [`prefix`] | raw accessors for the negative-offset prefix, and `release_one` |
//! | [`vec`] | [`PyroVec`] — unsealed, growable, self-owned |
//! | [`sealed`] | [`PyroBuf`] — sealed, fixed-capacity, refcounted |
//! | [`sink`] | [`PyroSink`] — one write target for [`PyroVec`] and [`PyroBuf`] |
//! | [`span`] | [`next_trace_id`] (u128), [`next_span`] (u64) — thread-local, ~1 ns |
//! | [`view`] | [`PyroView`] / [`PyroViewPtr`] — refcounted handles, `make_view` |
//! | [`reference`] | [`PyroRef`] / [`PyroRefPtr`] — borrowed and raw |
//! | `compio` | `compio_buf::IoBuf` for [`PyroBuf`] (`compio` feature) |

pub mod cart;
#[cfg(feature = "compio")]
pub mod compio;
pub mod encoding;
pub mod error;
pub mod header;
pub mod layout;
pub mod prefix;
pub mod reference;
pub mod release;
pub mod sealed;
pub mod sink;
pub mod span;
pub mod vec;
pub mod view;

pub use cart::{CartOwner, CovariantRef, PyroCart};
pub use error::{CapturedError, PyroError};
pub use header::{
    Allocation, DataStatus, FLAG_SAMPLED, HasHeader, HasHeaderMut, PROTOCOL_VERSION, ParseError,
    PyroHeader, PyroHeaderMut, PyroParser, UNIT_BYTES, UNIT_HEADER, schema_hash,
};
pub use layout::{ALIGN, FLAG_POOL_OWNED, FLAG_SEALED, HEADER_SIZE, MAX_PAYLOAD, PREFIX_SIZE};
pub use reference::{PyroRef, PyroRefPtr};
pub use release::{HOST_RELEASE_VTABLE, PyroReleaseVTable};
pub use sealed::PyroBuf;
pub use sink::{LocalMemSink, PyroSink};
pub use span::{next_span, next_trace_id, reseed_span};
pub use vec::PyroVec;
pub use view::{PyroView, PyroViewPtr, make_view};
