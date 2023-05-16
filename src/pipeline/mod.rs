use std::collections::HashMap;

use async_trait::async_trait;
pub mod visual_vocab;

/// Represents the flashcard output of a pipeline stage.
#[derive(Debug, Clone)]
pub struct Flashcard {
    pub word: String,
    pub definition: String,
}

/// Represents the output of a pipeline stage.
#[derive(Debug)]
pub enum PipelineOutput {
    PDF(Vec<u8>),
    Clipboard(String),
    Flashcard(Vec<Flashcard>),
}

/// Represents the input of a pipeline stage.
#[derive(Debug)]
pub enum PipelineInput {
    None,
    KV(HashMap<String, String>),
}

/// Represents a pipeline stage.
#[async_trait]
pub trait PipelineStage {
    async fn process(&self, input: PipelineInput) -> Result<Vec<PipelineOutput>, &'static str>;
}
