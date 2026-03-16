use defmt::info;
use embassy_executor::{Spawner, task};
use embassy_net::{Config as NetConfig, DhcpConfig, Runner, Stack, StackResources, new as new_net};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_println as _;
use esp_radio::init;
use esp_radio::wifi::Config;
use esp_radio::{
    Controller,
    wifi::{
        ClientConfig, ModeConfig, ScanConfig, WifiController, WifiDevice, WifiEvent, WifiStaState,
        new as new_wifi, sta_state,
    },
};
use static_cell::StaticCell;

static RADIO_CELL: StaticCell<Controller<'static>> = StaticCell::new();
static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

const SSID: &str = "WiFimodem-0CCC-2GHz";
const PASSWORD: &str = "VAM21K48";

pub async fn start_wifi(wifi: WIFI<'static>, rng: Rng, spawner: &Spawner) -> Stack<'static> {
    let radio_init = RADIO_CELL.init(init().expect("Failed to initialize Wi-Fi/BLE controller"));

    let (wifi_controller, interfaces) = new_wifi(radio_init, wifi, Config::default())
        .expect("Failed to initialize Wi-Fi controller");
    let wifi_interface = interfaces.sta;

    let net_seed = u64::from(rng.random()) | ((u64::from(rng.random())) << 32);

    let config = NetConfig::dhcpv4(DhcpConfig::default());
    let resources = STACK_RESOURCES.init(StackResources::new());
    let (stack, runner) = new_net(wifi_interface, config, resources, net_seed);

    spawner.spawn(connection(wifi_controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    wait_for_connection(stack).await;

    stack
}

async fn wait_for_connection(stack: Stack<'_>) {
    info!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[task]
pub async fn connection(mut controller: WifiController<'static>) {
    info!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if sta_state() == WifiStaState::Connected {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await;
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            controller.start_async().await.unwrap();

            let scan_config = ScanConfig::default().with_max(10);
            let result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();
            for ap in result {
                info!("{:?}", ap);
            }
        }

        match controller.connect_async().await {
            Ok(()) => info!("Wifi connected!"),
            Err(err) => {
                info!("Failed to connect to wifi: {:?}", err);
                Timer::after(Duration::from_millis(5000)).await;
            }
        }
    }
}

#[task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}
