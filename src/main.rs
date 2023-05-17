//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod error;
pub mod pipeline;
pub mod spider;

use clap::Parser;
use error::CliError;
use log::info;
use pipeline::Pipeline;
use simple_logger::SimpleLogger;

const PIPELINES: [&str; 2] = ["load", "visual_vocab"];

struct Cli {
    name: String,
    pipelines: Vec<Box<dyn Pipeline>>,
}

/// Parses the command line arguments and returns the corresponding pipelines.
#[allow(unused_assignments)]
fn parse_arguments() -> Result<Cli, Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let mut pipelines = Vec::new();
    let mut i = 1;
    let mut name = "default";
    while i < args.len() {
        let pipeline = &args[i];
        if !PIPELINES.contains(&pipeline.as_str()) {
            if pipeline.starts_with('-') {
                match pipeline.as_str() {
                    "-n" | "--name" => {
                        i += 1;
                        name = &args[i];
                    }
                    _ => {
                        return Err(CliError::new("Invalid option").into());
                    }
                }
            }
            return Err(CliError::new("Invalid pipeline").into());
        }
        i += 1;
        while i < args.len() && !PIPELINES.contains(&args[i].as_str()) {
            i += 1;
        }
        let result: Box<dyn Pipeline> = match pipeline.as_str() {
            "load" => Box::new(pipeline::load::LoadPipeline::try_parse_from(&args[1..i])?),
            "visual_vocab" => Box::new(
                pipeline::visual_vocab::VisualVocabPipeline::try_parse_from(&args[1..i])?,
            ),
            _ => unreachable!(),
        };
        pipelines.push(result);
    }
    if pipelines.is_empty() {
        return Err(CliError::new("No pipeline specified").into());
    }
    Ok(Cli {
        name: name.to_string(),
        pipelines,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_boxed_logger(Box::new(SimpleLogger::new()))
        .map(|()| log::set_max_level(log::LevelFilter::Info))?;
    info!(target: "main", "starting");

    let Cli { name, pipelines } = match parse_arguments() {
        Ok(cli) => cli,
        Err(err) => {
            println!("{}", err);
            return Ok(());
        }
    };

    let mut input = None;
    for pipeline in pipelines {
        input = Some(pipeline.run(input).await?);
    }
    info!(target: "main", "finished");

    if let Some(output) = input {
        output.dump(&name)?;
        info!(target: "main", "dumped output");
    }
    Ok(())
}
