#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc::heap_allocator;
use esp_backtrace as _;
use esp_hal::{Config, init, ram, rng::Rng, timer::timg::TimerGroup};
use esp_println as _;
use esp_rtos::{main, start};

use crate::{
    server::{WEB_TASK_POOL_SIZE, WebApp, web_task},
    wifi::start_wifi,
};

esp_bootloader_esp_idf::esp_app_desc!();

mod server;
mod wifi;

#[main]
async fn main(spawner: Spawner) {
    heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    heap_allocator!(size: 36 * 1024);

    let peripherals = init(Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    start(timg0.timer0);

    let rng = Rng::new();

    let stack = start_wifi(peripherals.WIFI, rng, &spawner).await;

    let web_app = WebApp::default();
    for id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(id, stack, web_app.router, web_app.config));
    }

    loop {
        info!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
