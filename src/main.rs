//! This project aims to create a simple web crawler to faciliate the completion of Spanish homework.

pub mod pipeline;
pub mod spider;

use log::info;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_boxed_logger(Box::new(SimpleLogger::new()))
        .map(|()| log::set_max_level(log::LevelFilter::Info))?;
    info!(target: "main", "starting");

    Ok(())
}
