// SPDX-License-Identifier: AGPL-3.0-or-later
//! `BingoCube` `UniBin` — serve, demo, generate, and verify.

use std::path::PathBuf;

use bingocube_core::BingoCube;
use bingocube_ipc::{
    ConfigParam, ConfigPreset, CryptoVerifyParams, SeedParam, ServeConfig, SubCubeWire,
    TransportEndpoint, VERSION, commitment_hash, resolve_socket_dir, serve,
};
use clap::{CommandFactory, Parser, Subcommand};
use serde_json::json;

/// `BingoCube` — human-verifiable cryptographic commitment system.
#[derive(Parser)]
#[command(name = "bingocube", version = VERSION, about = "BingoCube primal — crypto commitments and evolutionary reservoir")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start IPC server (G66 transport-abstract).
    Serve {
        /// Transport endpoint as JSON. Overrides --socket-dir.
        /// Example: '{"transport":"tcp","host":"127.0.0.1","port":7700}'
        #[arg(long, env = "TRANSPORT_ENDPOINT")]
        transport: Option<String>,

        /// Directory for `.sock` files (UDS mode, default on Unix).
        #[arg(long, default_value = "/run/bingocube")]
        socket_dir: PathBuf,

        /// Disable the tarpc binary socket (C2 dual-socket).
        #[arg(long)]
        no_tarpc: bool,

        /// G65 single-socket protocol negotiation (tarpc vs JSON-RPC on one socket).
        #[arg(long, env = "BINGOCUBE_NEGOTIATE")]
        negotiate: bool,
    },
    /// Launch the interactive egui demo.
    Demo,
    /// Generate a `BingoCube` commitment from a seed.
    Generate {
        /// Seed string.
        #[arg(long)]
        seed: String,

        /// Configuration preset.
        #[arg(long, default_value = "medium")]
        config: String,
    },
    /// Verify a subcube against a seed at reveal level x.
    Verify {
        /// Seed string.
        #[arg(long)]
        seed: String,

        /// Reveal level x ∈ (0, 1].
        #[arg(long)]
        reveal: f64,

        /// Subcube JSON (see `crypto.reveal` output).
        #[arg(long)]
        subcube: String,

        /// Configuration preset.
        #[arg(long, default_value = "medium")]
        config: String,
    },
}

fn parse_config(name: &str) -> Result<ConfigParam, String> {
    match name.to_ascii_lowercase().as_str() {
        "small" | "default" => Ok(ConfigParam::Preset(ConfigPreset::Small)),
        "medium" => Ok(ConfigParam::Preset(ConfigPreset::Medium)),
        "large" => Ok(ConfigParam::Preset(ConfigPreset::Large)),
        other => Err(format!(
            "unknown config preset: {other} (use small, medium, or large)"
        )),
    }
}

fn resolve_endpoint(
    transport: Option<String>,
    socket_dir: PathBuf,
) -> Result<TransportEndpoint, Box<dyn std::error::Error>> {
    if let Some(json) = transport {
        Ok(serde_json::from_str(&json)?)
    } else {
        Ok(TransportEndpoint::platform_default(
            "bingocube",
            &resolve_socket_dir(Some(socket_dir)),
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        None => {
            Cli::command().print_help()?;
            println!();
        }
        Some(Commands::Serve {
            transport,
            socket_dir,
            no_tarpc,
            negotiate,
        }) => {
            let endpoint = resolve_endpoint(transport, socket_dir)?;
            tracing::info!(endpoint = %endpoint, "resolved transport endpoint");
            if negotiate {
                tracing::info!("G65 single-socket protocol negotiation enabled");
            }
            let config = ServeConfig {
                endpoint,
                enable_tarpc: !no_tarpc,
                negotiate,
            };
            let endpoints = serve(config).await?;
            tracing::info!(
                primary = %endpoints.primary,
                tarpc = ?endpoints.tarpc.as_ref().map(std::string::ToString::to_string),
                "servers stopped"
            );
        }
        Some(Commands::Demo) => {
            bingocube_demos::run_demo()?;
        }
        Some(Commands::Generate { seed, config }) => {
            let config_param = parse_config(&config)?;
            let config = config_param.into_config()?;
            let cube = BingoCube::from_seed(seed.as_bytes(), config)?;
            let hash = commitment_hash(&cube);
            let output = json!({
                "commitment_hash": hash,
                "grid_size": cube.config.grid_size,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Some(Commands::Verify {
            seed,
            reveal,
            subcube,
            config,
        }) => {
            let config_param = parse_config(&config)?;
            let subcube: SubCubeWire = serde_json::from_str(&subcube)?;
            let params = CryptoVerifyParams {
                seed: SeedParam::Text(seed),
                x: reveal,
                subcube,
                config: Some(config_param),
            };
            let cube = bingocube_ipc::cube_from_params(params.seed, params.config)?;
            let sub = params.subcube.into_subcube()?;
            let valid = cube.verify_subcube(&sub, params.x);
            let output = json!({ "valid": valid });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
