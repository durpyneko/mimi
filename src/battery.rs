use crate::devices::ConnectedDevice;
use std::time::Duration;

pub fn read_battery(connected: &ConnectedDevice) -> Option<u8> {
    let api = match hidapi::HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            log::error!("HidApi::new failed: {e}");
            return None;
        }
    };

    log::info!(
        "Opening {} at {:?}",
        connected.device.display_name(),
        connected.path
    );
    let device_handle = match api.open(connected.device.vendor_id(), connected.device.product_id())
    {
        Ok(d) => d,
        Err(e) => {
            log::error!("open failed: {e}");
            return None;
        }
    };

    let request = connected.device.battery_request();

    match device_handle.write(&request) {
        Ok(n) => log::debug!("Wrote {n} bytes"),
        Err(e) => {
            log::error!("write failed: {e}");
            return None;
        }
    }

    // found that you need to wait for the device to respond
    // or something like that, as i have gotten null results
    // when not doing this so
    std::thread::sleep(Duration::from_millis(100));

    let mut buf = [0u8; 64];
    for i in 0..50 {
        match device_handle.read_timeout(&mut buf, 10) {
            Ok(n) if n > 0 => {
                log::debug!("iter {i}: got {n} bytes: {:02x?}", &buf[..n.min(8)]);
                if buf[0] == 0x66 && buf[1] == 0x89 {
                    return Some(buf[4]);
                }
            }
            Ok(_) => {}
            Err(e) => log::warn!("read_timeout error at iter {i}: {e}"),
        }
    }

    log::warn!(
        "No matching response from {} after 50 reads",
        connected.device.display_name()
    );
    None
}

pub fn battery_bar(level: u8) -> String {
    let w = 12;
    let filled = (level as usize * w / 100).min(w);
    let empty = w - filled;
    format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), level)
}
