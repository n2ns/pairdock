use anyhow::{Context, Result};
use boltkit_core::{
    enumerate::{list_receivers, open_receiver},
    hidpp::receiver::{Receiver, bolt},
};
use clap::{Parser, Subcommand};
use hidapi::HidApi;

#[derive(Parser)]
#[command(version, about = "Pair Logitech devices without Logi Options+")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List receivers and the devices paired to them.
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let api = HidApi::new().context("could not initialize hidapi")?;

    match cli.command {
        Command::List => list(&api).await,
    }
}

async fn list(api: &HidApi) -> Result<()> {
    let receivers = list_receivers(api);
    if receivers.is_empty() {
        println!("No supported receiver found.");
        return Ok(());
    }

    for iface in &receivers {
        let (_chan, receiver) = open_receiver(api, iface)
            .await
            .with_context(|| format!("could not open receiver {:04x}", iface.product_id))?;

        let unique_id = receiver.get_unique_id().await.unwrap_or_default();
        println!("{} ({unique_id})", receiver.name());

        match &receiver {
            Receiver::Bolt(bolt) => list_bolt(bolt).await?,
            _ => println!("  (listing devices is not supported for this receiver yet)"),
        }
    }

    Ok(())
}

async fn list_bolt(bolt: &bolt::Receiver) -> Result<()> {
    let count = bolt.count_pairings().await?;
    println!("  {count} paired device(s)");

    for slot in 1..=6u8 {
        let Ok(info) = bolt.get_device_pairing_information(slot).await else {
            continue;
        };
        let name = bolt.get_device_codename(slot).await.unwrap_or_default();
        println!(
            "  [{slot}] {name:<14} {:?}  wpid={:04X}  {}",
            info.kind,
            info.wpid,
            if info.online { "online" } else { "offline" },
        );
    }

    Ok(())
}
