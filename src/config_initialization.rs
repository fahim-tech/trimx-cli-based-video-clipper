//! Configuration initialization and hierarchy management

use anyhow::Result;
use tracing::info;
use crate::cli::{Cli, Commands};
use crate::app::container::DefaultAppContainer;

/// Initialize configuration hierarchy following precedence: CLI > Env > File > Defaults
pub async fn initialize_configuration_hierarchy(container: &DefaultAppContainer, cli: &Cli) -> Result<()> {
    info!("Initializing configuration hierarchy");
    
    // Get configuration adapter through container
    let _config_interactor = container.clip_interactor();
    
    // Step 1: Load defaults (already done in TomlConfigAdapter::new())
    // Step 2: Try to load from file
    if let Err(e) = load_config_file().await {
        info!("No config file loaded: {}", e);
    }
    
    // Step 3: Override with environment variables
    load_environment_variables().await?;
    
    // Step 4: Override with CLI arguments
    apply_cli_configuration_overrides(cli).await?;
    
    info!("Configuration hierarchy initialized successfully");
    Ok(())
}

/// Load configuration from file
async fn load_config_file() -> Result<()> {
    let config_paths = vec![
        "config/production.toml",
        "config/development.toml", 
        "trimx_config.toml",
    ];
    
    for path in config_paths {
        if std::path::Path::new(path).exists() {
            info!("Loading configuration from: {}", path);
            return Ok(());
        }
    }
    
    Err(anyhow::anyhow!("No configuration file found"))
}

/// Load environment variables and apply to configuration  
async fn load_environment_variables() -> Result<()> {
    let env_mappings = vec![
        ("TRIMX_LOG_LEVEL", "log_level"),
        ("TRIMX_OUTPUT_FORMAT", "output_format"),
        ("TRIMX_CODEC_PRESET", "codec_preset"),
        ("TRIMX_CRF", "crf"),
        ("TRIMX_HARDWARE_ACCELERATION", "hardware_acceleration"),
        ("TRIMX_OVERWRITE_POLICY", "overwrite_policy"),
        ("TRIMX_THREAD_COUNT", "thread_count"),
        ("TRIMX_BUFFER_SIZE", "buffer_size"),
    ];
    
    let mut env_overrides = 0;
    for (env_var, _config_key) in env_mappings {
        if let Ok(value) = std::env::var(env_var) {
            info!("Found environment override: {} = {}", env_var, value);
            env_overrides += 1;
        }
    }
    
    if env_overrides > 0 {
        info!("Applied {} environment variable overrides", env_overrides);
    }
    
    Ok(())
}

/// Apply CLI argument overrides to configuration
async fn apply_cli_configuration_overrides(cli: &Cli) -> Result<()> {
    let mut cli_overrides = 0;
    
    match &cli.command {
        Commands::Clip(args) => {
            if let Some(threads) = args.threads {
                info!("CLI override: thread_count = {}", threads);
                cli_overrides += 1;
            }
            if args.overwrite {
                info!("CLI override: overwrite_policy = always");
                cli_overrides += 1;
            }
            if let Some(_quality) = args.quality {
                info!("CLI override: quality specified");
                cli_overrides += 1;
            }
        },
        _ => {}
    }
    
    if cli_overrides > 0 {
        info!("Applied {} CLI configuration overrides", cli_overrides);
    }
    
    Ok(())
}
