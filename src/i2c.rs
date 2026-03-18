use defmt::error;
use esp_hal::{
    Async,
    i2c::master::{Config, I2c},
    peripherals::{GPIO13, GPIO15, I2C0},
    time::Rate,
};

pub fn setup_i2c(
    i2c: I2C0<'static>,
    scl: GPIO15<'static>,
    sda: GPIO13<'static>,
) -> I2c<'static, Async> {
    match I2c::new(i2c, Config::default().with_frequency(Rate::from_khz(400))) {
        Ok(bus) => bus.with_scl(scl).with_sda(sda).into_async(),
        Err(err) => {
            error!("Failed to initialize I2c-bus: {}", err);
            panic!();
        }
    }
}
