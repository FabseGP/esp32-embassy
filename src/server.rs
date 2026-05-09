use defmt::info;
use embassy_executor::{Spawner, task};
use embassy_net::Stack;
use embassy_time::Duration;
use picoserve::{
    AppBuilder, AppRouter, Config, Router, Server, Timeouts, make_static, response::File, routing,
};
use serde::Serialize;

use crate::{
    led::led_handler,
    motors::{motor_handler, servo_handler},
};

const WEB_TASK_POOL_SIZE: usize = 1;

#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

struct Application;

impl AppBuilder for Application {
    type PathRouter = impl routing::PathRouter;

    fn build_app(self) -> Router<Self::PathRouter> {
        Router::new()
            .route(
                "/",
                routing::get_service(File::html(include_str!("index.html"))),
            )
            .route("/led", routing::post(led_handler))
            .route("/servo", routing::post(servo_handler))
            .route("/motor", routing::post(motor_handler))
    }
}

struct WebApp {
    pub router: &'static Router<<Application as AppBuilder>::PathRouter>,
    pub config: &'static Config,
}

impl Default for WebApp {
    fn default() -> Self {
        static CONFIG: Config = Config::new(Timeouts {
            start_read_request: Duration::from_secs(5),
            read_request: Duration::from_secs(5),
            write: Duration::from_secs(5),
            persistent_start_read_request: Duration::from_secs(5),
        })
        .keep_connection_alive();

        let router = make_static!(AppRouter<Application>, Application.build_app());

        Self {
            router,
            config: &CONFIG,
        }
    }
}

pub fn setup_server(spawner: Spawner, stack: Stack<'static>) {
    let web_app = WebApp::default();
    for id in 0..WEB_TASK_POOL_SIZE {
        match web_task(id, stack, web_app.router, web_app.config) {
            Ok(web) => spawner.spawn(web),
            Err(err) => {
                info!("oof: {}", err);
            }
        }
    }
}

#[task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    task_id: usize,
    stack: Stack<'static>,
    router: &'static AppRouter<Application>,
    config: &'static Config,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    Server::new(router, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}
