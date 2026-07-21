mod app_paths;
mod archive;
mod cli;
mod config;
mod document_model;
mod document_package;
mod integrations;
mod json_file;
mod model;
mod pdf;
mod remote;
mod render;
mod storage;
mod templates;
#[cfg(test)]
mod test_support;
mod web;
mod workspace;

fn main() -> anyhow::Result<()> {
    cli::run()
}
