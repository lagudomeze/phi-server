use clap::Parser;
use ioc::{export, run};

mod auth;
mod common;
mod db;
mod ffmpeg;
mod log;
mod material;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enable debug mode
    #[arg(short, long, default_value = "false")]
    debug: bool,

    /// Directory for configuration files. Configuration file should be named as {app-name}-profile.toml
    #[arg(short, long, default_value = ".")]
    config_dir: String,

    /// Profile to use
    #[arg(short, long, default_value = "prod")]
    profile: String,
}

export!(root = "src/main.rs");

fn main() -> common::Result<()> {
    let args = Args::parse();

    println!("{args:?}!");

    let _ = run!(
        debug = args.debug;
        dir = args.config_dir.as_str();
        profile = args.profile.as_str();
        crates(ioc);
    );

    Ok(())
}
