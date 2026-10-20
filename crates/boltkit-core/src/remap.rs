//! The resident engine that applies button assignments and turns diverted
//! button presses into keyboard shortcuts.
//!
//! Diversion and remapping are not persistent on most devices: they reset
//! when the device reconnects (wakes up, is switched off and on, ...). The
//! engine therefore listens for connection notifications and re-applies the
//! configuration every time a device comes online.

use std::{collections::HashMap, sync::Arc, time::Duration};

use enigo::{Direction, Enigo, Keyboard, Settings};
use hidapi::HidApi;
use hidpp::{
    channel::HidppChannel,
    device::Device,
    feature::{
        EmittingFeature,
        reprog_controls::{ControlFlags, ControlInfo, ReprogControlsEvent, ReprogControlsFeature},
    },
    receiver::{Receiver, bolt},
};
use tokio::{sync::watch, task::JoinHandle, time::timeout};

use crate::{
    config::{ButtonAction, Config},
    enumerate::{list_receivers, open_receiver},
    keys::{Shortcut, parse_shortcut},
};

/// How often the engine looks for newly plugged-in receivers.
const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// Upper bound for a single request to a device. Devices that go to sleep in
/// the middle of a request never answer.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

/// The running engine. Dropping it without [`RemapEngine::stop`] leaves
/// diverted buttons diverted until the device reconnects.
pub struct RemapEngine {
    config: watch::Sender<Config>,
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl RemapEngine {
    /// Starts the engine on the current tokio runtime.
    pub fn start(config: Config) -> Self {
        let (config_tx, config_rx) = watch::channel(config);
        let (stop_tx, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run(config_rx, stop_rx, Injector::spawn()));
        Self {
            config: config_tx,
            stop: stop_tx,
            task,
        }
    }

    /// Applies a new configuration to all connected devices.
    pub fn set_config(&self, config: Config) {
        self.config.send_replace(config);
    }

    /// Restores every touched button to its default behavior and stops.
    ///
    /// Gives up after `limit` so that shutting down never hangs on a device
    /// that does not answer.
    pub async fn stop(self, limit: Duration) {
        self.stop.send_replace(true);
        if timeout(limit, self.task).await.is_err() {
            tracing::warn!("remap engine did not stop within {limit:?}");
        }
    }
}

async fn run(config: watch::Receiver<Config>, mut stop: watch::Receiver<bool>, injector: Injector) {
    let mut receivers: HashMap<String, (watch::Sender<bool>, JoinHandle<()>)> = HashMap::new();

    loop {
        match HidApi::new() {
            Ok(api) => {
                let present = list_receivers(&api);

                // Stop tasks whose receiver disappeared or whose task ended.
                receivers.retain(|key, (stop, task)| {
                    let keep = !task.is_finished()
                        && present.iter().any(|iface| iface.id() == *key);
                    if !keep {
                        stop.send_replace(true);
                    }
                    keep
                });

                for iface in present {
                    let key = iface.id();
                    if receivers.contains_key(&key) {
                        continue;
                    }
                    match open_receiver(&api, &iface).await {
                        Ok((chan, Receiver::Bolt(bolt))) => {
                            let (stop_tx, stop_rx) = watch::channel(false);
                            let task = tokio::spawn(run_bolt(
                                chan,
                                bolt,
                                config.clone(),
                                stop_rx,
                                injector.clone(),
                            ));
                            receivers.insert(key, (stop_tx, task));
                        },
                        Ok(_) => tracing::debug!("skipping unsupported receiver {key}"),
                        Err(err) => tracing::warn!("could not open receiver {key}: {err}"),
                    }
                }
            },
            Err(err) => tracing::warn!("could not enumerate HID devices: {err}"),
        }

        tokio::select! {
            _ = tokio::time::sleep(RESCAN_INTERVAL) => {},
            _ = stopped(&mut stop) => break,
        }
    }

    for (stop, task) in receivers.into_values() {
        stop.send_replace(true);
        let _ = task.await;
    }
}

/// Serves one Bolt receiver until it is unplugged or the engine stops.
async fn run_bolt(
    chan: Arc<HidppChannel>,
    bolt: bolt::Receiver,
    mut config: watch::Receiver<Config>,
    mut stop: watch::Receiver<bool>,
    injector: Injector,
) {
    let events = bolt.listen();
    let notifications = bolt::NotificationStateBuilder::default()
        .wireless_notifications(true)
        .build()
        .expect("all fields are set");
    if let Err(err) = with_timeout(bolt.set_notification_state(notifications)).await {
        tracing::warn!("could not enable receiver notifications: {err}");
    }

    let mut devices: HashMap<u8, LiveDevice> = HashMap::new();

    match with_timeout(bolt.collect_paired_devices()).await {
        Ok(paired) => {
            for conn in paired.into_iter().filter(|c| c.online) {
                attach(&chan, &mut devices, conn.index, conn.wpid, &config, &injector).await;
            }
        },
        Err(err) => tracing::warn!("could not list paired devices: {err}"),
    }

    loop {
        tokio::select! {
            event = events.recv() => {
                let Ok(event) = event else { break };
                if let bolt::Event::DeviceConnection(conn) = event {
                    if conn.online {
                        attach(&chan, &mut devices, conn.index, conn.wpid, &config, &injector).await;
                    } else {
                        devices.remove(&conn.index);
                    }
                }
            },
            changed = config.changed() => {
                if changed.is_err() { break; }
                let current = config.borrow_and_update().clone();
                for device in devices.values() {
                    device.apply(&current).await;
                }
            },
            _ = stopped(&mut stop) => break,
        }
    }

    for device in devices.values() {
        device.apply(&Config::default()).await;
    }
}

/// (Re-)initializes a device that just came online.
async fn attach(
    chan: &Arc<HidppChannel>,
    devices: &mut HashMap<u8, LiveDevice>,
    index: u8,
    wpid: u16,
    config: &watch::Receiver<Config>,
    injector: &Injector,
) {
    devices.remove(&index);
    match LiveDevice::open(Arc::clone(chan), index, wpid, config.clone(), injector.clone()).await {
        Ok(Some(device)) => {
            let current = config.borrow().clone();
            device.apply(&current).await;
            devices.insert(index, device);
        },
        Ok(None) => tracing::debug!("device {index} has no reprogrammable controls"),
        Err(err) => tracing::warn!("could not initialize device {index}: {err}"),
    }
}

/// A connected device with reprogrammable controls.
struct LiveDevice {
    index: u8,
    wpid: u16,
    reprog: Arc<ReprogControlsFeature>,
    controls: Vec<ControlInfo>,
    listener: JoinHandle<()>,
}

impl Drop for LiveDevice {
    fn drop(&mut self) {
        self.listener.abort();
    }
}

impl LiveDevice {
    async fn open(
        chan: Arc<HidppChannel>,
        index: u8,
        wpid: u16,
        config: watch::Receiver<Config>,
        injector: Injector,
    ) -> Result<Option<Self>, String> {
        let mut device = with_timeout(Device::new(chan, index)).await?;
        with_timeout(device.enumerate_features()).await?;
        let Some(reprog) = device.get_feature::<ReprogControlsFeature>() else {
            return Ok(None);
        };

        let count = with_timeout(reprog.get_count()).await?;
        let mut controls = Vec::with_capacity(count as usize);
        for i in 0..count {
            controls.push(with_timeout(reprog.get_control_info(i)).await?);
        }

        let listener = tokio::spawn(handle_presses(reprog.listen(), wpid, config, injector));
        Ok(Some(Self {
            index,
            wpid,
            reprog,
            controls,
            listener,
        }))
    }

    /// Brings every control's reporting in line with `config`.
    async fn apply(&self, config: &Config) {
        for control in &self.controls {
            let action = config.action(self.wpid, control.cid);
            let divertable = control.flags.has(ControlFlags::DIVERTABLE);
            let remappable = control.flags.has(ControlFlags::REPROGRAMMABLE);

            let (divert, remap) = match &action {
                ButtonAction::Default => (divertable.then_some(false), remappable.then_some(control.cid)),
                ButtonAction::Remap { to } if remappable => (divertable.then_some(false), Some(*to)),
                action if action.needs_divert() && divertable => (Some(true), None),
                _ => {
                    tracing::warn!(
                        "device {} cannot use {action:?} on control {:#06x}",
                        self.index,
                        control.cid
                    );
                    continue;
                },
            };
            if divert.is_none() && remap.is_none() {
                continue;
            }

            if let Err(err) =
                with_timeout(self.reprog.set_reporting(control.cid, divert, None, None, remap)).await
            {
                tracing::warn!(
                    "device {}: could not configure control {:#06x}: {err}",
                    self.index,
                    control.cid
                );
            }
        }
    }
}

/// Turns diverted button events into shortcut presses and releases.
async fn handle_presses(
    events: async_channel::Receiver<ReprogControlsEvent>,
    wpid: u16,
    config: watch::Receiver<Config>,
    injector: Injector,
) {
    // Shortcuts currently held down, by control. Releases use the shortcut
    // that was pressed even if the configuration changed in between.
    let mut held: HashMap<u16, Shortcut> = HashMap::new();

    while let Ok(event) = events.recv().await {
        let ReprogControlsEvent::DivertedButtons(pressed) = event else {
            continue;
        };

        let released: Vec<u16> = held.keys().filter(|c| !pressed.contains(c)).copied().collect();
        for cid in released {
            if let Some(shortcut) = held.remove(&cid) {
                injector.release(shortcut);
            }
        }

        for cid in pressed {
            if held.contains_key(&cid) {
                continue;
            }
            let ButtonAction::Shortcut { keys } = config.borrow().action(wpid, cid) else {
                continue;
            };
            match parse_shortcut(&keys) {
                Ok(shortcut) => {
                    injector.press(shortcut.clone());
                    held.insert(cid, shortcut);
                },
                Err(err) => tracing::warn!("invalid shortcut {keys:?}: {err}"),
            }
        }
    }

    for shortcut in held.into_values() {
        injector.release(shortcut);
    }
}

/// Resolves once `stop` is set. Unlike `wait_for` it holds no borrow of the
/// value, so it can sit in a `select!` next to branches that await.
async fn stopped(stop: &mut watch::Receiver<bool>) {
    let _ = stop.wait_for(|stop| *stop).await;
}

async fn with_timeout<T, E: std::fmt::Display>(
    fut: impl Future<Output = Result<T, E>>,
) -> Result<T, String> {
    match timeout(REQUEST_TIMEOUT, fut).await {
        Ok(result) => result.map_err(|e| e.to_string()),
        Err(_) => Err("the device did not answer".into()),
    }
}

/// Sends synthesized key events from a dedicated thread; `enigo` handles are
/// not meant to be shared across threads.
#[derive(Clone)]
struct Injector(std::sync::mpsc::Sender<(Shortcut, Direction)>);

impl Injector {
    fn spawn() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<(Shortcut, Direction)>();
        std::thread::spawn(move || {
            let mut enigo = match Enigo::new(&Settings::default()) {
                Ok(enigo) => enigo,
                Err(err) => {
                    tracing::error!("cannot synthesize input: {err}");
                    return;
                },
            };
            while let Ok((Shortcut(keys), direction)) = rx.recv() {
                let ordered: Box<dyn Iterator<Item = _>> = match direction {
                    Direction::Release => Box::new(keys.into_iter().rev()),
                    _ => Box::new(keys.into_iter()),
                };
                for key in ordered {
                    if let Err(err) = enigo.key(key, direction) {
                        tracing::warn!("could not send {key:?}: {err}");
                    }
                }
            }
        });
        Self(tx)
    }

    fn press(&self, shortcut: Shortcut) {
        let _ = self.0.send((shortcut, Direction::Press));
    }

    fn release(&self, shortcut: Shortcut) {
        let _ = self.0.send((shortcut, Direction::Release));
    }
}
