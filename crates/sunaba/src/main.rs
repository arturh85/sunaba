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

    /// Training scenario: locomotion, foraging, survival, balanced, parcour, simple
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

    /// Use simple morphology (fewer body parts, viability filter)
    #[arg(long)]
    simple: bool,

    /// Creature archetype: all (default), evolved, spider, snake, worm, flyer
    #[arg(long, default_value = "all")]
    archetype: String,

    /// Server URL to connect to on startup (multiplayer mode)
    #[arg(long)]
    #[cfg(feature = "multiplayer")]
    server: Option<String>,
}

fn main() -> anyhow::Result<()> {
    // Initialize egui_logger for in-game log viewing (also provides env_logger functionality)
    egui_logger::builder()
        .init()
        .expect("Failed to initialize logger");

    // Initialize puffin profiler (native only, when feature enabled)
    #[cfg(feature = "profiling")]
    {
        puffin::set_scopes_on(true);
        log::info!("Puffin profiler initialized - press F3 in-game to view");
    }

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

    // Extract server URL for multiplayer if provided
    #[cfg(feature = "multiplayer")]
    let server_url = args.server;

    #[cfg(not(feature = "multiplayer"))]
    let server_url: Option<String> = None;

    pollster::block_on(run(server_url))
}

#[cfg(feature = "headless")]
fn run_training(args: &Args) -> anyhow::Result<()> {
    use sunaba::creature::morphology::CreatureArchetype;
    use sunaba::headless::{Scenario, TrainingConfig, TrainingEnv};

    // Parse archetype(s)
    let archetypes: Vec<CreatureArchetype> = if args.archetype.to_lowercase() == "all" {
        CreatureArchetype::all_with_evolved().to_vec()
    } else {
        let arch: CreatureArchetype = args.archetype.parse().unwrap_or_else(|e| {
            log::warn!("{}, using Evolved", e);
            CreatureArchetype::Evolved
        });
        vec![arch]
    };

    log::info!("Starting headless evolution training");
    log::info!("  Scenario: {}", args.scenario);
    log::info!(
        "  Archetypes: {:?}",
        archetypes.iter().map(|a| a.name()).collect::<Vec<_>>()
    );
    log::info!("  Generations: {}", args.generations);
    log::info!("  Population: {}", args.population);
    log::info!("  Output: {}", args.output);
    log::info!("  Simple morphology: {}", args.simple);

    // If --simple flag or "simple" scenario, use simple locomotion
    let (scenario, use_simple) = if args.simple || args.scenario == "simple" {
        log::info!("Using simple morphology with viability filter");
        (Scenario::simple_locomotion(), true)
    } else {
        let s = match args.scenario.as_str() {
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
        (s, false)
    };

    let config = TrainingConfig {
        generations: args.generations,
        population_size: args.population,
        output_dir: args.output.clone(),
        use_simple_morphology: use_simple,
        archetypes: archetypes.clone(),
        archetype: archetypes.first().copied().unwrap_or_default(),
        ..TrainingConfig::default()
    };

    let mut env = TrainingEnv::new(config, scenario);
    env.run()
}

#[cfg_attr(not(feature = "multiplayer"), allow(unused_variables, unused_mut))]
async fn run(server_url: Option<String>) -> anyhow::Result<()> {
    let (mut app, event_loop) = App::new().await?;

    // If server URL provided, connect to multiplayer server before starting game loop
    #[cfg(feature = "multiplayer")]
    if let Some(url) = server_url {
        log::info!("Connecting to server: {}", url);
        if let Err(e) = app.connect_to_server(url).await {
            log::error!("Failed to connect to server: {}", e);
            log::info!("Continuing in singleplayer mode");
        }
    }

    App::run(event_loop, app)
}
