use clap::Parser;
use std::path::PathBuf;
use sunaba::App;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Delete existing world and generate fresh
    #[arg(long)]
    regenerate: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Parse command-line arguments
    let args = Args::parse();

    // Handle --regenerate flag
    if args.regenerate {
        log::info!("--regenerate flag detected, deleting existing world");
        let world_dir = PathBuf::from("worlds/default");
        if world_dir.exists() {
            std::fs::remove_dir_all(&world_dir)?;
            log::info!("Deleted world directory: {:?}", world_dir);
        }
    }

    log::info!("Starting Sunaba");
    pollster::block_on(run())
}

async fn run() -> anyhow::Result<()> {
    let (app, event_loop) = App::new().await?;
    App::run(event_loop, app)
}
