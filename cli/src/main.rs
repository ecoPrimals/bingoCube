// SPDX-License-Identifier: AGPL-3.0-or-later
//! `BingoCube` `UniBin` — serve, demo, generate, and verify.

use std::path::PathBuf;

use bingocube_core::BingoCube;
use bingocube_ipc::{
    ConfigParam, ConfigPreset, CryptoVerifyParams, SeedParam, ServeConfig, SubCubeWire, VERSION,
    commitment_hash, resolve_socket_dir, serve,
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
    /// Start JSON-RPC (+ tarpc) IPC servers.
    Serve {
        /// Directory for `.sock` files.
        #[arg(long, default_value = "/run/bingocube")]
        socket_dir: PathBuf,

        /// Disable the tarpc binary socket.
        #[arg(long)]
        no_tarpc: bool,

        /// G65 single-socket protocol negotiation (tarpc vs JSON-RPC on one socket).
        #[arg(long, env = "BINGOCUBE_NEGOTIATE")]
        negotiate: bool,

        /// Optional TCP fallback port on 127.0.0.1 for JSON-RPC.
        #[arg(long)]
        tcp_port: Option<u16>,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        None => {
            // clap prints help when no subcommand and --help is passed; for bare invocation show help.
            Cli::command().print_help()?;
            println!();
        }
        Some(Commands::Serve {
            socket_dir,
            no_tarpc,
            negotiate,
            tcp_port,
        }) => {
            let config = ServeConfig {
                socket_dir: resolve_socket_dir(Some(socket_dir)),
                enable_tarpc: !no_tarpc,
                negotiate,
                tcp_port,
            };
            if negotiate {
                tracing::info!("G65 single-socket protocol negotiation enabled");
            }
            let endpoints = serve(config).await?;
            tracing::info!(
                jsonrpc = %endpoints.jsonrpc_socket.display(),
                tarpc = ?endpoints.tarpc_socket.as_ref().map(|p| p.display().to_string()),
                tcp = ?endpoints.tcp_addr,
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
