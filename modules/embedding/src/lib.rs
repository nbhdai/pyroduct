use pyroduct::*;

#[magma]
struct Embedding {
    message: String,
    embedding: Vec<f32>,
}

#[module(output = messages)]
fn process(input: Vec<EmbeddingRef<'_>>) -> Result<Vec<String>> {
    Ok(input.iter().map(|e| {
       let embedding: &[f32] = e.embedding;
        format!("Embedding of {} has {}", e.message, embedding.len())
    }).collect())
}