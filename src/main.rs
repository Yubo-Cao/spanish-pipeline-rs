pub mod google_image;

use google_image::image_search;
use log::info;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_boxed_logger(Box::new(SimpleLogger::new()))
        .map(|()| log::set_max_level(log::LevelFilter::Info))?;
    info!(target: "main", "starting");

    let images = image_search("cat", 100).await?;
    for image in images {
        println!("{}", image);
    }
    Ok(())
}
