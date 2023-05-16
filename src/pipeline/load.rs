use super::{Flashcard, Pipeline, PipelineIO};
use async_trait::async_trait;
use serde_json::from_reader;
use serde_yaml::from_str;
use std::fs::File;
use std::io::Read;

use super::{Pipeline, PipelineIO};
use async_trait::async_trait;

/// Represents the different file types that can be loaded
enum VocabFile {
    Yaml,
    Json,
    Docx,
}

/// Represents the input of a pipeline stage.
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
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|_| "Failed to read file")?;
        let flashcard = match self.filetype {
            VocabFile::Yaml => from_str(&contents).map_err(|_| "Failed to parse YAML"),
            VocabFile::Json => from_reader(&mut file).map_err(|_| "Failed to parse JSON"),
            VocabFile::Docx => todo!("Implement loading for docx file type"),
        }?;
        Ok(PipelineIO::Flashcard(flashcard))
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
