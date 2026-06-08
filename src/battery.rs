use std::time::Duration;

pub fn read_battery() -> Option<u8> {
    let api = match hidapi::HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            log::error!("HidApi::new failed: {e}");
            return None;
        }
    };

    let info = api
        .device_list()
        .find(|d| d.vendor_id() == crate::VENDOR_ID && d.product_id() == crate::PRODUCT_ID);

    let info = match info {
        Some(d) => d,
        None => {
            log::warn!(
                "Device {:04x}:{:04x} not found in enumeration",
                crate::VENDOR_ID,
                crate::PRODUCT_ID
            );
            return None;
        }
    };

    log::debug!("Found device at {:?}", info.path());

    let device = match info.open_device(&api) {
        Ok(d) => d,
        Err(e) => {
            log::error!("open_device failed: {e}");
            return None;
        }
    };

    let mut request = [0u8; 64];
    request[0] = 0x66;
    request[1] = 0x89;

    match device.write(&request) {
        Ok(n) => log::debug!("Wrote {n} bytes"),
        Err(e) => {
            log::error!("write failed: {e}");
            return None;
        }
    }

    std::thread::sleep(Duration::from_millis(100));

    let mut buf = [0u8; 64];
    for i in 0..50 {
        match device.read_timeout(&mut buf, 10) {
            Ok(n) if n > 0 => {
                log::debug!("iter {i}: got {n} bytes: {:02x?}", &buf[..n.min(8)]);
                if buf[0] == 0x66 && buf[1] == 0x89 {
                    return Some(buf[4]);
                }
            }
            Ok(_) => {} // 0 bytes, keep trying
            Err(e) => log::warn!("read_timeout error at iter {i}: {e}"),
        }
    }

    log::warn!("No matching response after 50 reads");
    None
}

pub fn battery_bar(level: u8) -> String {
    let filled = (level as usize * 8 / 100).min(8);
    let empty = 8 - filled;
    format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), level)
}
