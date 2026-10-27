//! Fuzz the 48-byte header parser: arbitrary bytes in, no panic and no UB out.
//!
//! The header is the first thing that touches attacker- or peer-controlled
//! bytes, and `check_raw` is called on pointers that arrive across FFI. A panic
//! here would unwind across an ABI boundary; a missed bound would be read out of
//! range downstream. So both are fuzzed, and the invariant that a *passing*
//! header's claimed length actually fits is asserted rather than assumed.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pyroformat::buf::{DataStatus, HEADER_SIZE, PyroHeader, PyroParser, PyroRef};

/// 16-aligned so `check` sees the alignment it demands; the misaligned case is
/// covered by a unit test, since alignment is not something fuzzing varies.
#[repr(C, align(16))]
struct Aligned([u8; 256]);

fuzz_target!(|data: &[u8]| {
    let mut buf = Aligned([0u8; 256]);
    let n = data.len().min(buf.0.len());
    buf.0[..n].copy_from_slice(&data[..n]);
    let slice = &buf.0[..n.max(0)];

    // Slice form: must never panic, whatever the bytes say.
    let checked = PyroParser::check(slice);

    if checked.is_ok() {
        // A header that passed must be safe to build a PyroRef from, and the
        // resulting message must be inside the slice we were given.
        let r = PyroRef::try_from_slice(slice).expect("check passed but try_from_slice failed");
        assert!(HEADER_SIZE + r.payload().len() <= slice.len());
        assert_eq!(r.version(), pyroformat::buf::PROTOCOL_VERSION);
        assert_eq!(r.reserved(), 0, "check let a nonzero reserved byte through");
        // Accessors on validated bytes must not panic.
        let _ = r.schema_hash();
        let _ = r.flags();
        let _ = r.is_unit();
        let _ = r.status();
        let _ = r.trace_id();
        let _ = r.parent_span();
        let _ = r.is_sampled();
        let _ = r.timestamp();
        // is_unit implies the exact triple, never a partial match.
        if r.is_unit() {
            assert_eq!(r.payload_len(), 0);
            assert_eq!(r.schema_hash(), 0);
            assert_eq!(r.status(), Ok(DataStatus::Unit));
        }
    }

    // Raw form, the FFI entry point. Only run when there are enough bytes to
    // read a header from; check_raw's contract requires HEADER_SIZE readable.
    if n >= HEADER_SIZE {
        let raw = unsafe { PyroParser::check_raw(buf.0.as_ptr()) };
        // check() is strictly stronger: it also bounds the length.
        if checked.is_ok() {
            assert!(raw.is_ok(), "check accepted what check_raw rejected");
        }
        if raw.is_ok() {
            let len =
                u32::from_le_bytes([buf.0[0], buf.0[1], buf.0[2], buf.0[3]]) as usize;
            // check_strict_raw must agree with the length it was given.
            assert!(unsafe { PyroParser::check_strict_raw(buf.0.as_ptr(), len) }.is_ok());
            if len > 0 {
                assert!(
                    unsafe { PyroParser::check_strict_raw(buf.0.as_ptr(), len - 1) }.is_err()
                );
            }
        }
    }
});
