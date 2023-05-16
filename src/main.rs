//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod pipeline;
pub mod spider;

use clap::{ArgMatches, Command};
use log::info;
use pipeline::Pipeline;
use simple_logger::SimpleLogger;

/// Parses the command line arguments.
fn parse_arguments() -> ArgMatches {
    Command::new("spanish_pipeline")
        .about("Pipeline to finish Spanish homework")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .author("Yubo-Cao <cao20006721@gmail.com>")
        .subcommand(pipeline::visual_vocab::VisualVocabPipeline::get_command())
        .get_matches()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_boxed_logger(Box::new(SimpleLogger::new()))
        .map(|()| log::set_max_level(log::LevelFilter::Info))?;
    info!(target: "main", "starting");
    let _matches = parse_arguments();
    info!(target: "main", "pipline finished");

    Ok(())
}
