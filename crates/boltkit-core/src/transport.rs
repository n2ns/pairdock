//! HID transport built on `hidapi`.
//!
//! Logitech exposes HID++ through vendor-defined HID collections. On Windows
//! (and macOS) the short (7 byte) and long (20 byte) report collections show
//! up as two separate HID interfaces, so one [`HidappChannel`] may own up to
//! two `hidapi` handles and routes each report to the handle that supports
//! its report ID.

use std::{
    error::Error,
    sync::{
        Arc,
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use hidapi::{HidApi, HidDevice};
use hidpp::{
    async_trait,
    channel::{LONG_REPORT_ID, LONG_REPORT_LENGTH, RawHidChannel, SHORT_REPORT_ID},
};

/// How long a reader thread blocks in one `read_timeout` call before it checks
/// the stop flag again.
const READ_POLL: Duration = Duration::from_millis(100);

/// A HID++ capable channel made of up to two `hidapi` handles.
pub struct HidappChannel {
    vendor_id: u16,
    product_id: u16,
    // `HidDevice` is `Send` but not `Sync`; writes are short, so a plain mutex
    // is enough to share the write handles with the async trait methods.
    short: Option<Mutex<HidDevice>>,
    long: Option<Mutex<HidDevice>>,
    incoming: async_channel::Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
    readers: Vec<JoinHandle<()>>,
}

impl HidappChannel {
    /// Opens the given HID paths. At least one of `short_path` / `long_path`
    /// must be present.
    pub fn open(
        api: &HidApi,
        vendor_id: u16,
        product_id: u16,
        short_path: Option<&std::ffi::CStr>,
        long_path: Option<&std::ffi::CStr>,
    ) -> Result<Self, hidapi::HidError> {
        let short = short_path.map(|p| api.open_path(p).map(Mutex::new)).transpose()?;
        let long = long_path.map(|p| api.open_path(p).map(Mutex::new)).transpose()?;

        let (tx, rx) = async_channel::unbounded::<Vec<u8>>();
        let stop = Arc::new(AtomicBool::new(false));
        let mut readers = Vec::new();

        // Readers need their own handles; hidapi handles can't be shared across
        // threads, so re-open the path for reading and keep the first handle for
        // writing.
        for (path, max_len) in [
            (short_path, LONG_REPORT_LENGTH),
            (long_path, LONG_REPORT_LENGTH),
        ] {
            let Some(path) = path else { continue };
            let dev = api.open_path(path)?;
            readers.push(spawn_reader(dev, max_len, tx.clone(), Arc::clone(&stop)));
        }

        Ok(Self {
            vendor_id,
            product_id,
            short,
            long,
            incoming: rx,
            stop,
            readers,
        })
    }
}

fn spawn_reader(
    dev: HidDevice,
    max_len: usize,
    tx: async_channel::Sender<Vec<u8>>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = vec![0u8; max_len];
        while !stop.load(Ordering::Relaxed) {
            match dev.read_timeout(&mut buf, READ_POLL.as_millis() as i32) {
                Ok(0) => continue,
                Ok(n) => {
                    if tx.send_blocking(buf[..n].to_vec()).is_err() {
                        break;
                    }
                },
                Err(err) => {
                    tracing::debug!("HID read failed, stopping reader: {err}");
                    break;
                },
            }
        }
    })
}

impl Drop for HidappChannel {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for hdl in self.readers.drain(..) {
            let _ = hdl.join();
        }
    }
}

#[async_trait]
impl RawHidChannel for HidappChannel {
    fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    fn product_id(&self) -> u16 {
        self.product_id
    }

    async fn write_report(&self, src: &[u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
        let dev = match src.first() {
            Some(&SHORT_REPORT_ID) => self.short.as_ref().or(self.long.as_ref()),
            Some(&LONG_REPORT_ID) => self.long.as_ref(),
            _ => None,
        };
        let Some(dev) = dev else {
            return Err("no HID handle supports this report".into());
        };
        let dev = dev.lock().unwrap();

        // A short message sent through the long collection has to be padded to
        // the long report length.
        if src[0] == SHORT_REPORT_ID && self.short.is_none() {
            let mut padded = vec![0u8; LONG_REPORT_LENGTH];
            padded[0] = LONG_REPORT_ID;
            padded[1..src.len()].copy_from_slice(&src[1..]);
            dev.write(&padded)?;
            return Ok(src.len());
        }

        Ok(dev.write(src)?)
    }

    async fn read_report(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
        let report = self
            .incoming
            .recv()
            .await
            .map_err(|_| "all HID readers stopped")?;
        let n = report.len().min(buf.len());
        buf[..n].copy_from_slice(&report[..n]);
        Ok(n)
    }

    fn supports_short_long_hidpp(&self) -> Option<(bool, bool)> {
        Some((self.short.is_some(), self.long.is_some()))
    }

    async fn get_report_descriptor(
        &self,
        _buf: &mut [u8],
    ) -> Result<usize, Box<dyn Error + Sync + Send>> {
        Err("report descriptors are not needed: support is known from the HID usage".into())
    }
}
