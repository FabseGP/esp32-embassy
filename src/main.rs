#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![feature(stmt_expr_attributes)]
#![feature(cfg_select)]
#![no_std]
#![no_main]

use core::future::pending;

use embassy_executor::Spawner;
use esp_alloc::heap_allocator;
use esp_backtrace as _;
use esp_hal::{
    Config, clock::CpuClock, init, interrupt::software::SoftwareInterruptControl, ram,
    timer::timg::TimerGroup,
};
use esp_println as _;
use esp_rtos::{main, start};

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

mod i2c;
mod led;
mod pir;
mod server;
mod supersonic;
mod wifi;

cfg_select! {
    feature = "esp32s2" => {
        mod joystick;
        mod oled;
        use crate::{
            i2c::setup_i2c, joystick::setup_joystick, oled::setup_display_async,
            supersonic::setup_ultrasonic,
        };
    }
    feature = "esp32" => {
        mod bluetooth;
        use esp_hal::rng::Rng;
        use crate::{bluetooth::start_bluetooth, led::setup_led, server::setup_server, wifi::start_wifi};
    }
    _ => {}
}

#[main]
async fn main(spawner: Spawner) {
    heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    heap_allocator!(size: 36 * 1024);

    let chip_config = Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = init(chip_config);

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    start(timg0.timer0, sw_int.software_interrupt0);

    cfg_select! {
        feature = "esp32s2" => {
            let i2c_bus = setup_i2c(peripherals.I2C0, peripherals.GPIO15, peripherals.GPIO13);
            setup_display_async(i2c_bus, spawner).await;
            setup_ultrasonic(peripherals.GPIO18, peripherals.GPIO19, spawner);
            setup_joystick(
                peripherals.GPIO5,
                peripherals.GPIO6,
                peripherals.ADC1,
                peripherals.GPIO7,
                spawner,
            );
        }
        feature = "esp32" => {
            setup_led(peripherals.GPIO2, spawner);
            let rng = Rng::new();
            let stack = start_wifi(peripherals.WIFI, rng, &spawner).await;
            setup_server(spawner, stack);
            start_bluetooth(peripherals.BT).await;
        }
        _ => {}
    }

    pending::<()>().await;
}
