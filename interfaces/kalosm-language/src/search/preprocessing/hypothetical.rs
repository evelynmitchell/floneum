use kalosm_language_model::{Embedder, Model, StructureParserResult, SyncModel, VectorSpace};
use kalosm_sample::{LiteralParser, ParserExt, StopOn};
use kalosm_streams::text_stream::ChannelTextStream;

use crate::{
    prelude::{Document, IndexParser, Task},
    search::Chunk,
};

use super::{ChunkStrategy, Chunker};

const TASK_DESCRIPTION: &str =
    "You generate hypothetical questions that may be answered by the given text. The questions restate any information nessisary to understand the question";

const EXAMPLES:[(&str, &str);2] = [
    (
"Yesterday, apple stock rose 10%, google stock rose 15%, and microsoft stock rose 5%",
"Questions that are answered by the previous text: How have tech stocks changed recently? How have apple stocks changed recently? How have google stocks changed recently? How have microsoft stocks changed recently?"
    ),
    (
        "The Eiffel Tower is a wrought-iron lattice tower on the Champ de Mars in Paris, France. It is named after the engineer Gustave Eiffel, whose company designed and built the tower.",
        "Questions that are answered by the previous text: What is the Eiffel Tower? Who designed the Eiffel Tower?"
    ),
];

const QUESTION_STARTERS: [&str; 9] = [
    "Who", "What", "When", "Where", "Why", "How", "Which", "Whom", "Whose",
];

fn create_constraints() -> kalosm_sample::SequenceParser<
    LiteralParser<&'static str>,
    kalosm_sample::RepeatParser<
        kalosm_sample::SequenceParser<
            IndexParser<
                LiteralParser<&'static str>,
                kalosm_sample::LiteralMismatchError,
                (),
                kalosm_sample::LiteralParserOffset,
            >,
            StopOn<&'static str>,
        >,
    >,
> {
    LiteralParser::new("Questions that are answered by the previous text: ").then(
        IndexParser::new(
            QUESTION_STARTERS
                .iter()
                .copied()
                .map(LiteralParser::new)
                .collect::<Vec<_>>(),
        )
        .then(StopOn::new("?").filter_characters(
            |c| matches!(c, ' ' | '?' | 'a'..='z' | 'A'..='Z' | '0'..='9' | ','),
        ))
        .repeat(1..=5),
    )
}

/// A builder for a hypothetical chunker.
pub struct HypotheticalBuilder<'a, M>
where
    M: Model,
    <M::SyncModel as SyncModel>::Session: Send,
{
    model: &'a mut M,
    task_description: Option<String>,
    examples: Option<Vec<(String, String)>>,
    chunking: Option<ChunkStrategy>,
}

impl<'a, M> HypotheticalBuilder<'a, M>
where
    M: Model,
    <M::SyncModel as SyncModel>::Session: Send,
{
    /// Set the chunking strategy.
    pub fn with_chunking(mut self, chunking: ChunkStrategy) -> Self {
        self.chunking = Some(chunking);
        self
    }

    /// Set the examples for this task. Each example should include the text and the questions that are answered by the text.
    pub fn with_examples<S: Into<String>>(
        mut self,
        examples: impl IntoIterator<Item = (S, S)>,
    ) -> HypotheticalBuilder<'a, M> {
        self.examples = Some(
            examples
                .into_iter()
                .map(|(a, b)| (a.into(), b.into()))
                .collect::<Vec<_>>(),
        );
        self
    }

    /// Set the task description. The task description should describe a task of generating hypothetical questions that may be answered by the given text.
    pub fn with_task_description(mut self, task_description: String) -> Self {
        self.task_description = Some(task_description);
        self
    }

    /// Build the hypothetical chunker.
    pub fn build(self) -> anyhow::Result<Hypothetical> {
        let task_description = self
            .task_description
            .unwrap_or_else(|| TASK_DESCRIPTION.to_string());
        let examples = self.examples.unwrap_or_else(|| {
            EXAMPLES
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>()
        });
        let chunking = self.chunking;

        let task = Task::builder(self.model, task_description)
            .with_constraints(create_constraints)
            .with_examples(examples)
            .build();

        Ok(Hypothetical { chunking, task })
    }
}

/// Generates embeddings of questions
pub struct Hypothetical {
    chunking: Option<ChunkStrategy>,
    task: Task<StructureParserResult<ChannelTextStream<String>, ((), Vec<((usize, ()), String)>)>>,
}

impl Hypothetical {
    /// Create a new hypothetical chunker.
    pub fn builder<M>(model: &mut M) -> HypotheticalBuilder<M>
    where
        M: Model,
        <M::SyncModel as SyncModel>::Session: Send,
    {
        HypotheticalBuilder {
            model,
            task_description: None,
            examples: None,
            chunking: None,
        }
    }

    /// Generate a list of hypothetical questions about the given text.
    pub async fn generate_question(&self, text: &str) -> anyhow::Result<Vec<String>> {
        let questions = self.task.run(text).await?.result().await?;
        let documents = questions
            .1
            .into_iter()
            .map(|((i, _), s)| QUESTION_STARTERS[i].to_string() + &s)
            .collect::<Vec<_>>();

        println!("documents: {:?}", documents);

        Ok(documents)
    }
}

#[async_trait::async_trait]
impl<S: VectorSpace + Send + Sync + 'static> Chunker<S> for Hypothetical {
    async fn chunk<E: Embedder<S> + Send>(
        &self,
        document: &Document,
        embedder: &mut E,
    ) -> anyhow::Result<Vec<Chunk<S>>> {
        let body = document.body();

        #[allow(clippy::single_range_in_vec_init)]
        let byte_chunks = self
            .chunking
            .map(|chunking| chunking.chunk_str(body))
            .unwrap_or_else(|| vec![0..body.len()]);

        println!("body: {:?}", body);
        println!("byte_chunks: {:?}", byte_chunks);

        if byte_chunks.is_empty() {
            return Ok(vec![]);
        }

        let mut questions = Vec::new();
        let mut questions_count = Vec::new();
        for byte_chunk in &byte_chunks {
            let text = &body[byte_chunk.clone()];
            let mut chunk_questions = self.generate_question(text).await?;
            questions.append(&mut chunk_questions);
            questions_count.push(chunk_questions.len());
        }
        let embeddings = embedder
            .embed_batch(&questions.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .await?;

        let mut chunks = Vec::with_capacity(embeddings.len());
        let mut questions_count = questions_count.iter();
        let mut byte_chunks = byte_chunks.into_iter();

        let mut remaining_embeddings = *questions_count.next().unwrap();
        let mut byte_chunk = byte_chunks.next().unwrap();

        for embedding in embeddings {
            while remaining_embeddings == 0 {
                if let Some(&questions_count) = questions_count.next() {
                    remaining_embeddings = questions_count;
                    byte_chunk = byte_chunks.next().unwrap();
                }
            }
            remaining_embeddings -= 1;
            chunks.push(Chunk {
                byte_range: byte_chunk.clone(),
                embeddings: vec![embedding],
            });
        }

        Ok(chunks)
    }
}
