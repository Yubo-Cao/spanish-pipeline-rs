//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod pipeline;
pub mod spider;

use std::path::PathBuf;

use clap::{value_parser, Arg, ArgMatches, Command};
use log::info;
use pipeline::flashcard::Flashcard;
use pipeline::PipelineStage;
use simple_logger::SimpleLogger;

/// Parses the command line arguments.
fn parse_arguments() -> ArgMatches {
    Command::new("spanish_pipeline")
        .about("Pipeline to finish Spanish homework")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .author("Yubo-Cao <cao20006721@gmail.com>")
        .arg(
            Arg::new("vocab_path")
                .short('p')
                .long("vocab_path")
                .value_name("FILE")
                .help("Sets the path to the vocabulary file")
                .value_parser(value_parser!(PathBuf)),
        )
        .subcommand(pipeline::visual_vocab::VisualVocabPipeline::get_command())
        .get_matches()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_boxed_logger(Box::new(SimpleLogger::new()))
        .map(|()| log::set_max_level(log::LevelFilter::Info))?;
    info!(target: "main", "starting");
    let matches = parse_arguments();

    let vocab_path: &PathBuf = matches
        .get_one("vocab_path")
        .expect("vocab_path is required");
    let vocabs = match Flashcard::load(vocab_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to load vocab: {}", e);
            Err("Failed to load vocab")?
        }
    };
    info!(target: "main", "loaded {} vocabs", vocabs.len());

    let _result = match matches.subcommand() {
        Some(("visual_vocab", matches)) => {
            let pipeline = pipeline::visual_vocab::VisualVocabPipeline::new(matches);
            pipeline.run(vocabs).await?
        }
        _ => Err("No subcommand provided")?,
    };
    info!(target: "main", "pipline finished");

    Ok(())
}
