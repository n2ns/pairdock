use std::time::Duration;

use anyhow::{Context, Result, bail};
use boltkit_core::{
    enumerate::{list_receivers, open_receiver},
    hidpp::receiver::{Receiver, bolt},
    pairing::{BoltPairing, Click, PairingEvent, PasskeyPress, PasskeyPrompt},
};
use clap::{Parser, Subcommand};
use hidapi::HidApi;

#[derive(Parser)]
#[command(version, about = "Pair Logitech devices without Logi Options+")]
struct Cli {
    /// Write a protocol trace (every HID++ report) to this file.
    #[arg(long, global = true, value_name = "FILE")]
    trace: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List receivers and the devices paired to them.
    List,

    /// Pair the first device found in pairing mode.
    Pair,

    /// Remove the device in the given slot from the receiver.
    Unpair {
        /// Slot number as shown by `list`.
        slot: u8,
    },
}

/// How long `pair` waits for discovery and passkey entry in total.
const PAIR_TIMEOUT: Duration = Duration::from_secs(120);


#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(path) = &cli.trace {
        let file = std::fs::File::create(path)
            .with_context(|| format!("could not create {}", path.display()))?;
        tracing_subscriber::fmt()
            .with_env_filter("boltkit_core=trace")
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .init();
    }

    let api = HidApi::new().context("could not initialize hidapi")?;

    match cli.command {
        Command::List => list(&api).await,
        Command::Pair => pair(&api).await,
        Command::Unpair { slot } => unpair(&api, slot).await,
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

/// Opens the first Bolt receiver. Pairing and unpairing are only implemented
/// for Bolt so far.
async fn first_bolt(
    api: &HidApi,
) -> Result<(std::sync::Arc<boltkit_core::hidpp::channel::HidppChannel>, bolt::Receiver)> {
    for iface in list_receivers(api) {
        let (chan, receiver) = open_receiver(api, &iface).await?;
        if let Receiver::Bolt(bolt) = receiver {
            return Ok((chan, bolt));
        }
    }
    bail!("no Bolt receiver found")
}

async fn pair(api: &HidApi) -> Result<()> {
    let (_chan, bolt) = first_bolt(api).await?;
    let (session, events) = BoltPairing::start(bolt).await?;
    println!("Put the device into pairing mode (long-press its connect button)...");

    let outcome = tokio::time::timeout(PAIR_TIMEOUT, async {
        let mut pairing_started = false;
        let mut clicks_needed = 0;
        let mut clicks_done = 0;
        loop {
            let Ok(event) = events.recv().await else {
                bail!("the receiver stopped sending events")
            };
            match event {
                PairingEvent::DiscoveryStarted => println!("Searching..."),
                PairingEvent::Discovered(device) if !pairing_started => {
                    println!("Found {} ({:?}), pairing...", device.name, device.kind);
                    session.pair(&device).await?;
                    pairing_started = true;
                },
                PairingEvent::DiscoveryStopped { .. } if !pairing_started => {
                    // Discovery times out after 30s; keep searching until the
                    // overall timeout.
                    session.rediscover().await?;
                },
                PairingEvent::Passkey { prompt, .. } => {
                    clicks_needed = match &prompt {
                        PasskeyPrompt::Click(clicks) => clicks.len(),
                        PasskeyPrompt::Type(digits) => digits.len(),
                    };
                    print_prompt(&prompt);
                },
                PairingEvent::PasskeyProgress { press, .. } => match press {
                    PasskeyPress::Key => {
                        clicks_done += 1;
                        println!("  {clicks_done}/{clicks_needed}");
                        if clicks_done == clicks_needed {
                            println!("  Now confirm: press left and right together (or Enter on a keyboard).");
                        }
                    },
                    PasskeyPress::Submitted => println!("  Confirmed."),
                    PasskeyPress::Started => {},
                },
                PairingEvent::Paired { slot } => return Ok(slot),
                PairingEvent::Failed { reason } => bail!("pairing failed: {reason}"),
                _ => {},
            }
        }
    })
    .await;

    session.finish().await?;
    match outcome {
        Ok(Ok(slot)) => println!("Paired to slot {slot}."),
        Ok(Err(err)) => return Err(err),
        Err(_) => bail!("timed out"),
    }
    Ok(())
}

fn print_prompt(prompt: &PasskeyPrompt) {
    match prompt {
        PasskeyPrompt::Type(digits) => {
            println!("Type {digits} on the keyboard, then press Enter.");
        },
        PasskeyPrompt::Click(clicks) => {
            let seq: Vec<&str> = clicks
                .iter()
                .map(|c| match c {
                    Click::Left => "left",
                    Click::Right => "right",
                })
                .collect();
            println!(
                "Click on the mouse: {}, then press left and right together.",
                seq.join(", ")
            );
        },
    }
}

async fn unpair(api: &HidApi, slot: u8) -> Result<()> {
    let (_chan, bolt) = first_bolt(api).await?;
    let name = bolt.get_device_codename(slot).await.unwrap_or_default();
    bolt.unpair_device(slot)
        .await
        .with_context(|| format!("could not unpair slot {slot}"))?;
    println!("Unpaired slot {slot} ({name}).");
    Ok(())
}
