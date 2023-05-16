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
    Document { name: String, content: Vec<u8> },
    Clipboard(String),
    Flashcard(Vec<Flashcard>),
}

/// Represents the input of a pipeline stage.
#[derive(Debug)]
pub enum PipelineInput {
    None,
    KV(HashMap<String, String>),
}

/// Gets the value of a key from a pipeline input, or returns a default value.
pub fn get_or_default<T>(map: &PipelineInput, key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    match map {
        PipelineInput::None => default,
        PipelineInput::KV(map) => match map.get(key) {
            Some(value) => match value.parse::<T>() {
                Ok(value) => value,
                Err(_) => default,
            },
            None => default,
        },
    }
}

/// Represents a pipeline stage.
#[async_trait]
pub trait PipelineStage {
    async fn process(&self, input: PipelineInput) -> Result<Vec<PipelineOutput>, &'static str>;
}
