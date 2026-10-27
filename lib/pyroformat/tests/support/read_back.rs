use pyroformat::buf::sink::LocalMemSink;
use pyroformat::buf::vec::PyroVec;
use pyroformat::{Inode, Layout, Reader, Realize, Writer};

pub fn assert_roundtrip<T: Reader + Writer + Layout + PartialEq + std::fmt::Debug>(
    data: &T,
    golden_bytes: &[u8],
    golden_inodes: &[Inode],
) {
    assert_eq!(T::INODES, golden_inodes);
    let buf = data.encode().unwrap();
    assert!(data.fixed_len() + data.variable_length() >= golden_bytes.len());
    assert_eq!(buf.payload(), golden_bytes);

    assert_fully_written(data, golden_bytes);

    let restored = T::pyro_read(&buf).unwrap();
    let realized = restored.realize();
    assert_eq!(&realized, data);
}

/// [`assert_roundtrip`]'s write half, for a type the derive can't read back
/// into (`pyroformat::Map`).
pub fn assert_written<T: Writer + Layout>(data: &T, golden_bytes: &[u8], golden_inodes: &[Inode]) {
    assert_eq!(T::INODES, golden_inodes);
    let buf = data.encode().unwrap();
    assert!(data.fixed_len() + data.variable_length() >= golden_bytes.len());
    assert_eq!(buf.payload(), golden_bytes);

    assert_fully_written(data, golden_bytes);
}

/// What a fresh variable region is filled with instead of zeros, so a byte
/// no writer wrote reads back as it. Two, so a hole whose golden byte happens
/// to equal one still differs from the other.
const POISONS: [u8; 2] = [0xAA, 0x55];

/// Both writers write every byte of the message themselves, alignment
/// padding included: `LocalMemSink` leaves the variable region as the
/// allocator hands it over, so a byte they skip would go out as whatever was
/// there before.
///
/// Writes into a `LocalMemSink` whose variable region is poisoned rather than
/// zeroed, and checks it against the golden bytes, which were written into
/// zeroed memory: any byte left unwritten shows up as the poison where the
/// golden has its zero.
pub fn assert_fully_written<T: Writer + Layout>(data: &T, golden_bytes: &[u8]) {
    for poison in POISONS {
        let mut sink = poisoned_sink::<T>(golden_bytes.len(), poison);
        data.pyro_write(&mut sink).unwrap();
        assert_unwritten("reg", &sink, golden_bytes, poison);
    }
}

/// A sink over a buffer whose fixed region is zeroed, as `LocalMemSink`
/// makes it, and whose variable region is all `poison`. It is sized well past
/// the message — past the pre writer's loose bound too — so neither writer
/// grows out of it into a fresh, unpoisoned buffer.
fn poisoned_sink<T: Layout>(message_len: usize, poison: u8) -> LocalMemSink {
    let fixed = <T as Layout>::FIXED_SIZE;
    let len = 16 * message_len + 4096;
    let mut vec = PyroVec::with_capacity(len);
    vec.resize(fixed, 0);
    vec.resize(len, poison);
    LocalMemSink {
        schema_hash: <T as Layout>::SCHEMA_HASH,
        buf: vec.seal(<T as Layout>::SCHEMA_HASH),
        fixed,
        variable: len - fixed,
    }
}

/// What `path` wrote into `sink` against `golden`, naming the first byte it
/// left as `poison`.
fn assert_unwritten(path: &str, sink: &LocalMemSink, golden: &[u8], poison: u8) {
    assert_eq!(
        sink.buf.capacity(),
        16 * golden.len() + 4096,
        "{path} writer grew out of the poisoned buffer"
    );
    let written = sink.buf.payload();
    if let Some(at) = written.iter().zip(golden).position(|(w, g)| w != g) {
        panic!(
            "{path} writer, variable region poisoned with {poison:#04x}: byte {at} is {:#04x}, \
             golden {:#04x} ({})",
            written[at],
            golden[at],
            if written[at] == poison {
                "left unwritten"
            } else {
                "written wrong"
            },
        );
    }
    assert_eq!(written.len(), golden.len(), "{path} writer: message length");
}
