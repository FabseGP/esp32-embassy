use alloc::borrow::ToOwned;
use defmt::{error, info};
use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
use embassy_net::{
    Config as NetConfig, DhcpConfig, Runner, Stack, StackResources,
    dns::DnsSocket,
    new as new_net,
    tcp::{
        Error as TCPError, TcpSocket,
        client::{TcpClient, TcpClientState},
    },
};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{peripherals::WIFI, rng::Rng};
use esp_radio::wifi::{
    Config, ControllerConfig, Interface, WifiController, ap::EventInfo, sta::StationConfig,
};
use reqwless::{
    client::HttpClient,
    request::{Method, RequestBuilder},
};
use static_cell::StaticCell;

static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

pub async fn start_wifi(wifi: WIFI<'static>, rng: Rng, spawner: &Spawner) -> Stack<'static> {
    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(env!("WIFI_SSID"))
            .with_password(env!("WIFI_PASS").to_owned()),
    );

    let wifi_controller = WifiController::new(
        wifi,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");
    let wifi_interface = Interface::station();

    let net_seed = u64::from(rng.random()) | ((u64::from(rng.random())) << 32);

    let config = NetConfig::dhcpv4(DhcpConfig::default());
    let resources = STACK_RESOURCES.init(StackResources::new());
    let (stack, runner) = new_net(wifi_interface, config, resources, net_seed);

    spawner.spawn(connection(wifi_controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    wait_for_connection(stack).await;

    stack
}

async fn wait_for_connection(stack: Stack<'_>) {
    info!("Waiting for link to be up");
    stack.wait_link_up().await;

    info!("Waiting to get IP address...");
    stack.wait_config_up().await;
    info!("Got IP: {}", stack.config_v4().unwrap().address);
}

#[task]
async fn connection(mut controller: WifiController<'static>) {
    info!("start connection task");

    loop {
        match controller.connect_async().await {
            Ok(_) => loop {
                let info = select(
                    controller.wait_for_disconnect_async(),
                    controller.wait_for_access_point_connected_event_async(),
                )
                .await;

                match info {
                    Either::First(station_disconnected) => {
                        if let Ok(station_disconnected) = station_disconnected {
                            info!("Station disconnected: {:?}", station_disconnected);
                            break;
                        }
                    }
                    Either::Second(event) => {
                        if let Ok(event) = event {
                            match event {
                                EventInfo::Connected(access_point_station_connected_info) => {
                                    info!(
                                        "Station connected: {:?}",
                                        access_point_station_connected_info
                                    );
                                }
                                EventInfo::Disconnected(access_point_station_disconnected_info) => {
                                    info!(
                                        "Station disconnected: {:?}",
                                        access_point_station_disconnected_info
                                    );
                                }
                            }
                        }
                    }
                }
            },
            Err(err) => {
                info!("Failed to connect to wifi: {:?}", err);
                Timer::after_secs(5).await;
            }
        }
    }
}

#[task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await;
}

async fn access_website(stack: Stack<'_>) {
    let dns = DnsSocket::new(stack);
    let tcp_state = TcpClientState::<1, 4096, 4096>::new();
    let tcp = TcpClient::new(stack, &tcp_state);

    let mut client = HttpClient::new(&tcp, &dns);
    let mut buffer = [0u8; 4096];
    let http = match client
        .request(Method::POST, "https://jsonplaceholder.typicode.com/posts/1")
        .await
    {
        Ok(request) => request,
        Err(err) => {
            error!("Failed to create request: {}", err);
            return;
        }
    };
    if let Err(err) = http.basic_auth("test", "test").send(&mut buffer).await {
        error!("Failed to send request: {}", err);
    }
}

pub async fn read_exact(socket: &mut TcpSocket<'_>, mut buf: &mut [u8]) -> Result<(), TCPError> {
    while !buf.is_empty() {
        match socket.read(buf).await {
            Ok(0) => return Err(TCPError::ConnectionReset),
            Ok(n) => {
                buf = &mut buf[n..];
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
