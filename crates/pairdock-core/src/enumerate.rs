//! Finds Logitech receivers and opens HID++ channels to them.

use std::{collections::BTreeMap, ffi::CString, sync::Arc};

use hidapi::HidApi;
use hidpp::{
    nibble::U4,
    channel::{
        HidppChannel,
        LONG_REPORT_USAGE,
        LONG_REPORT_USAGE_PAGE,
        SHORT_REPORT_USAGE,
        SHORT_REPORT_USAGE_PAGE,
    },
    receiver::{self, Receiver},
};

use crate::transport::HidappChannel;

/// Logitech's USB vendor ID.
pub const LOGITECH_VENDOR_ID: u16 = 0x046d;

/// The HID interfaces that make up one receiver's HID++ endpoint.
#[derive(Clone, Debug)]
pub struct ReceiverInterface {
    pub vendor_id: u16,
    pub product_id: u16,
    pub short_path: Option<CString>,
    pub long_path: Option<CString>,
}

impl ReceiverInterface {
    /// A stable identifier of the receiver for as long as it stays plugged
    /// into the same port.
    pub fn id(&self) -> String {
        let path = self.long_path.as_ref().or(self.short_path.as_ref());
        path.map(|p| interface_key(&p.to_string_lossy()))
            .unwrap_or_default()
    }
}

/// Lists all connected receivers that pairdock knows how to talk to.
pub fn list_receivers(api: &HidApi) -> Vec<ReceiverInterface> {
    let mut found: BTreeMap<String, ReceiverInterface> = BTreeMap::new();

    for info in api.device_list() {
        if info.vendor_id() != LOGITECH_VENDOR_ID || !is_known_receiver(info.product_id()) {
            continue;
        }

        let is_short = info.usage_page() == SHORT_REPORT_USAGE_PAGE
            && info.usage() == SHORT_REPORT_USAGE;
        let is_long =
            info.usage_page() == LONG_REPORT_USAGE_PAGE && info.usage() == LONG_REPORT_USAGE;
        if !is_short && !is_long {
            continue;
        }

        let path = info.path().to_owned();
        let entry = found
            .entry(interface_key(&path.to_string_lossy()))
            .or_insert_with(|| ReceiverInterface {
                vendor_id: info.vendor_id(),
                product_id: info.product_id(),
                short_path: None,
                long_path: None,
            });

        if is_short {
            entry.short_path = Some(path);
        } else {
            entry.long_path = Some(path);
        }
    }

    found.into_values().collect()
}

/// Opens a receiver interface and detects which kind of receiver it is.
pub async fn open_receiver(
    api: &HidApi,
    iface: &ReceiverInterface,
    role: ChannelRole,
) -> Result<(Arc<HidppChannel>, Receiver), OpenError> {
    let raw = HidappChannel::open(
        api,
        iface.vendor_id,
        iface.product_id,
        iface.short_path.as_deref(),
        iface.long_path.as_deref(),
    )?;
    let chan = Arc::new(HidppChannel::from_raw_channel(raw).await?);
    prepare_channel(&chan, role);

    let receiver = receiver::detect(Arc::clone(&chan)).ok_or(OpenError::UnknownReceiver)?;
    Ok((chan, receiver))
}

/// Who uses a channel. Windows hands every input report to every open handle
/// of a device, so channels opened at the same time must not reuse each
/// other's software IDs, or one picks up the other's responses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelRole {
    /// The long-lived remapping engine.
    Engine,
    /// Short-lived channels for listing, pairing and the CLI.
    Shared,
}

/// The software ID reserved for the engine; other channels rotate below it.
const ENGINE_SW_ID: u8 = 0x0f;

fn prepare_channel(chan: &HidppChannel, role: ChannelRole) {
    match role {
        // The engine sends one request at a time, so a fixed ID is enough.
        ChannelRole::Engine => {
            chan.set_rotating_sw_id(false);
            chan.set_sw_id(U4::from_lo(ENGINE_SW_ID));
        },
        ChannelRole::Shared => {
            chan.set_max_rotating_sw_id(U4::from_lo(ENGINE_SW_ID - 1));
            chan.set_rotating_sw_id(true);
        },
    }
}

fn is_known_receiver(product_id: u16) -> bool {
    let pair = (LOGITECH_VENDOR_ID, product_id);
    receiver::bolt::VPID_PAIRS.contains(&pair) || receiver::unifying::VPID_PAIRS.contains(&pair)
}

/// On Windows every top-level collection of an interface gets its own path:
///
/// ```text
/// \\?\hid#vid_046d&pid_c548&mi_02&col01#7&11cf588c&0&0000#{guid}
/// \\?\hid#vid_046d&pid_c548&mi_02&col02#7&11cf588c&0&0001#{guid}
/// ```
///
/// The collection shows up twice: as `&colNN` in the hardware id and as the
/// last `&NNNN` of the instance id. Dropping both groups the short and long
/// HID++ collections of one receiver together. On other platforms one path
/// already covers the whole interface and is used as is.
pub fn interface_key(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('#').collect();
    if parts.len() < 3 || !parts[1].contains("&col") {
        return lower;
    }

    let hardware_id = parts[1].split("&col").next().unwrap_or(parts[1]);
    let instance_id = parts[2].rsplit_once('&').map_or(parts[2], |(head, _)| head);
    format!("{hardware_id}#{instance_id}")
}

/// Errors from opening a receiver.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("could not open the HID device: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("could not establish HID++ communication: {0}")]
    Channel(#[from] hidpp::channel::ChannelError),

    #[error("the device is not a supported receiver")]
    UnknownReceiver,
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        error::Error,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use hidpp::{
        async_trait,
        channel::{HidppChannel, RawHidChannel},
        nibble::U4,
        protocol::v20,
    };

    use super::{ChannelRole, interface_key, prepare_channel};

    /// Two handles on one device: every response goes to both of them, like
    /// Windows delivers input reports to every open handle. Only requests
    /// written through `answering` get a response.
    struct Bus {
        queues: Mutex<[VecDeque<Vec<u8>>; 2]>,
        answering: usize,
    }

    struct Handle {
        bus: Arc<Bus>,
        index: usize,
    }

    #[async_trait]
    impl RawHidChannel for Handle {
        fn vendor_id(&self) -> u16 {
            0x046d
        }

        fn product_id(&self) -> u16 {
            0xc548
        }

        async fn write_report(&self, src: &[u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            if self.index == self.bus.answering {
                let mut reply = src.to_vec();
                reply[4] = 0xbb;
                for queue in self.bus.queues.lock().unwrap().iter_mut() {
                    queue.push_back(reply.clone());
                }
            }
            Ok(src.len())
        }

        async fn read_report(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            let report = self.bus.queues.lock().unwrap()[self.index].pop_front();
            match report {
                Some(report) => {
                    buf[..report.len()].copy_from_slice(&report);
                    Ok(report.len())
                },
                None => {
                    std::thread::sleep(Duration::from_millis(10));
                    Ok(0)
                },
            }
        }

        fn supports_short_long_hidpp(&self) -> Option<(bool, bool)> {
            Some((true, false))
        }

        async fn get_report_descriptor(
            &self,
            _buf: &mut [u8],
        ) -> Result<usize, Box<dyn Error + Sync + Send>> {
            unreachable!()
        }
    }

    fn request(chan: &HidppChannel) -> v20::Message {
        v20::Message::Short(
            v20::MessageHeader {
                device_index: 0x02,
                feature_index: 0x08,
                function_id: U4::from_lo(0x1),
                software_id: chan.get_sw_id(),
            },
            [0x00, 0x00, 0x00],
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn engine_does_not_take_responses_meant_for_another_channel() {
        let bus = Arc::new(Bus {
            queues: Mutex::default(),
            answering: 1,
        });
        let engine = Arc::new(
            HidppChannel::from_raw_channel(Handle {
                bus: Arc::clone(&bus),
                index: 0,
            })
            .await
            .unwrap(),
        );
        let ui = HidppChannel::from_raw_channel(Handle {
            bus: Arc::clone(&bus),
            index: 1,
        })
        .await
        .unwrap();
        prepare_channel(&engine, ChannelRole::Engine);
        prepare_channel(&ui, ChannelRole::Shared);

        // The engine asks first and gets no answer of its own...
        let engine_request = tokio::spawn({
            let engine = Arc::clone(&engine);
            async move {
                let msg = request(&engine);
                tokio::time::timeout(Duration::from_millis(500), engine.send_v20(msg)).await
            }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        // ...while the UI sends the same request and is answered.
        ui.send_v20(request(&ui)).await.unwrap();

        assert!(
            engine_request.await.unwrap().is_err(),
            "the engine received the response to the UI's request"
        );
    }

    #[test]
    fn groups_windows_collections() {
        let short = r"\\?\hid#vid_046d&pid_c548&mi_02&col01#7&11cf588c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        let long = r"\\?\hid#vid_046d&pid_c548&mi_02&col02#7&11cf588c&0&0001#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert_eq!(interface_key(short), interface_key(long));
        assert_eq!(interface_key(short), "vid_046d&pid_c548&mi_02#7&11cf588c&0");
    }

    #[test]
    fn separates_different_receivers() {
        let a = r"\\?\hid#vid_046d&pid_c548&mi_02&col01#7&11cf588c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        let b = r"\\?\hid#vid_046d&pid_c548&mi_02&col01#7&2b9d1e07&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert_ne!(interface_key(a), interface_key(b));
    }

    #[test]
    fn keeps_paths_without_collections() {
        assert_eq!(interface_key("/dev/hidraw3"), "/dev/hidraw3");
    }
}
