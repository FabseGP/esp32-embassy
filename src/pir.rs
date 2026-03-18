use defmt::info;
use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals::GPIO3,
};

pub fn setup_pir(input_pin: GPIO3<'static>, spawner: Spawner) {
    let sensor_pin = Input::new(input_pin, InputConfig::default().with_pull(Pull::Down));

    spawner.spawn(pir_task(sensor_pin)).ok();
}

#[task]
async fn pir_task(sensor_pin: Input<'static>) {
    loop {
        if sensor_pin.is_high() {
            info!("Motion detected");
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}
