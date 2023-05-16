pub mod flashcard;
pub mod parse;
pub mod visual_vocab;

use std::path::Path;

use async_trait::async_trait;
use clipboard::{ClipboardContext, ClipboardProvider};
use flashcard::Flashcard;

/// Represents the output of a pipeline stage.
#[derive(Debug)]
pub enum PipelineIO {
    Document { name: String, content: Vec<u8> },
    Clipboard(String),
    Flashcard(Vec<Flashcard>),
}

impl PipelineIO {
    /// Dump the output to the specified path.
    pub fn dump(&self, vocab_path: &Path) -> Result<(), &'static str> {
        let out_dir = format!(
            "./out/{}",
            vocab_path.file_stem().unwrap().to_str().unwrap()
        );
        std::fs::create_dir_all(&out_dir).map_err(|_| "Failed to create output directory")?;

        match self {
            PipelineIO::Document { name, content } => {
                let path = format!("{}/{}", out_dir, name);
                std::fs::write(path, content).map_err(|_| "Failed to write document to file")?;
            }
            PipelineIO::Clipboard(info) => {
                let mut clipboard: ClipboardContext = clipboard::ClipboardProvider::new().unwrap();
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
                let serialized = serde_yaml::to_string(flashcards)
                    .map_err(|_| "Failed to serialize flashcards")?;
                std::fs::write(path, serialized)
                    .map_err(|_| "Failed to write flashcards to file")?;
            }
        }
        Ok(())
    }
}

/// Represents a pipeline for processing the input.
#[async_trait]
pub trait Pipeline {
    /// Processes the input and returns the output.
    async fn run(&self, input: Option<PipelineIO>) -> Result<PipelineIO, &'static str>;

    /// Provide the subcommand to be registered
    fn get_command() -> clap::Command;

    /// Process the match from the subcommand
    fn new(m: &clap::ArgMatches) -> Self;
}
