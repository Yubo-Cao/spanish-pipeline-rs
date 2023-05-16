use async_trait::async_trait;

use super::{Pipeline, PipelineIO};

struct ParsePipeline {}

#[async_trait]
impl Pipeline for ParsePipeline {
    async fn run(&self, _input: Option<PipelineIO>) -> Result<PipelineIO, &'static str> {
        todo!()
    }

    fn get_command() -> clap::Command {
        todo!()
    }

    fn new(_m: &clap::ArgMatches) -> Self {
        todo!()
    }
}
