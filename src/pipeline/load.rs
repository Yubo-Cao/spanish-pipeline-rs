use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use async_trait::async_trait;
use clap::{arg, Parser, ValueEnum};
use docx_rs::{read_docx, TableChild, TableRowChild};
use log::{info, warn};
use serde_json::from_reader;
use serde_yaml::from_str;

use super::{Flashcard, Pipeline, PipelineIO};

/// Represents the different file types that can be loaded
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum VocabFileType {
    Yaml,
    Json,
    Docx,
}

/// Represents the input of a pipeline stage.
#[derive(Parser)]
pub struct LoadPipeline {
    /// The path to the file to load
    #[arg(value_parser = |x: &str| {
        let path = PathBuf::from(x);
        if path.exists() {
            Ok(Box::new(path))
        } else {
            Err("File does not exist")
        }
    })]
    path: Box<PathBuf>,

    /// The type of file to load
    #[arg(short = 't', long = "type")]
    filetype: Option<VocabFileType>,
}

#[async_trait]
impl Pipeline for LoadPipeline {
    async fn run(
        &self,
        input: Option<PipelineIO>,
    ) -> Result<PipelineIO, Box<dyn std::error::Error>> {
        info!(target: "load_pipeline", "Pipeline starting");

        if input.is_some() {
            Err("LoadPipeline does not accept input")?
        }

        let mut file = File::open(&self.path as &PathBuf)?;

        let extension = self
            .path
            .extension()
            .ok_or("Failed to get file extension")?
            .to_str()
            .ok_or("Failed to convert file extension to string")?;
        let mut filetype = self.filetype;
        if filetype.is_none() {
            filetype = match extension {
                "yml" | "yaml" => Some(VocabFileType::Yaml),
                "json" => Some(VocabFileType::Json),
                "docx" => Some(VocabFileType::Docx),
                _ => Err("Failed to determine file type")?,
            };
        }
        match filetype {
            None => Err("Failed to determine file type")?,
            Some(filetype) => {
                let flashcard = match filetype {
                    VocabFileType::Yaml => {
                        info!(target: "load_pipeline", "Loading YAML file: {}", self.path.display());
                        let mut contents = String::new();
                        file.read_to_string(&mut contents)?;
                        from_str::<Vec<Flashcard>>(&contents)?
                    }
                    VocabFileType::Json => {
                        info!(target: "load_pipeline", "Loading JSON file: {}", self.path.display());
                        from_reader(&mut file)?
                    }
                    VocabFileType::Docx => {
                        info!(target: "load_pipeline", "Loading DOCX file: {}", self.path.display());
                        let mut buf = Vec::new();
                        File::open(&self.path as &PathBuf)?.read_to_end(&mut buf)?;
                        let docx = read_docx(&buf)?;

                        let mut flashcard = Vec::new();
                        for table in docx.document.children.iter().filter_map(|x| {
                            if let docx_rs::DocumentChild::Table(x) = x {
                                Some(x)
                            } else {
                                None
                            }
                        }) {
                            let rows = &table.rows;
                            if rows.is_empty() {
                                warn!(target: "load_pipeline", "Skipping empty table");
                                continue;
                            }

                            for row in rows.iter() {
                                let TableChild::TableRow(row) = row;
                                if row.cells.len() != 2 {
                                    warn!(target: "load_pipeline", "Skipping row {:?} with {} columns", textify_row(row), row.cells.len());
                                    continue;
                                }

                                let cells = &row.cells;
                                let word = textify_cell(&cells[0]);
                                let definition = textify_cell(&cells[1]);

                                if !word.is_empty()
                                    && !definition.is_empty()
                                    && word.to_lowercase() != definition.to_lowercase()
                                {
                                    let word = word
                                        .replace("->", "→")
                                        .replace(['“', '”'], "\"")
                                        .replace('¨', "");
                                    let definition = definition
                                        .replace("->", "→")
                                        .replace(['“', '”'], "\"")
                                        .replace('¨', "");
                                    flashcard.push(Flashcard { word, definition });
                                }
                            }
                        }
                        flashcard
                    }
                };
                Ok(PipelineIO::Flashcard(flashcard))
            }
        }
    }
}

fn textify_row(row: &docx_rs::TableRow) -> String {
    "| ".to_string()
        + row
            .cells
            .iter()
            .map(|x| textify_cell(x))
            .collect::<Vec<_>>()
            .join(" | ")
            .trim()
        + " |"
}

fn textify_cell(cell: &docx_rs::TableRowChild) -> String {
    let TableRowChild::TableCell(cell) = cell;
    cell.children
        .iter()
        .map(|x| {
            if let docx_rs::TableCellContent::Paragraph(paragraph) = x {
                textify_paragraph(paragraph)
            } else {
                String::new()
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn textify_paragraph(paragraph: &docx_rs::Paragraph) -> String {
    paragraph
        .children
        .iter()
        .map(|x| {
            if let docx_rs::ParagraphChild::Run(run) = x {
                textify_run(run)
            } else {
                String::new()
            }
        })
        .collect::<String>()
}

fn textify_run(run: &Box<docx_rs::Run>) -> String {
    run.children
        .iter()
        .map(|x| {
            if let docx_rs::RunChild::Text(text) = x {
                text.text.clone()
            } else {
                String::new()
            }
        })
        .collect::<String>()
}
