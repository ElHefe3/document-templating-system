mod app_paths;
mod builtin_classic_template;
mod builtin_country_templates;
mod builtin_templates;
mod cli;
mod cli_args;
#[cfg(test)]
mod cli_args_tests;
mod cli_help;
mod config;
mod country_resume_defaults;
mod country_resume_sections;
mod country_resume_spec;
mod document_model;
mod integrations;
mod json_file;
mod model;
mod pdf_command;
#[cfg(test)]
mod pdf_command_tests;
mod pdf_renderer;
mod remote_template_actions;
mod remote_template_keys;
mod remote_templates;
#[cfg(test)]
mod remote_templates_tests;
mod render;
mod s3_list_xml;
mod s3_request;
mod s3_signing;
mod s3_target;
mod storage;
#[cfg(test)]
mod storage_memory;
mod storage_s3;
#[cfg(test)]
mod storage_s3_tests;
mod template_bundle;
mod template_bundle_discovery;
mod template_bundle_manifest;
mod template_bundle_manifest_discovery;
mod template_bundle_renderer;
mod template_bundle_renderer_discovery;
mod template_catalog;
mod template_fields;
mod template_files;
mod template_manifest_validation;
mod template_package;
mod template_service;
#[cfg(test)]
mod test_support;
mod web_api;
mod web_asset_support;
mod web_assets;
mod web_http;
mod web_launch;
mod web_server;
mod web_state;
mod workspace;
mod zip_archive;
mod zip_archive_reader;
#[cfg(test)]
mod zip_archive_tests;
mod zip_archive_writer;

fn main() -> anyhow::Result<()> {
    cli::run()
}
