use std::env;

use anyhow::Result;

mod args;
mod help;

#[cfg(test)]
mod args_tests;

use self::{
    args::{parse_args, Command},
    help::help_text,
};
use crate::integrations::{self, Paths};

pub fn run() -> Result<()> {
    let cli = parse_args(env::args().skip(1))?;

    if matches!(cli.command, Command::Help) {
        print_help();
        return Ok(());
    }

    match cli.command {
        Command::Init { path } => {
            let workspace = integrations::init_new_workspace(path, cli.init_template.as_deref())?;
            println!("Initialized {}.", workspace.root.display());
            println!("Template: {}", workspace.active_template);
            return Ok(());
        }
        Command::Templates => {
            let paths = cli
                .workspace
                .clone()
                .and_then(|workspace| Paths::discover(Some(workspace)).ok());
            for line in integrations::list_templates(paths.as_ref()) {
                println!("{line}");
            }
            return Ok(());
        }
        _ => {}
    }

    let paths = Paths::discover(cli.workspace)?;

    match cli.command {
        Command::Check => {
            print!("{}", integrations::check(&paths)?);
            Ok(())
        }
        Command::Summary => {
            print!("{}", integrations::summary(&paths)?);
            Ok(())
        }
        Command::Render => {
            print!("{}", integrations::render_html(&paths)?);
            Ok(())
        }
        Command::Pdf => {
            print!("{}", integrations::render_pdf(&paths)?);
            Ok(())
        }
        Command::Web => {
            integrations::run_web_server(&paths, &cli.web_host, cli.web_port, cli.web_open)
        }
        Command::UseTemplate { template } => {
            let workspace = integrations::use_template(&paths, &template)?;
            println!("Active template: {}", workspace.active_template);
            Ok(())
        }
        Command::ExportTemplate { path, output } => {
            println!("{}", integrations::export_template_bundle(&path, &output)?);
            Ok(())
        }
        Command::Doctor => {
            print!("{}", integrations::doctor(&paths)?);
            Ok(())
        }
        Command::Init { .. } | Command::Templates | Command::Help => unreachable!(),
    }
}

fn print_help() {
    println!("{}", help_text());
}
