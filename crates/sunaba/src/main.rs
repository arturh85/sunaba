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

    /// Capture a screenshot (level:N, ui:panel, or just N for backward compat)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    screenshot: Option<String>,

    /// Output path for screenshot (default: screenshots/<scenario_name>.png)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    screenshot_output: Option<String>,

    /// Screenshot width in pixels
    #[arg(long, default_value = "1920")]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    screenshot_width: usize,

    /// Screenshot height in pixels
    #[arg(long, default_value = "1080")]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    screenshot_height: usize,

    /// Number of frames to simulate before capturing (let physics settle)
    #[arg(long, default_value = "60")]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    screenshot_settle: usize,

    /// List available demo levels
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    list_levels: bool,

    /// List all available screenshot scenarios (levels + UI panels)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    list_scenarios: bool,

    /// Capture UI screenshot (requires full game initialization)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    screenshot_ui: bool,

    /// UI panel to show in screenshot (params, inventory, crafting, logger, worldgen, levels)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    ui_panel: Option<String>,

    /// List available UI panels
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    list_ui_panels: bool,

    /// Run test scenario from RON file (headless)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    test_scenario: Option<String>,

    /// Output directory for scenario results
    #[arg(long, default_value = "scenario_results")]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    scenario_output: String,

    /// Capture screenshots during scenario execution
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    scenario_screenshots: bool,

    /// Enable detailed profiling (flamegraph output)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    detailed_profiling: bool,

    /// Read test scenario from stdin (RON format)
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    test_scenario_stdin: bool,

    /// Enable TCP remote control server on port 7453
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    remote_control: bool,

    /// Generate video animation from scenario ID
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    video_scenario: Option<String>,

    /// List all available video scenarios
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    list_video_scenarios: bool,

    /// Generate all video scenarios
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    generate_all_videos: bool,

    /// Output directory for videos (default: "videos/")
    #[arg(long, default_value = "videos")]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    video_output_dir: String,

    /// Enable debug statistics output during video generation
    #[arg(long)]
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    debug_stats: bool,
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

    // Handle --list-levels flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.list_levels {
        sunaba::screenshot::list_levels();
        return Ok(());
    }

    // Handle --list-scenarios flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.list_scenarios {
        sunaba::screenshot::list_all_scenarios();
        return Ok(());
    }

    // Handle --list-ui-panels flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.list_ui_panels {
        sunaba::screenshot::list_ui_panels();
        return Ok(());
    }

    // Handle --screenshot flag (scenario-based)
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if let Some(scenario_str) = args.screenshot {
        if args.screenshot_ui || args.ui_panel.is_some() {
            eprintln!("Error: --screenshot-ui and --ui-panel require running without --screenshot");
            eprintln!("For UI screenshots, use: --screenshot ui:panel");
            std::process::exit(1);
        }

        // Parse scenario
        let scenario =
            sunaba::screenshot::ScreenshotScenario::parse(&scenario_str, args.screenshot_settle)?;

        // Determine output path
        let output_path = args.screenshot_output.unwrap_or_else(|| {
            std::fs::create_dir_all("screenshots").ok();
            format!("screenshots/{}.png", scenario.name())
        });

        return sunaba::screenshot::capture_scenario(
            scenario,
            output_path,
            args.screenshot_width,
            args.screenshot_height,
        );
    }

    // Handle --screenshot-ui flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.screenshot_ui || args.ui_panel.is_some() {
        // Determine which panel to screenshot (default: params)
        let panel_name = args.ui_panel.as_deref().unwrap_or("params");

        // Parse UI panel scenario
        let scenario_str = format!("ui:{}", panel_name);
        let scenario = sunaba::screenshot::ScreenshotScenario::parse(&scenario_str, 0)?;

        // Determine output path
        let output_path = args.screenshot_output.unwrap_or_else(|| {
            std::fs::create_dir_all("screenshots").ok();
            format!("screenshots/{}.png", scenario.name())
        });

        return sunaba::screenshot::capture_scenario(
            scenario,
            output_path,
            args.screenshot_width,
            args.screenshot_height,
        );
    }

    // Handle --list-video-scenarios flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.list_video_scenarios {
        sunaba::screenshot::list_video_scenarios();
        return Ok(());
    }

    // Handle --generate-all-videos flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.generate_all_videos {
        use std::path::PathBuf;

        // Create output directory
        std::fs::create_dir_all(&args.video_output_dir)?;

        let scenarios = sunaba::screenshot::get_all_video_scenarios();
        let total = scenarios.len();

        log::info!(
            "Generating {} video scenarios to: {}",
            total,
            args.video_output_dir
        );

        for (i, scenario) in scenarios.iter().enumerate() {
            log::info!("=== Video {}/{}: {} ===", i + 1, total, scenario.name);

            let output_path =
                PathBuf::from(&args.video_output_dir).join(format!("{}.mp4", scenario.id));

            match sunaba::screenshot::capture_video_scenario(
                scenario,
                &output_path,
                args.debug_stats,
            ) {
                Ok(_) => log::info!("✓ Successfully generated: {:?}", output_path),
                Err(e) => {
                    log::error!("✗ Failed to generate {}: {}", scenario.id, e);
                    // Continue with other scenarios
                }
            }

            println!(); // Blank line between scenarios
        }

        log::info!("Finished generating videos!");
        return Ok(());
    }

    // Handle --video-scenario flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if let Some(scenario_id) = args.video_scenario {
        use std::path::PathBuf;

        // Get scenario by ID
        let scenario = sunaba::screenshot::get_video_scenario_by_id(&scenario_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown video scenario: '{}'\nRun with --list-video-scenarios to see available scenarios",
                    scenario_id
                )
            })?;

        // Create output directory
        std::fs::create_dir_all(&args.video_output_dir)?;

        // Determine output path
        let output_path =
            PathBuf::from(&args.video_output_dir).join(format!("{}.mp4", scenario.id));

        log::info!("Generating video scenario: {}", scenario.name);

        return sunaba::screenshot::capture_video_scenario(
            &scenario,
            output_path,
            args.debug_stats,
        );
    }

    // Handle --test-scenario-stdin flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if args.test_scenario_stdin {
        use anyhow::Context;
        use std::io::Read;
        use sunaba::scenario::{ScenarioDefinition, ScenarioExecutor, ScenarioExecutorConfig};
        use sunaba_core::world::World;

        // Initialize detailed profiling if requested
        #[cfg(feature = "detailed_profiling")]
        let _guard = if args.detailed_profiling {
            let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file("profiling_trace.json")
                .build();

            use tracing_subscriber::prelude::*;
            let _ = tracing_subscriber::registry().with(chrome_layer).try_init();

            log::info!("Detailed profiling enabled - trace will be saved to profiling_trace.json");
            Some(guard)
        } else {
            None
        };

        log::info!("Reading scenario from stdin...");

        // Read scenario from stdin
        let mut stdin_content = String::new();
        std::io::stdin().read_to_string(&mut stdin_content)?;

        // Parse RON from stdin
        let scenario: ScenarioDefinition =
            ron::from_str(&stdin_content).context("Failed to parse RON scenario from stdin")?;

        log::info!("Loaded scenario: {}", scenario.name);

        // Create executor with config
        let config = ScenarioExecutorConfig {
            capture_screenshots: args.scenario_screenshots,
            screenshot_dir: "screenshots".to_string(),
            verbose: false,
            detailed_profiling: args.detailed_profiling,
        };
        let mut executor = ScenarioExecutor::with_config(config);

        // Create world
        let mut world = World::new(false);

        // Execute scenario
        let report = executor.execute_scenario(&scenario, &mut world)?;

        // Save JSON results
        std::fs::create_dir_all(&args.scenario_output)?;
        let sanitized_name = scenario
            .name
            .replace(' ', "_")
            .replace('/', "_")
            .replace('\\', "_");
        let output_file = format!("{}/{}_result.json", args.scenario_output, sanitized_name);
        report.save_json(&output_file)?;

        // Print results
        println!("═══════════════════════════════════════════════════");
        println!("Scenario: {}", scenario.name);
        println!("═══════════════════════════════════════════════════");
        println!(
            "Result: {}",
            if report.passed {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        );
        println!("Frames executed: {}", report.frames_executed);
        println!("Actions executed: {}", report.actions_executed);

        if !report.verification_failures.is_empty() {
            println!("\nVerification failures:");
            for failure in &report.verification_failures {
                println!("  ✗ {}", failure.message);
            }
        }

        if !report.screenshots.is_empty() {
            println!("\nScreenshots:");
            for screenshot in &report.screenshots {
                println!("  - {}", screenshot);
            }
        }

        println!("\nResults saved to: {}", output_file);
        println!("═══════════════════════════════════════════════════");

        // Explicitly drop guard to flush profiling data before exit
        #[cfg(feature = "detailed_profiling")]
        drop(_guard);

        std::process::exit(if report.passed { 0 } else { 1 });
    }

    // Handle --test-scenario flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if let Some(scenario_path) = args.test_scenario {
        use sunaba::scenario::{ScenarioDefinition, ScenarioExecutor, ScenarioExecutorConfig};
        use sunaba_core::world::World;

        // Initialize detailed profiling if requested
        #[cfg(feature = "detailed_profiling")]
        let _guard = if args.detailed_profiling {
            let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file("profiling_trace.json")
                .build();

            use tracing_subscriber::prelude::*;
            let _ = tracing_subscriber::registry().with(chrome_layer).try_init();

            log::info!("Detailed profiling enabled - trace will be saved to profiling_trace.json");
            Some(guard)
        } else {
            None
        };

        log::info!("Loading scenario from: {}", scenario_path);

        // Load scenario from file
        let scenario = ScenarioDefinition::from_file(&scenario_path)?;

        // Create executor with config
        let config = ScenarioExecutorConfig {
            capture_screenshots: args.scenario_screenshots,
            screenshot_dir: "screenshots".to_string(),
            verbose: false,
            detailed_profiling: args.detailed_profiling,
        };
        let mut executor = ScenarioExecutor::with_config(config);

        // Create world
        let mut world = World::new(false);

        // Execute scenario
        let report = executor.execute_scenario(&scenario, &mut world)?;

        // Save JSON results
        std::fs::create_dir_all(&args.scenario_output)?;
        let sanitized_name = scenario
            .name
            .replace(' ', "_")
            .replace('/', "_")
            .replace('\\', "_");
        let output_file = format!("{}/{}_result.json", args.scenario_output, sanitized_name);
        report.save_json(&output_file)?;

        // Print results
        println!("═══════════════════════════════════════════════════");
        println!("Scenario: {}", scenario.name);
        println!("═══════════════════════════════════════════════════");
        println!(
            "Result: {}",
            if report.passed {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        );
        println!("Frames executed: {}", report.frames_executed);
        println!("Actions executed: {}", report.actions_executed);

        if !report.verification_failures.is_empty() {
            println!("\nVerification failures:");
            for failure in &report.verification_failures {
                println!("  ✗ {}", failure.message);
            }
        }

        if !report.screenshots.is_empty() {
            println!("\nScreenshots:");
            for screenshot in &report.screenshots {
                println!("  - {}", screenshot);
            }
        }

        println!("\nResults saved to: {}", output_file);
        println!("═══════════════════════════════════════════════════");

        // Explicitly drop guard to flush profiling data before exit
        #[cfg(feature = "detailed_profiling")]
        drop(_guard);

        std::process::exit(if report.passed { 0 } else { 1 });
    }

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

    // Extract remote control flag
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    let remote_control = args.remote_control;

    #[cfg(not(all(not(target_arch = "wasm32"), feature = "headless")))]
    let remote_control = false;

    pollster::block_on(run(server_url, remote_control))
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
async fn run(server_url: Option<String>, enable_remote_control: bool) -> anyhow::Result<()> {
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

    // Start TCP remote control server if requested
    #[cfg(all(not(target_arch = "wasm32"), feature = "headless"))]
    if enable_remote_control {
        let (cmd_rx, resp_tx) = sunaba::remote_control::start_server()?;
        app.enable_remote_control(cmd_rx, resp_tx);
    }

    App::run(event_loop, app)
}
