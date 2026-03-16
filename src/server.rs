use embassy_executor::task;
use embassy_net::Stack;
use embassy_time::Duration;
use picoserve::{
    AppBuilder, AppRouter, Config, Router, Server, Timeouts, make_static, response::File, routing,
};

pub const WEB_TASK_POOL_SIZE: usize = 2;

pub struct Application;

impl AppBuilder for Application {
    type PathRouter = impl routing::PathRouter;

    fn build_app(self) -> Router<Self::PathRouter> {
        Router::new().route(
            "/",
            routing::get_service(File::html(include_str!("index.html"))),
        )
    }
}

pub struct WebApp {
    pub router: &'static Router<<Application as AppBuilder>::PathRouter>,
    pub config: &'static Config,
}

impl Default for WebApp {
    fn default() -> Self {
        static CONFIG: Config = Config::new(Timeouts {
            start_read_request: Duration::from_secs(5),
            read_request: Duration::from_secs(1),
            write: Duration::from_secs(1),
            persistent_start_read_request: Duration::from_secs(1),
        })
        .keep_connection_alive();

        let router = make_static!(AppRouter<Application>, Application.build_app());

        Self {
            router,
            config: &CONFIG,
        }
    }
}

#[task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(
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
