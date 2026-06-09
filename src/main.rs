//
//               ,--.          ,--.
//     ,--,--,--.`--',--,--,--.`--'
//    |        |,--.|        |,--.
//   |  |  |  ||  ||  |  |  ||  |
//   `--`--`--'`--'`--`--`--'`--'
//                                      HyperX tray icon

mod battery;
mod icon;

use image::ImageFormat;
use ksni::TrayMethods;
use std::time::Duration;
use tokio::sync::mpsc;

const VENDOR_ID: u16 = 0x03f0; // HP / HyperX
const PRODUCT_ID: u16 = 0x05b7; // Cloud III Wireless

#[derive(Debug)]
struct MimiTrayState {
    tray: MimiTray,
    battery: Option<u8>,
    refresh_tx: mpsc::UnboundedSender<()>,
}

#[derive(Debug)]
struct MimiTray {
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
            title: "Mimi - HyperX Tray".into(),
            ..Default::default()
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: self.tray.width,
            height: self.tray.height,
            data: icon::render_icon(self.battery),
        }]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let battery_label = match self.battery {
            Some(level) => battery::battery_bar(level),
            None => "Battery: disconnected".into(),
        };

        vec![
            StandardItem {
                label: "Mimi — HyperX Tray".into(),
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

    let initial_battery = tokio::task::spawn_blocking(battery::read_battery)
        .await
        .unwrap_or(None);

    let (refresh_tx, mut refresh_rx) = mpsc::unbounded_channel::<()>();

    let handle = MimiTrayState {
        tray: MimiTray {
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

            let level = tokio::task::spawn_blocking(battery::read_battery)
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
