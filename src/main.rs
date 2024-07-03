use clap::Parser;
use ioc::{Bean, LogPatcher, run};
use simply_poem::load_types;

use crate::web::WebServer;

mod auth;
mod material;
mod common;
mod db;
mod web;
mod log;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,

    /// Directory for configuration files. Configuration file should be named as {app-name}-profile.toml
    #[arg(short, long, default_value = ".")]
    config_dir: String,

    /// Profile to use
    #[arg(short, long, default_value = "prod")]
    profile: String,
}

load_types!();

fn main() -> common::Result<()> {
    let args = Args::parse();

    println!("{args:?}!" );

    let _ = run!(
        dir = args.config_dir.as_str(),
        profile = args.profile.as_str()
    );

    if args.debug {
        LogPatcher::try_get()?
            .reload(["trace"])?;
    }


    WebServer::run()?;

    Ok(())
}
