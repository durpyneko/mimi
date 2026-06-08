//
//               ,--.          ,--.
//     ,--,--,--.`--',--,--,--.`--'
//    |        |,--.|        |,--.
//   |  |  |  ||  ||  |  |  ||  |
//   `--`--`--'`--'`--`--`--'`--'
//                                      HyperX tray icon

use image::ImageFormat;
use ksni::TrayMethods;
use std::time::Duration;
use tokio::sync::mpsc;

const VENDOR_ID: u16 = 0x03f0; // HP / HyperX
const PRODUCT_ID: u16 = 0x05b7; // Cloud III Wireless

fn read_battery() -> Option<u8> {
    let api = match hidapi::HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            log::error!("HidApi::new failed: {e}");
            return None;
        }
    };

    let info = api
        .device_list()
        .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID);

    let info = match info {
        Some(d) => d,
        None => {
            log::warn!(
                "Device {:04x}:{:04x} not found in enumeration",
                VENDOR_ID,
                PRODUCT_ID
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

fn battery_bar(level: u8) -> String {
    let filled = (level as usize * 8 / 100).min(8);
    let empty = 8 - filled;
    format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), level)
}

#[derive(Debug)]
struct MimiTrayState {
    tray: MimiTray,
    battery: Option<u8>,
    refresh_tx: mpsc::UnboundedSender<()>,
}

#[derive(Debug)]
struct MimiTray {
    icon_data: Vec<u8>,
    width: i32,
    height: i32,
}

impl ksni::Tray for MimiTrayState {
    fn id(&self) -> String {
        "mimi".into()
    }

    fn title(&self) -> String {
        "Mimi".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "nya!".into(),
            ..Default::default()
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: self.tray.width,
            height: self.tray.height,
            data: self.tray.icon_data.clone(),
        }]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let battery_label = match self.battery {
            // todo if zero displaythis (it's 1 am on a work morning i need to go to bed)
            Some(level) => battery_bar(level),
            None => "Battery: disconnected".into(),
        };

        vec![
            StandardItem {
                label: "Mimi — (˵◝ ⩊ ◜˵マ".into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: battery_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Refresh".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.refresh_tx.send(());
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[tokio::main]
async fn main() {
    nyaaan::init().unwrap();
    if cfg!(debug_assertions) {
        nyaaan::set_level(log::Level::Debug)
    } else {
        nyaaan::set_level(log::Level::Info)
    };

    let image = image::load_from_memory_with_format(
        include_bytes!("../assets/mimi-catgirl.png"),
        ImageFormat::Png,
    )
    .expect("Failed to load image")
    .resize_exact(32, 32, image::imageops::FilterType::Triangle)
    .to_rgba8();

    let (width, height) = image.dimensions();

    // SNI requires ARGB32 — image crate gives RGBA, so reorder each pixel
    let icon_data: Vec<u8> = image
        .pixels()
        .flat_map(|p| {
            let [r, g, b, a] = p.0;
            [a, r, g, b]
        })
        .collect();

    let initial_battery = tokio::task::spawn_blocking(read_battery)
        .await
        .unwrap_or(None);

    let (refresh_tx, mut refresh_rx) = mpsc::unbounded_channel::<()>();

    let handle = MimiTrayState {
        tray: MimiTray {
            icon_data,
            width: width as i32,
            height: height as i32,
        },
        battery: initial_battery,
        refresh_tx,
    }
    .spawn()
    .await
    .expect("Failed to register Mimi tray");

    log::info!("Tray registered. Battery: {:?}", initial_battery);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_mins(10));
        interval.tick().await; // consume the immediate first tick

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    log::debug!("Poll: timer")
                },
                _ = refresh_rx.recv() => log::debug!("Poll: manual refresh"),
            }

            let level = tokio::task::spawn_blocking(read_battery)
                .await
                .unwrap_or(None);
            log::debug!("Battery: {:?}", level);

            handle
                .update(|state: &mut MimiTrayState| state.battery = level)
                .await;
        }
    });

    // runtime keepalive
    tokio::signal::ctrl_c().await.unwrap();
}
