//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod error;
pub mod pipeline;
pub mod spider;

use clap::Parser;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;
use pipeline::Pipeline;

const PIPELINES: [&str; 3] = ["load", "visual_vocab", "transform"];

#[derive(Parser)]
struct Cli {
    /// The name of the group of output files.
    #[clap(short, long, default_value = "default")]
    name: String,

    /// The log level.
    #[clap(short, long, default_value = "info")]
    level: log::LevelFilter,

    /// Quiet mode.
    #[clap(short, long)]
    quiet: bool,

    #[clap(skip)]
    pipelines: Vec<Box<dyn Pipeline>>,
}

impl std::fmt::Debug for Cli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cli")
            .field("name", &self.name)
            .field("level", &self.level)
            .field("quiet", &self.quiet)
            .field(
                "pipelines",
                &self.pipelines.iter().map(|p| p.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Parses the command line arguments and returns the corresponding pipelines.
#[allow(unused_assignments)]
fn parse_arguments() -> Cli {
    let args = std::env::args().collect::<Vec<String>>();
    let mut pipelines = Vec::new();
    let mut i = 1;

    // parse the cli arguments
    let start = i;
    while i < args.len() && !PIPELINES.contains(&args[i].as_str()) {
        i += 1;
    }
    let mut cli =
        Cli::parse_from([&["".to_string()], &args[start..i]].concat());

    // parse the pipelines
    while i < args.len() {
        let pipeline = &args[i];
        if !PIPELINES.contains(&pipeline.as_str()) {
            Cli::parse_from(&["", "--help"]);
            unreachable!("should have printed help");
        }
        let start = i;
        i += 1;
        while i < args.len() && !PIPELINES.contains(&args[i].as_str()) {
            i += 1;
        }
        let args = &args[start..i];

        let result: Box<dyn Pipeline> = match pipeline.as_str() {
            "load" => Box::new(pipeline::load::LoadPipeline::parse_from(args)),
            "visual_vocab" => Box::new(
                pipeline::visual_vocab::VisualVocabPipeline::parse_from(args),
            ),
            "transform" => Box::new(
                pipeline::transform::TransformPipeline::parse_from(args),
            ),
            _ => unreachable!(),
        };

        pipelines.push(result);
    }
    if pipelines.is_empty() {
        Cli::parse_from(&["", "--help"]);
        unreachable!("should have printed help");
    }
    cli.pipelines = pipelines;
    cli
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse the cli arguments
    let Cli {
        name,
        level,
        pipelines,
        quiet,
    } = parse_arguments();
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Magenta);
    let mut dispatch =
        fern::Dispatch::new().format(move |out, message, record| {
            // if terminal
            out.finish(format_args!(
                "[{}] [{}] {}",
                record.target(),
                colors.color(record.level()),
                message
            ))
        });
    dispatch = match quiet {
        true => dispatch.level(log::LevelFilter::Off),
        false => dispatch
            .level(level)
            .chain(
                fern::Dispatch::new()
                    .level(log::LevelFilter::Warn)
                    .chain(std::io::stderr()),
            )
            .chain(
                fern::Dispatch::new()
                    .level(log::LevelFilter::Info)
                    .chain(std::io::stdout()),
            ),
    };
    dispatch.apply()?;

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
