use clap::Parser;
use std::path::PathBuf;
use sunaba::App;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Delete existing world and generate fresh
    #[arg(long)]
    regenerate: bool,

    /// Run headless evolution training
    #[arg(long)]
    train: bool,

    /// Training scenario: locomotion, foraging, survival, balanced, parcour
    #[arg(long, default_value = "locomotion")]
    scenario: String,

    /// Number of generations to train
    #[arg(long, default_value = "100")]
    generations: usize,

    /// Population size per generation
    #[arg(long, default_value = "50")]
    population: usize,

    /// Output directory for training reports
    #[arg(long, default_value = "training_output")]
    output: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Parse command-line arguments
    let args = Args::parse();

    // Validate flag combinations
    if args.train && args.regenerate {
        eprintln!("Error: --train and --regenerate are mutually exclusive");
        std::process::exit(1);
    }

    // Handle training mode
    if args.train {
        #[cfg(feature = "headless")]
        {
            return run_training(&args);
        }
        #[cfg(not(feature = "headless"))]
        {
            eprintln!("Error: --train requires 'headless' feature");
            eprintln!("Run: cargo run --features headless -- --train");
            std::process::exit(1);
        }
    }

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

#[cfg(feature = "headless")]
fn run_training(args: &Args) -> anyhow::Result<()> {
    use sunaba::headless::{Scenario, TrainingConfig, TrainingEnv};

    log::info!("Starting headless evolution training");
    log::info!("  Scenario: {}", args.scenario);
    log::info!("  Generations: {}", args.generations);
    log::info!("  Population: {}", args.population);
    log::info!("  Output: {}", args.output);

    let scenario = match args.scenario.as_str() {
        "foraging" => Scenario::foraging(),
        "survival" => Scenario::survival(),
        "balanced" => Scenario::balanced(),
        "locomotion" => Scenario::locomotion(),
        "parcour" => Scenario::parcour(),
        other => {
            log::warn!("Unknown scenario '{}', defaulting to locomotion", other);
            Scenario::locomotion()
        }
    };

    let config = TrainingConfig {
        generations: args.generations,
        population_size: args.population,
        output_dir: args.output.clone(),
        ..TrainingConfig::default()
    };

    let mut env = TrainingEnv::new(config, scenario);
    env.run()
}

async fn run() -> anyhow::Result<()> {
    let (app, event_loop) = App::new().await?;
    App::run(event_loop, app)
}
