//! The resident engine that applies button assignments and turns diverted
//! button presses into keyboard shortcuts.
//!
//! Diversion and remapping are not persistent on most devices: they reset
//! when the device reconnects (wakes up, is switched off and on, ...). The
//! engine therefore listens for connection notifications and re-applies the
//! configuration every time a device comes online.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use enigo::{Direction, Enigo, Settings};
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
    enumerate::{ChannelRole, list_receivers, open_receiver},
    keys::{self, Shortcut, parse_shortcut},
};

/// How often the engine looks for newly plugged-in receivers.
const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// How often a receiver task checks whether its receiver is still there.
const LIVENESS_INTERVAL: Duration = Duration::from_secs(1);

/// Upper bound for a single request to a device. Devices that go to sleep in
/// the middle of a request never answer.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

/// The running engine. Dropping it without [`RemapEngine::stop`] leaves
/// diverted buttons diverted until the device reconnects.
pub struct RemapEngine {
    config: watch::Sender<Config>,
    stop: watch::Sender<bool>,
    task: JoinHandle<bool>,
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
    /// that does not answer. Returns whether every device was restored.
    pub async fn stop(self, limit: Duration) -> bool {
        self.stop.send_replace(true);
        match timeout(limit, self.task).await {
            Ok(Ok(restored)) => restored,
            Ok(Err(_)) => false,
            Err(_) => {
                tracing::warn!("remap engine did not stop within {limit:?}");
                false
            },
        }
    }
}

/// Returns whether every device was restored.
async fn run(config: watch::Receiver<Config>, mut stop: watch::Receiver<bool>, injector: Injector) -> bool {
    let mut receivers: HashMap<String, (watch::Sender<bool>, JoinHandle<bool>)> = HashMap::new();

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
                    match open_receiver(&api, &iface, ChannelRole::Engine).await {
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

    // Stop all receivers first so that they restore their devices in parallel.
    let tasks: Vec<_> = receivers
        .into_values()
        .map(|(stop, task)| {
            stop.send_replace(true);
            task
        })
        .collect();
    futures::future::join_all(tasks)
        .await
        .into_iter()
        .all(|restored| restored.unwrap_or(false))
}

/// Serves one Bolt receiver until it is unplugged or the engine stops.
/// Returns whether its devices were restored.
async fn run_bolt(
    chan: Arc<HidppChannel>,
    bolt: bolt::Receiver,
    mut config: watch::Receiver<Config>,
    mut stop: watch::Receiver<bool>,
    injector: Injector,
) -> bool {
    let events = bolt.listen();
    let notifications = bolt::NotificationStateBuilder::default()
        .wireless_notifications(true)
        .build()
        .expect("all fields are set");
    if let Err(err) = with_timeout(bolt.set_notification_state(notifications)).await {
        tracing::warn!("could not enable receiver notifications: {err}");
    }

    let mut devices: HashMap<u8, LiveDevice> = HashMap::new();
    // Online devices that could not be reached, so their buttons may still
    // carry settings that stopping cannot restore.
    let mut unreached: HashSet<u8> = HashSet::new();

    // The receiver announces every paired device with a connection
    // notification, which the loop below handles like any other.
    if let Err(err) = with_timeout(bolt.trigger_device_arrival()).await {
        tracing::warn!("could not list paired devices: {err}");
    }

    let mut liveness = tokio::time::interval(LIVENESS_INTERVAL);
    loop {
        tokio::select! {
            // A receiver that is unplugged and plugged back in between two
            // scans keeps its id, so the engine only reopens it once this task
            // ends. The event stream never closes on its own.
            _ = liveness.tick() => {
                if chan.is_closed() {
                    tracing::info!("receiver disconnected");
                    return true;
                }
            },
            event = events.recv() => {
                let Ok(event) = event else { break };
                if let bolt::Event::DeviceConnection(conn) = event {
                    if conn.online {
                        if attach(&chan, &mut devices, conn.index, conn.wpid, &config, &injector).await {
                            unreached.remove(&conn.index);
                        } else {
                            unreached.insert(conn.index);
                        }
                    } else {
                        devices.remove(&conn.index);
                        unreached.remove(&conn.index);
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

    // Devices announced but not handled yet were not restored either.
    while let Ok(event) = events.try_recv() {
        if let bolt::Event::DeviceConnection(conn) = event {
            if conn.online {
                unreached.insert(conn.index);
            } else {
                unreached.remove(&conn.index);
            }
        }
    }

    restore(devices.values()).await && unreached.is_empty()
}

/// Returns every control of `devices` to its default behavior.
/// Devices are restored concurrently so that one that does not answer cannot
/// use up the time the others need.
/// Returns whether all of them answered.
async fn restore(devices: impl IntoIterator<Item = &LiveDevice>) -> bool {
    let defaults = Config::default();
    futures::future::join_all(devices.into_iter().map(|device| device.apply(&defaults)))
        .await
        .into_iter()
        .all(|answered| answered)
}

/// (Re-)initializes a device that just came online. Returns whether the
/// device answered.
async fn attach(
    chan: &Arc<HidppChannel>,
    devices: &mut HashMap<u8, LiveDevice>,
    index: u8,
    wpid: u16,
    config: &watch::Receiver<Config>,
    injector: &Injector,
) -> bool {
    devices.remove(&index);
    match LiveDevice::open(Arc::clone(chan), index, wpid, config.clone(), injector.clone()).await {
        Ok(Some(device)) => {
            let current = config.borrow().clone();
            let answered = device.apply(&current).await;
            devices.insert(index, device);
            answered
        },
        Ok(None) => {
            tracing::debug!("device {index} has no reprogrammable controls");
            true
        },
        Err(err) => {
            tracing::warn!("could not initialize device {index}: {err}");
            false
        },
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

    /// Brings every control's reporting in line with `config`. Returns false
    /// if the device stopped answering.
    async fn apply(&self, config: &Config) -> bool {
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

            let request = self.reprog.set_reporting(control.cid, divert, None, None, remap);
            match timeout(REQUEST_TIMEOUT, request).await {
                Ok(Ok(())) => {},
                Ok(Err(err)) => tracing::warn!(
                    "device {}: could not configure control {:#06x}: {err}",
                    self.index,
                    control.cid
                ),
                // An asleep device would let every remaining control time out
                // as well; it is configured again when it comes back online.
                Err(_) => {
                    tracing::warn!("device {} did not answer", self.index);
                    return false;
                },
            }
        }
        true
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
    let mut held = HeldShortcuts {
        keys: HashMap::new(),
        injector: injector.clone(),
    };
    let held = &mut held.keys;

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
}

/// Releases the shortcuts still held down when the listener ends, including
/// when it is aborted because the device went away.
struct HeldShortcuts {
    keys: HashMap<u16, Shortcut>,
    injector: Injector,
}

impl Drop for HeldShortcuts {
    fn drop(&mut self) {
        for (_, shortcut) in self.keys.drain() {
            self.injector.release(shortcut);
        }
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
            // Without the permission to send keys (Accessibility on macOS),
            // creating `Enigo` fails; it is tried again with every press, so
            // shortcuts start working as soon as the user grants it. Asking
            // for the permission is left to the UI.
            let settings = Settings {
                open_prompt_to_get_permissions: false,
                ..Settings::default()
            };
            let mut enigo = None;
            let mut warned = false;
            while let Ok((shortcut, direction)) = rx.recv() {
                if enigo.is_none() {
                    match Enigo::new(&settings) {
                        Ok(created) => enigo = Some(created),
                        Err(err) if !warned => {
                            tracing::error!("cannot synthesize input: {err}");
                            warned = true;
                        },
                        Err(_) => {},
                    }
                }
                let Some(enigo) = enigo.as_mut() else { continue };
                keys::send(enigo, &shortcut, direction);
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

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        error::Error,
        sync::{
            Arc, Condvar, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    use enigo::Direction;
    use hidpp::{
        async_trait,
        channel::{HidppChannel, RawHidChannel},
        feature::{
            CreatableFeature,
            reprog_controls::{ControlFlags, ControlInfo, ReprogControlsEvent, ReprogControlsFeature},
        },
        receiver::{Receiver, detect},
    };
    use tokio::sync::watch;

    use super::{Injector, LiveDevice, RemapEngine, handle_presses, restore, run_bolt};
    use crate::config::{ButtonAction, Config};

    /// A Bolt receiver that answers its own requests and those of the devices
    /// in `answering` by echoing them, and records every report it is sent.
    /// Triggering device arrival announces the devices in `online`.
    struct ScriptedReceiver {
        answering: Vec<u8>,
        online: Vec<u8>,
        sent: Arc<Mutex<Vec<Vec<u8>>>>,
        inbox: (Mutex<VecDeque<Vec<u8>>>, Condvar),
    }

    impl ScriptedReceiver {
        fn new(answering: Vec<u8>, online: Vec<u8>) -> (Self, Arc<Mutex<Vec<Vec<u8>>>>) {
            let sent = Arc::new(Mutex::new(Vec::new()));
            let receiver = Self {
                answering,
                online,
                sent: Arc::clone(&sent),
                inbox: Default::default(),
            };
            (receiver, sent)
        }
    }

    #[async_trait]
    impl RawHidChannel for ScriptedReceiver {
        fn vendor_id(&self) -> u16 {
            0x046d
        }

        fn product_id(&self) -> u16 {
            0xc548
        }

        async fn write_report(&self, src: &[u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            self.sent.lock().unwrap().push(src.to_vec());
            let (queue, ready) = &self.inbox;
            let mut queue = queue.lock().unwrap();
            if src[1] == 0xff && src[2] == 0x80 && src[3] == 0x02 {
                // Device arrival: one "mouse B02A online" notification each.
                for &index in &self.online {
                    queue.push_back(vec![0x10, index, 0x41, 0x04, 0x02, 0x2a, 0xb0]);
                }
            }
            if src[1] == 0xff || self.answering.contains(&src[1]) {
                queue.push_back(src.to_vec());
            }
            ready.notify_all();
            Ok(src.len())
        }

        async fn read_report(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            let (queue, ready) = &self.inbox;
            let mut queue = queue.lock().unwrap();
            if queue.is_empty() {
                queue = ready.wait_timeout(queue, Duration::from_millis(50)).unwrap().0;
            }
            match queue.pop_front() {
                Some(report) => {
                    buf[..report.len()].copy_from_slice(&report);
                    Ok(report.len())
                },
                None => Ok(0),
            }
        }

        fn supports_short_long_hidpp(&self) -> Option<(bool, bool)> {
            Some((true, true))
        }

        async fn get_report_descriptor(
            &self,
            _buf: &mut [u8],
        ) -> Result<usize, Box<dyn Error + Sync + Send>> {
            unreachable!()
        }
    }

    fn sent_to(sent: &Mutex<Vec<Vec<u8>>>, index: u8) -> usize {
        sent.lock().unwrap().iter().filter(|r| r[1] == index).count()
    }

    /// A device with `controls` divertable controls.
    fn live_device(chan: &Arc<HidppChannel>, index: u8, controls: u16) -> LiveDevice {
        LiveDevice {
            index,
            wpid: 0xb02a,
            reprog: Arc::new(ReprogControlsFeature::new(Arc::clone(chan), index, 0x08)),
            controls: (0..controls)
                .map(|i| ControlInfo {
                    cid: 0x53 + i,
                    task_id: 0,
                    flags: ControlFlags(ControlFlags::DIVERTABLE),
                    position: 0,
                    group: 0,
                    group_mask: 0,
                })
                .collect(),
            listener: tokio::spawn(async {}),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stopping_the_listener_releases_held_shortcuts() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut config = Config::default();
        config.devices.entry(0xb02a).or_default().buttons.insert(0x53, ButtonAction::Shortcut {
            keys: "Ctrl+C".into(),
        });
        let (_config_tx, config_rx) = watch::channel(config);
        let (events_tx, events_rx) = async_channel::unbounded();
        let listener = tokio::spawn(handle_presses(events_rx, 0xb02a, config_rx, Injector(tx)));

        events_tx.send(ReprogControlsEvent::DivertedButtons(vec![0x53])).await.unwrap();
        let (_, direction) = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(direction, Direction::Press);

        // The device goes offline while the button is held down.
        listener.abort();
        let _ = listener.await;
        let (_, direction) = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("the held shortcut was never released");
        assert_eq!(direction, Direction::Release);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_silent_device_does_not_hold_up_restoring_the_others() {
        let (receiver, sent) = ScriptedReceiver::new(vec![2], vec![]);
        let chan = Arc::new(HidppChannel::from_raw_channel(receiver).await.unwrap());
        let asleep = live_device(&chan, 1, 3);
        let awake = live_device(&chan, 2, 1);

        let started = Instant::now();
        let finished = tokio::time::timeout(Duration::from_secs(3), restore([&asleep, &awake])).await;

        assert_eq!(sent_to(&sent, 2), 1, "the awake device was not restored");
        let restored = finished.unwrap_or_else(|_| panic!("restoring took {:?}", started.elapsed()));
        assert!(!restored, "the asleep device was reported as restored");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_device_that_never_answered_is_not_reported_as_restored() {
        let (receiver, _sent) = ScriptedReceiver::new(vec![], vec![1]);
        let chan = Arc::new(HidppChannel::from_raw_channel(receiver).await.unwrap());
        let Some(Receiver::Bolt(bolt)) = detect(Arc::clone(&chan)) else {
            panic!("not detected as a Bolt receiver");
        };

        let (_config_tx, config_rx) = watch::channel(Config::default());
        let (stop_tx, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run_bolt(chan, bolt, config_rx, stop_rx, Injector::spawn()));

        // Device 1 is announced online but never answers, so it is not attached.
        tokio::time::sleep(Duration::from_secs(3)).await;
        stop_tx.send_replace(true);
        let restored = tokio::time::timeout(Duration::from_secs(3), task).await.unwrap().unwrap();
        assert!(!restored, "a device that was never reached was reported as restored");
    }

    #[tokio::test]
    async fn stop_reports_devices_that_were_not_restored() {
        let engine = RemapEngine {
            config: watch::channel(Config::default()).0,
            stop: watch::channel(false).0,
            task: tokio::spawn(async { false }),
        };
        assert!(!engine.stop(Duration::from_secs(1)).await);
    }

    #[tokio::test]
    async fn stop_reports_restoring_that_did_not_finish() {
        let engine = RemapEngine {
            config: watch::channel(Config::default()).0,
            stop: watch::channel(false).0,
            task: tokio::spawn(std::future::pending()),
        };
        assert!(!engine.stop(Duration::from_millis(10)).await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_device_online_at_startup_is_initialized_once() {
        let (receiver, sent) = ScriptedReceiver::new(vec![], vec![1]);
        let chan = Arc::new(HidppChannel::from_raw_channel(receiver).await.unwrap());
        let Some(Receiver::Bolt(bolt)) = detect(Arc::clone(&chan)) else {
            panic!("not detected as a Bolt receiver");
        };

        let (_config_tx, config_rx) = watch::channel(Config::default());
        let (_stop_tx, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run_bolt(chan, bolt, config_rx, stop_rx, Injector::spawn()));

        // The device never answers, so each initialization is one request
        // that times out after two seconds.
        tokio::time::sleep(Duration::from_secs(5)).await;
        task.abort();
        assert_eq!(sent_to(&sent, 1), 1);
    }

    /// A Bolt receiver that never answers and can be "unplugged", after which
    /// every read fails like a vanished HID handle.
    struct FakeReceiver {
        unplugged: Arc<AtomicBool>,
    }

    #[async_trait]
    impl RawHidChannel for FakeReceiver {
        fn vendor_id(&self) -> u16 {
            0x046d
        }

        fn product_id(&self) -> u16 {
            0xc548
        }

        async fn write_report(&self, src: &[u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            Ok(src.len())
        }

        async fn read_report(&self, _buf: &mut [u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            if self.unplugged.load(Ordering::SeqCst) {
                return Err("device disconnected".into());
            }
            std::thread::sleep(Duration::from_millis(20));
            Ok(0)
        }

        fn supports_short_long_hidpp(&self) -> Option<(bool, bool)> {
            Some((true, true))
        }

        async fn get_report_descriptor(
            &self,
            _buf: &mut [u8],
        ) -> Result<usize, Box<dyn Error + Sync + Send>> {
            unreachable!()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn receiver_task_ends_when_the_receiver_is_unplugged() {
        let unplugged = Arc::new(AtomicBool::new(false));
        let chan = Arc::new(
            HidppChannel::from_raw_channel(FakeReceiver {
                unplugged: Arc::clone(&unplugged),
            })
            .await
            .unwrap(),
        );
        let Some(Receiver::Bolt(bolt)) = detect(Arc::clone(&chan)) else {
            panic!("not detected as a Bolt receiver");
        };

        let (_config_tx, config_rx) = watch::channel(Config::default());
        let (_stop_tx, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run_bolt(chan, bolt, config_rx, stop_rx, Injector::spawn()));

        // Startup requests time out (nobody answers), then the task waits for
        // events. Unplug the receiver while it waits.
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(!task.is_finished());
        unplugged.store(true, Ordering::SeqCst);

        // The engine only reopens receivers whose task ended, so it must end.
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .expect("the receiver task kept running on an unplugged receiver")
            .unwrap();
    }
}
