//! Test #[derive(Layout, Writer, Reader)] with byte blob fields: `Bytes`,
//! `Box<[u8]>`, and a list of them, written and read back.

use pyroformat::{Reader, Realize, Layout, Writer};

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithBytes32 {
    id: u32,
    payload: bytes::Bytes,
    boxed: Box<[u8]>,
    chunks: Vec<bytes::Bytes>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct WithBytes16 {
    id: u32,
    payload: bytes::Bytes,
    boxed: Box<[u8]>,
    chunks: Vec<bytes::Bytes>,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {
    {
        assert_pyro_traits::<WithBytes32>();

        let row = WithBytes32 {
            id: 7,
            payload: bytes::Bytes::from_static(b"hello"),
            boxed: vec![1u8, 2, 3].into_boxed_slice(),
            chunks: vec![
                bytes::Bytes::from_static(b"ab"),
                bytes::Bytes::new(),
                bytes::Bytes::from_static(b"c"),
            ],
        };

        let buffer = row.encode().unwrap();

        let read = WithBytes32::pyro_read(&buffer).unwrap();
        assert_eq!(read.payload, b"hello");
        assert_eq!(read.realize(), row);
    }
    {
        assert_pyro_traits::<WithBytes16>();

        let row = WithBytes16 {
            id: 7,
            payload: bytes::Bytes::from_static(b"hello"),
            boxed: vec![1u8, 2, 3].into_boxed_slice(),
            chunks: vec![
                bytes::Bytes::from_static(b"ab"),
                bytes::Bytes::new(),
                bytes::Bytes::from_static(b"c"),
            ],
        };

        let buffer = row.encode().unwrap();

        let read = WithBytes16::pyro_read(&buffer).unwrap();
        assert_eq!(read.payload, b"hello");
        assert_eq!(read.realize(), row);
    }
}
