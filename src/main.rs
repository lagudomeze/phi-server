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
    /// Name of the person to greet
    #[structopt(short, long)]
    debug: bool,

    /// Name of the person to greet
    #[arg(short, long, default_value = ".")]
    config_dir: String,

    #[arg(short, long, default_value = "prod")]
    profile: String,
}

fn main() -> common::Result<()> {
    let args = Args::try_parse()?;

    run!(dir = args.config_dir.as_str(), profile = args.profile.as_str());

    let router = Router::new()
        .push(auth::router());

    WebServer::run(router)?;

    Ok(())
}
