use pyroduct::format::Document;

#[derive(Document)]
struct DataContainer {
    items: Vec<u8>,
    labels: Vec<String>,
}

fn main() {}
