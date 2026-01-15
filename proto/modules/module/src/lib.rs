use proto_reporter::report;

#[unsafe(no_mangle)]
pub extern "C" fn exter_call(input_ptr: *mut u8, input_len: usize) -> u64 {
    #[derive(::pyroduct::FromRow, ::pyroduct::DeepRef)]
    struct __Input {
        input: String,
    }

    #[derive(::pyroduct::ToRow)]
    struct __Output {
        output: String,
    }

    let call = |input: &__InputRef| call(&input.input).map(|result| __Output { output: result });

    ::pyroduct::module::call::<__Input, __Output, _>(input_ptr, input_len, call)
}

/// This should be represented in the macro by
/// pub fn call(input: &str) -> Result<{ output: String }, String> { ... }
pub fn call(input: &str) -> Result<String, String> {
    tracing::info!("Calling!");
    let mut result = String::from("Processed: ");
    result.push_str(input);

    let words: Vec<&str> = result.split_whitespace().collect();
    report("Reporting to host".to_string());
    report("Reporting to host, again".to_string());
    result.push_str(&format!(" [words: {}]", words.len()));
    Ok(result)
}
