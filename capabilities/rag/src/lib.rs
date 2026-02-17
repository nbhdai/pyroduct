// Simple RAG capability with linear search

/// Configuration for the RAG system, defining the initial knowledge base.
#[pyroduct::config]
pub struct RagConfig {
    /// The collection of documents to be indexed and searched.
    pub documents: Vec<RagDocument>,
    /// The default number of results to return for a search query.
    pub top_k: usize,
}

/// A single unit of text information within the RAG knowledge base.
#[pyroduct::config]
pub struct RagDocument {
    /// Unique identifier for the document.
    pub id: String,
    /// The raw text content to be embedded and retrieved.
    pub content: String,
}

/// The Pyroduct interface item for the RAG Client.
#[pyroduct::magma]
pub struct RagClient;

/// Internal representation of a document after it has been processed into a vector.
struct EmbeddedDocument {
    id: String,
    content: String,
    /// The high-dimensional vector representation of the content.
    embedding: Vec<f32>,
}

pub struct RagServer {
    documents: Vec<EmbeddedDocument>,
    top_k: usize,
}

#[pyroduct::capability]
/// A stateful server handling document embeddings and similarity searches.
impl RagServer {
    type Client = RagClient;
    type Config = RagConfig;
    type Error = String;
    
    /// Initializes the RAG server by embedding all documents provided in the config.
    /// Note: This performs a linear pass and generates embeddings for every document.
    async fn new(config: Option<RagConfig>) -> Self {
        let config = config.unwrap_or(RagConfig {
            documents: Vec::new(),
            top_k: 5,
        });
        
        let mut documents = Vec::with_capacity(config.documents.len());
        
        for doc in config.documents {
            let embedding = embed_text(&doc.content).await;
            documents.push(EmbeddedDocument {
                id: doc.id,
                content: doc.content,
                embedding,
            });
        }
        
        Self {
            documents,
            top_k: config.top_k,
        }
    }
    
    /// Resets the internal state.
    async fn reset(&mut self) {}
    
    /// Validates and prepares a new RAG client.
    fn register(&self, _client: &RagClient) -> Result<(), String> {
        Ok(())
    }
    
    /// Searches the document store using cosine similarity and returns the top_k results.
    /// The query is embedded in real-time before comparison.
    async fn search(&self, _client: &RagClient, query: String) -> Result<Vec<SearchResult>, String> {
        let query_embedding = embed_text(&query).await;
        
        let mut scored: Vec<(f32, &EmbeddedDocument)> = self.documents
            .iter()
            .map(|doc| (cosine_similarity(&query_embedding, &doc.embedding), doc))
            .collect();
        
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        let results = scored
            .into_iter()
            .take(self.top_k)
            .map(|(score, doc)| SearchResult {
                id: doc.id.clone(),
                content: doc.content.clone(),
                score,
            })
            .collect();
        
        Ok(results)
    }
    
    /// Searches the document store and allows the caller to override the default `k` value.
    async fn search_with_k(&self, _client: &RagClient, query: String, k: usize) -> Result<Vec<SearchResult>, String> {
        let query_embedding = embed_text(&query).await;
        
        let mut scored: Vec<(f32, &EmbeddedDocument)> = self.documents
            .iter()
            .map(|doc| (cosine_similarity(&query_embedding, &doc.embedding), doc))
            .collect();
        
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        let results = scored
            .into_iter()
            .take(k)
            .map(|(score, doc)| SearchResult {
                id: doc.id.clone(),
                content: doc.content.clone(),
                score,
            })
            .collect();
        
        Ok(results)
    }
}

/// A search result containing the matched document and its relevance score.
#[pyroduct::magma]
pub struct SearchResult {
    /// The identifier of the matched document.
    pub id: String,
    /// The text content of the match.
    pub content: String,
    /// The similarity score (higher is more relevant).
    pub score: f32,
}

async fn embed_text(text: &str) -> Vec<f32> {
    // Placeholder logic...
    let mut embedding = vec![0.0f32; 384];
    for (i, byte) in text.bytes().enumerate() {
        embedding[i % 384] += (byte as f32) / 255.0;
    }
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }
    embedding
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}