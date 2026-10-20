//! Finds Logitech receivers and opens HID++ channels to them.

use std::{collections::BTreeMap, ffi::CString, sync::Arc};

use hidapi::HidApi;
use hidpp::{
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

/// Lists all connected receivers that boltkit knows how to talk to.
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
) -> Result<(Arc<HidppChannel>, Receiver), OpenError> {
    let raw = HidappChannel::open(
        api,
        iface.vendor_id,
        iface.product_id,
        iface.short_path.as_deref(),
        iface.long_path.as_deref(),
    )?;
    let chan = Arc::new(HidppChannel::from_raw_channel(raw).await?);
    chan.set_rotating_sw_id(true);

    let receiver = receiver::detect(Arc::clone(&chan)).ok_or(OpenError::UnknownReceiver)?;
    Ok((chan, receiver))
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
fn interface_key(path: &str) -> String {
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
    use super::interface_key;

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
