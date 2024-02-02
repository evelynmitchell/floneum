// Retrieval Strategies:
// 1. Simple keyword search
// 2. Vector search
//  1. Search by sentence, return window around match
//  2. Search by summary, return document
//  3. Search through document tree
//  4. Search by questions that may be answered by the document
//  5. Classify documents, search by class
//
// Context extraction strategies:
// 1. Dump all sentences
// 2. Dump all sentences that mention an entity
// 3. Extract relevant sentences with an llm

use kalosm_language_model::{Embedder, VectorSpace};

use crate::context::Document;

use super::Chunk;

mod chunking;
pub use chunking::*;
mod hypothetical;
pub use hypothetical::*;
mod summary;
pub use summary::*;

/// A strategy for chunking a document into smaller pieces.
#[async_trait::async_trait]
pub trait Chunker<S: VectorSpace + Send + Sync + 'static> {
    /// Chunk a document into embedded snippets.
    async fn chunk<E: Embedder<S> + Send>(
        &mut self,
        document: &Document,
        embedder: &mut E,
    ) -> anyhow::Result<Vec<Chunk<S>>>;

    /// Chunk a batch of documents into embedded snippets.
    async fn chunk_batch<'a, I, E: Embedder<S> + Send>(
        &mut self,
        documents: I,
        embedder: &mut E,
    ) -> anyhow::Result<Vec<Vec<Chunk<S>>>>
    where
        I: IntoIterator<Item = &'a Document> + Send,
        I::IntoIter: Send,
    {
        let mut chunks = Vec::new();
        for document in documents {
            chunks.push(self.chunk(document, embedder).await?);
        }
        Ok(chunks)
    }
}
