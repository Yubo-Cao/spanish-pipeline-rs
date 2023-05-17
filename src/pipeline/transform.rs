use std::io::{Read, Write};

use async_trait::async_trait;
use clap::{Parser, ValueEnum};

use super::{Flashcard, Pipeline, PipelineError, PipelineIO};

/// Represents the different file types that can be loaded
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum TransformOutputType {
    Yaml,
    Pdf,
    Json,
}

#[derive(Parser)]
pub struct TransformPipeline {
    /// The name of the output file.
    #[clap(short, long)]
    name: Option<String>,

    /// The type of the output file.
    #[clap(short, long, default_value = "pdf")]
    output_type: TransformOutputType,

    /// The row of flashcard in a page.
    #[clap(short, long, default_value = "6")]
    row: usize,

    /// The column of flashcard in a page.
    #[clap(short, long, default_value = "3")]
    column: usize,

    /// The fontsize of the flashcard, specified in Typst length
    #[clap(short, long, default_value = "14pt")]
    fontsize: String,
}

const TYPST_FLASHCARD_TEMPLATE: &str = include_str!("../templates/flashcard.typ");

impl TransformPipeline {
    fn run_pdf(&self, flashcard: Vec<Flashcard>) -> Result<PipelineIO, Box<dyn std::error::Error>> {
        let mut content = TYPST_FLASHCARD_TEMPLATE
            .replace("<ROW>", self.row.to_string().as_str())
            .replace("<COLUMN>", self.column.to_string().as_str())
            .replace("<FONT_SIZE>", self.fontsize.as_str());

        content.push_str(
            flashcard
                .chunks(self.row * self.column)
                .map(|cards| {
                    [
                        "#card_layout(".to_string(),
                        cards
                            .iter()
                            .map(|card| format!("front[{}]", card.word))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        ")".to_string(),
                        "#pagebreak()".to_string(),
                        "#card_layout(".to_string(),
                        cards
                            .iter()
                            .map(|card| format!("back[{}]", card.definition))
                            .collect::<Vec<_>>()
                            .join(",\n"),
                        ")".to_string(),
                    ]
                    .join("\n")
                })
                .collect::<Vec<_>>()
                .join("\n")
                .as_str(),
        );

        let temp_dir = tempfile::tempdir()?;
        let flashcard_file_path = temp_dir.path().join("flashcard.typ");
        let mut flashcard_file = std::fs::File::create(&flashcard_file_path)?;
        flashcard_file.write_all(content.as_bytes())?;

        let output = std::process::Command::new("typst")
            .arg("compile")
            .arg(flashcard_file_path)
            .output()?;

        if !output.status.success() {
            return Err(Box::new(PipelineError::new("typst failed to compile")));
        }

        let mut buf = Vec::new();
        let mut pdf_file = std::fs::File::open(temp_dir.path().join("flashcard.pdf"))?;
        pdf_file.read_to_end(&mut buf)?;

        let name = self.name.clone().unwrap_or("flashcard.pdf".to_string());

        Ok(PipelineIO::Document { name, content: buf })
    }
}

#[async_trait]
impl Pipeline for TransformPipeline {
    async fn run(
        &self,
        input: Option<PipelineIO>,
    ) -> Result<PipelineIO, Box<dyn std::error::Error>> {
        let flashcards = match input {
            Some(PipelineIO::Flashcard(flashcard)) => flashcard,
            _ => return Err(Box::new(PipelineError::new("input is not a flashcard"))),
        };
        match self.output_type {
            TransformOutputType::Yaml => {
                let name = self.name.clone().unwrap_or("flashcard.yml".to_string());
                Ok(PipelineIO::Document {
                    name,
                    content: serde_yaml::to_string(&flashcards)?.into_bytes(),
                })
            }
            TransformOutputType::Json => {
                let name = self.name.clone().unwrap_or("flashcard.json".to_string());
                Ok(PipelineIO::Document {
                    name,
                    content: serde_json::to_vec(&flashcards)?,
                })
            }
            TransformOutputType::Pdf => self.run_pdf(flashcards),
        }
    }

    fn name(&self) -> &'static str {
        "transform"
    }
}
