use pyroduct::module;

struct NoToRow {  // ERROR: does not implement ToRow
    value: i32,
}

#[module(output = NoToRow)]
fn call(input: &str) -> Result<NoToRow, String> {
    println!("{input}");
    Ok(NoToRow { value: 42 })
}

fn main() {}