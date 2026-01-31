use rag::RagClient;
use pyroduct::{module, ToRow};
// Importing from the external 'text_splitters' crate
use text_splitters::{ChunkConfig, TextSplitter};

#[derive(ToRow)]
struct RagResponse {
    query: String,
    results: Vec<SearchResultOutput>,
}

#[derive(ToRow)]
struct SearchResultOutput {
    id: String,
    content: String,
    score: f32,
    preview: String,
}

#[module(output = response)]
fn rag_search(query: &str) -> Result<RagResponse, String> {
    let rag = RagClient.register()?;
    let results = rag.search(query.to_string())?;

    let splitter = TextSplitter::new(ChunkConfig::new(100));

    let processed_results: Vec<SearchResultOutput> = results
        .into_iter()
        .map(|r| {
            let preview = splitter
                .chunks(&r.content)
                .next()
                .unwrap_or("")
                .to_string();

            SearchResultOutput {
                id: r.id,
                content: r.content,
                score: r.score,
                preview,
            }
        })
        .collect();

    Ok(RagResponse {
        query: query.to_string(),
        results: processed_results,
    })
}