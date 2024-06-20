use ioc::run;
use salvo::Router;
use clap::Parser;

use crate::web::WebServer;

mod auth;
mod material;
mod common;
mod db;
mod web;

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

fn main() -> common::Result<()> {
    let args = Args::parse();

    println!("{args:?}!" );

    if args.debug {
        unsafe { std::env::set_var("RUST_LOG", "debug"); }
    }

    run!(
        dir = args.config_dir.as_str(),
        profile = args.profile.as_str()
    );

    let router = Router::new()
        .push(auth::router());

    WebServer::run(router)?;

    Ok(())
}
