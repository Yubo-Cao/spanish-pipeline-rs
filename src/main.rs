//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod error;
pub mod pipeline;
pub mod spider;

use clap::Parser;
use error::CliError;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use pipeline::Pipeline;

const PIPELINES: [&str; 2] = ["load", "visual_vocab"];

struct Cli {
    name: String,
    level: log::LevelFilter,
    pipelines: Vec<Box<dyn Pipeline>>,
}

/// Parses the command line arguments and returns the corresponding pipelines.
#[allow(unused_assignments)]
fn parse_arguments() -> Result<Cli, Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let mut pipelines = Vec::new();
    let mut i = 1;
    let mut name = "default";
    let mut level = log::LevelFilter::Info;
    while i < args.len() {
        let pipeline = &args[i];
        if !PIPELINES.contains(&pipeline.as_str()) {
            if pipeline.starts_with('-') {
                match pipeline.as_str() {
                    "-n" | "--name" => {
                        i += 1;
                        name = &args[i];
                    }
                    "-l" | "--level" => {
                        i += 1;
                        level = match args[i].as_str() {
                            "debug" => log::LevelFilter::Debug,
                            "info" => log::LevelFilter::Info,
                            "warn" => log::LevelFilter::Warn,
                            "error" => log::LevelFilter::Error,
                            _ => {
                                return Err(CliError::new("Invalid log level").into());
                            }
                        };
                    }
                    _ => {
                        return Err(CliError::new("Invalid option").into());
                    }
                }
                i += 1;
                continue;
            } else {
                return Err(CliError::new(&format!("Invalid pipeline: {}", pipeline)).into());
            }
        }
        let start = i;
        i += 1;
        while i < args.len() && !PIPELINES.contains(&args[i].as_str()) {
            i += 1;
        }
        let result: Box<dyn Pipeline> = match pipeline.as_str() {
            "load" => Box::new(pipeline::load::LoadPipeline::try_parse_from(
                &args[start..i],
            )?),
            "visual_vocab" => Box::new(
                pipeline::visual_vocab::VisualVocabPipeline::try_parse_from(&args[start..i])?,
            ),
            _ => unreachable!(),
        };

        pipelines.push(result);
    }
    if pipelines.is_empty() {
        return Err(CliError::new("No pipeline specified").into());
    }

    Ok(Cli {
        level,
        name: name.to_string(),
        pipelines,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        name,
        level,
        pipelines,
    } = match parse_arguments() {
        Ok(cli) => cli,
        Err(err) => {
            println!("{}", err);
            return Ok(());
        }
    };

    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Magenta);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(level)
        .level_for("cached_path", log::LevelFilter::Error)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
    info!(target: "main", "logger initialized");

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
