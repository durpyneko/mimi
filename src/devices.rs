use std::path::PathBuf;

pub mod vid {
    pub const HP: u16 = 0x03f0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedDevice {
    /// HP, Inc. - HyperX Cloud III Wireless
    HyperXCloud3W,
}

impl SupportedDevice {
    pub const ALL: &'static [Self] = &[Self::HyperXCloud3W];

    pub const fn vendor_id(self) -> u16 {
        match self {
            Self::HyperXCloud3W => vid::HP,
        }
    }

    pub const fn product_id(self) -> u16 {
        match self {
            Self::HyperXCloud3W => 0x05b7,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::HyperXCloud3W => "HyperX Cloud III Wireless",
        }
    }

    /// Match a (vendor_id, product_id) pair to a known device.
    pub fn from_ids(vendor_id: u16, product_id: u16) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|d| d.vendor_id() == vendor_id && d.product_id() == product_id)
    }

    pub fn battery_request(self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        match self {
            Self::HyperXCloud3W => {
                buf[0] = 0x66; // battery?
                buf[1] = 0x89; // level?
            }
        }
        buf
    }
}

/// A supported device that's actually plugged in right now.
#[derive(Debug)]
pub struct ConnectedDevice {
    pub device: SupportedDevice,
    pub path: PathBuf, // e.g. /dev/hidraw4
}

pub fn connected_devices() -> Vec<ConnectedDevice> {
    let api = match hidapi::HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            log::error!("hidapi init failed: {e}");
            return vec![];
        }
    };

    api.device_list()
        .filter_map(|info| {
            SupportedDevice::from_ids(info.vendor_id(), info.product_id()).map(|device| {
                ConnectedDevice {
                    device,
                    path: PathBuf::from(info.path().to_string_lossy().as_ref()),
                }
            })
        })
        .collect()
}

pub fn first_connected() -> Option<ConnectedDevice> {
    connected_devices().into_iter().next()
}
