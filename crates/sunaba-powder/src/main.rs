//! Sunaba Powder - Powder Game-style demo

use sunaba_powder::App;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Sunaba Powder");

    pollster::block_on(run())
}

async fn run() -> anyhow::Result<()> {
    let (app, event_loop) = App::new().await?;
    App::run(event_loop, app)
}
