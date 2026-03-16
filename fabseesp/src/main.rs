#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{init, timer::timg::TimerGroup};
use esp_println as _;
use esp_rtos::start;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run() {
    loop {
        info!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = init(esp_hal::Config::default());

    info!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    start(timg0.timer0);

    spawner.spawn(run()).ok();

    loop {
        info!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
