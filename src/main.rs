use clap::Parser;
use ioc::{Bean, export, LogPatcher, run};

mod auth;
mod material;
mod common;
mod db;
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

export!();

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

    Ok(())
}
