use embassy_executor::{Spawner, task};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::GPIO2,
};
use picoserve::{extract::Json, response::IntoResponse};
use serde::Deserialize;

use crate::server::SuccessResponse;

pub static LED_STATE: Signal<CriticalSectionRawMutex, bool> = Signal::new();

#[derive(Deserialize)]
pub struct LedRequest {
    is_on: bool,
}

pub async fn led_handler(input: Json<LedRequest>) -> impl IntoResponse {
    LED_STATE.signal(input.0.is_on);

    Json(SuccessResponse { success: true })
}

pub fn setup_led(led_pin: GPIO2<'static>, spawner: Spawner) {
    let led = Output::new(led_pin, Level::Low, OutputConfig::default());
    spawner.spawn(led_task(led).unwrap());
}

#[task]
async fn led_task(mut led: Output<'static>) {
    loop {
        let led_on = LED_STATE.wait().await;
        if led_on {
            led.set_high();
        } else {
            led.set_low();
        }
    }
}
