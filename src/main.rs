//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod error;
pub mod pipeline;
pub mod spider;

use clap::{ArgAction, Parser};
use error::CliError;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use pipeline::Pipeline;

const PIPELINES: [&str; 3] = ["load", "visual_vocab", "transform"];

#[derive(Parser)]
struct Cli {
    /// The name of the group of output files.
    #[clap(default_value = "default")]
    name: String,

    /// The log level.
    level: Option<log::LevelFilter>,

    /// Quiet mode.
    #[clap(short, long, action = ArgAction::SetTrue)]
    quiet: Option<bool>,

    #[clap(skip)]
    pipelines: Vec<Box<dyn Pipeline>>,
}

/// Parses the command line arguments and returns the corresponding pipelines.
#[allow(unused_assignments)]
fn parse_arguments() -> Result<Cli, Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let mut pipelines = Vec::new();
    let mut i = 1;

    // parse the cli arguments
    let start = i;
    while i < args.len() && !PIPELINES.contains(&args[i].as_str()) {
        i += 1;
    }
    let mut cli = Cli::try_parse_from(&args[start..i])?;

    // parse the pipelines
    while i < args.len() {
        let pipeline = &args[i];
        if !PIPELINES.contains(&pipeline.as_str()) {
            return Err(CliError::new(&format!("Invalid pipeline: {}", pipeline)).into());
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
            "transform" => Box::new(pipeline::transform::TransformPipeline::try_parse_from(
                &args[start..i],
            )?),
            _ => unreachable!(),
        };

        pipelines.push(result);
    }
    if pipelines.is_empty() {
        return Err(CliError::new("No pipeline specified").into());
    }
    cli.pipelines = pipelines;

    Ok(cli)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse the cli arguments
    let Cli {
        name,
        level,
        pipelines,
        quiet,
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
    let mut dispatch = fern::Dispatch::new().format(move |out, message, record| {
        // if terminal
        out.finish(format_args!(
            "[{}] [{}] {}",
            record.target(),
            colors.color(record.level()),
            message
        ))
    });
    if let Some(level) = level {
        dispatch = dispatch.level(level);
    }
    if let Some(_) = quiet {
        dispatch = dispatch.level(log::LevelFilter::Off);
    }
    dispatch
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    info!(target: "main", "logger initialized");

    // run the pipelines
    let mut input = None;
    for pipeline in pipelines {
        info!(target: "main", "running pipeline: {}", pipeline.name());
        input = Some(pipeline.run(input).await?);
        info!(target: "main", "finished pipeline: {}", pipeline.name());
    }
    info!(target: "main", "finished");

    // dump the output
    if let Some(output) = input {
        output.dump(&name)?;
        info!(target: "main", "dumped output");
    }
    Ok(())
}
