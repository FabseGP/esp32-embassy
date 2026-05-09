use defmt::info;
use embassy_executor::{Spawner, task};
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    peripherals::GPIO3,
};

pub fn setup_pir(input_pin: GPIO3<'static>, spawner: Spawner) {
    let sensor_pin = Input::new(input_pin, InputConfig::default().with_pull(Pull::Down));

    spawner.spawn(pir_task(sensor_pin).unwrap());
}

#[task]
async fn pir_task(mut sensor_pin: Input<'static>) {
    loop {
        sensor_pin.wait_for_high().await;
        info!("Motion detected");
    }
}
