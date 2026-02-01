// Simple RAG capability with linear search
#[pyroduct::config]
pub struct RagConfig {
    pub documents: Vec<RagDocument>,
    pub top_k: usize,
}

#[pyroduct::config]
pub struct RagDocument {
    pub id: String,
    pub content: String,
}

#[pyroduct::interface_item]
pub struct RagClient;

struct EmbeddedDocument {
    id: String,
    content: String,
    embedding: Vec<f32>,
}

pub struct RagServer {
    documents: Vec<EmbeddedDocument>,
    top_k: usize,
}

#[pyroduct::capability]
impl RagServer {
    type Client = RagClient;
    type Config = RagConfig;
    type Error = String;
    
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
    
    async fn reset(&mut self) {}
    
    fn new_client(&self, _client: &RagClient) -> Result<(), String> {
        Ok(())
    }
    
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


#[pyroduct::interface_item]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
}

async fn embed_text(text: &str) -> Vec<f32> {
    // Placeholder: In real implementation, call an embedding API
    // For now, simple hash-based fake embedding
    let mut embedding = vec![0.0f32; 384];
    for (i, byte) in text.bytes().enumerate() {
        embedding[i % 384] += (byte as f32) / 255.0;
    }
    // Normalize
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