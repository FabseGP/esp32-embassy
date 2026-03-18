use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::GPIO2,
};
use picoserve::{extract::Json, response::IntoResponse};

pub static LED_STATE: AtomicBool = AtomicBool::new(false);

#[derive(serde::Deserialize)]
pub struct LedRequest {
    is_on: bool,
}

#[derive(serde::Serialize)]
struct LedResponse {
    success: bool,
}

pub async fn led_handler(input: Json<LedRequest>) -> impl IntoResponse {
    LED_STATE.store(input.0.is_on, Ordering::Relaxed);

    Json(LedResponse { success: true })
}

pub fn setup_led(led_pin: GPIO2<'static>, spawner: Spawner) {
    let led = Output::new(led_pin, Level::Low, OutputConfig::default());
    spawner.must_spawn(led_task(led));
}

#[task]
async fn led_task(mut led: Output<'static>) {
    loop {
        if LED_STATE.load(Ordering::Relaxed) {
            led.set_high();
        } else {
            led.set_low();
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}
