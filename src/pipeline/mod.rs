use std::collections::HashMap;

/// Represents the flashcard output of a pipeline stage.
#[derive(Debug)]
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
trait PipelineStage {
    fn process(&self, input: PipelineInput) -> Result<Vec<PipelineOutput>, &'static str>;
}
