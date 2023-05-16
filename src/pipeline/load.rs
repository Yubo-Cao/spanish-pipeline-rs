use super::{Flashcard, Pipeline, PipelineIO};
use async_trait::async_trait;
use serde_json::from_reader;
use serde_yaml::from_str;
use std::fs::File;
use std::io::Read;

use super::{Pipeline, PipelineIO};
use async_trait::async_trait;
use docx_rs::{Docx, Table, TableCell, TableRow};

/// Represents the different file types that can be loaded
enum VocabFile {
    Yaml,
    Json,
    Docx,
}

/// Load pipeline that loads a vocabulary file into Vec<Flashcard>
struct LoadPipeline {
    filetype: VocabFile,
}

#[async_trait]
impl Pipeline for LoadPipeline {
    async fn run(&self, input: Option<PipelineIO>) -> Result<PipelineIO, &'static str> {
        let path = match input {
            Some(PipelineIO::FilePath(path)) => path,
            _ => return Err("Invalid input"),
        };
        let mut file = File::open(&path).map_err(|_| "Failed to open file")?;
        let flashcard = match self.filetype {
            VocabFile::Yaml => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|_| "Failed to read file")?;
                from_str(&contents).map_err(|_| "Failed to parse YAML")
            }
            VocabFile::Json => from_reader(&mut file).map_err(|_| "Failed to parse JSON"),
            VocabFile::Docx => {
                let docx = Docx::from_file(&path).map_err(|_| "Failed to open Docx file")?;
                let mut flashcard = Vec::new();
                for table in docx.tables() {
                    if table.rows().len() > 0 && table.rows()[0].cells().len() == 2 {
                        let mut prev_word = String::new();
                        for row in table.rows() {
                            let cells = row.cells();
                            let word = cells[0].text().trim().replace("  ", " ");
                            let definition = cells[1].text().trim().replace("  ", " ");
                            if !word.is_empty()
                                && !definition.is_empty()
                                && word.to_lowercase() != prev_word.to_lowercase()
                            {
                                prev_word = word.clone();
                                let word = word
                                    .replace("->", "→")
                                    .replace("“", "\"")
                                    .replace("”", "\"")
                                    .replace("¨", "");
                                let definition = definition
                                    .replace("->", "→")
                                    .replace("“", "\"")
                                    .replace("”", "\"")
                                    .replace("¨", "");
                                flashcard.push(Flashcard { word, definition });
                            }
                        }
                    }
                }
                Ok(PipelineIO::Flashcard(flashcard))
            }
        };
        Ok(PipelineIO::Flashcard(flashcard?))
    }

    fn get_command() -> clap::Command {
        clap::App::new("load_vocab")
            .arg(
                Arg::new("vocab_path")
                    .short('p')
                    .long("vocab_path")
                    .value_name("FILE")
                    .help("Sets the path to the vocabulary file")
                    .value_parser(value_parser!(PathBuf)),
            )
            .to_owned()
    }

    fn new(m: &clap::ArgMatches) -> Self {
        let filetype = match m.value_of("filetype") {
            Some("yaml") => VocabFile::Yaml,
            Some("json") => VocabFile::Json,
            Some("docx") => VocabFile::Docx,
            _ => panic!("Invalid filetype"),
        };
        LoadPipeline { filetype }
    }
}
