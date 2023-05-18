mod docx;
pub mod flashcard;
pub mod load;
pub mod transform;
pub mod visual_vocab;

use async_trait::async_trait;
use clipboard::{ClipboardContext, ClipboardProvider};
pub use flashcard::Flashcard;

/// Represents the output of a pipeline stage.
#[derive(Debug)]
pub enum PipelineIO {
    Document { name: String, content: Vec<u8> },
    Clipboard(String),
    Flashcard(Vec<Flashcard>),
}

impl PipelineIO {
    /// Dump the output to the specified path.
    pub fn dump(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let out_dir = format!("./out/{}", name);
        std::fs::create_dir_all(&out_dir)?;

        match self {
            PipelineIO::Document { name, content } => {
                let path = format!("{}/{}", out_dir, name);
                std::fs::write(path, content)?;
            }
            PipelineIO::Clipboard(info) => {
                let mut clipboard: ClipboardContext =
                    clipboard::ClipboardProvider::new().unwrap();
                clipboard.set_contents(info.to_owned()).unwrap();
                let clipboard_info = if info.len() > 20 {
                    format!("{}...", &info[..20])
                } else {
                    info.to_owned()
                };
                println!("Clipboard copied: {}", clipboard_info);
            }
            PipelineIO::Flashcard(flashcards) => {
                let path = format!("{}/flashcard.yml", out_dir);
                let serialized = serde_yaml::to_string(flashcards)?;
                std::fs::write(path, serialized)?;
            }
        }
        Ok(())
    }
}

/// Represents a pipeline for processing the input.
#[async_trait]
pub trait Pipeline {
    /// Processes the input and returns the output.
    async fn run(
        &self,
        input: Option<PipelineIO>,
    ) -> Result<PipelineIO, Box<dyn std::error::Error>>;

    /// Return the name of the pipeline.
    fn name(&self) -> &'static str;
}

/// Represents a Pipeline Error

#[derive(Debug)]
pub struct PipelineError {
    message: String,
}

impl std::error::Error for PipelineError {}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl PipelineError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}
