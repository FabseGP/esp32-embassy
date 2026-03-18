use defmt::info;
use embassy_executor::{Spawner, task};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    gpio::{Input, InputConfig, Pull},
    peripherals::{ADC1, GPIO5, GPIO6, GPIO7},
};
use nb::block;

#[derive(Clone, Copy)]
pub struct JoystickData {
    pub x: u16,
    pub y: u16,
}

pub static JOYSTICK_DATA: Mutex<CriticalSectionRawMutex, JoystickData> =
    Mutex::new(JoystickData { x: 0, y: 0 });

pub fn setup_joystick(
    vrx_pin: GPIO5<'static>,
    vry_pin: GPIO6<'static>,
    adc: ADC1<'static>,
    button_pin: GPIO7<'static>,
    spawner: Spawner,
) {
    let mut adc2_config = AdcConfig::new();
    let vrx = adc2_config.enable_pin(vrx_pin, Attenuation::_11dB);
    let vry = adc2_config.enable_pin(vry_pin, Attenuation::_11dB);

    let adc2 = Adc::new(adc, adc2_config);

    let btn = Input::new(button_pin, InputConfig::default().with_pull(Pull::Up));

    spawner.spawn(joystick_task(vrx, vry, adc2, btn)).ok();
}

#[task]
async fn joystick_task(
    mut vrx: AdcPin<GPIO5<'static>, ADC1<'static>>,
    mut vry: AdcPin<GPIO6<'static>, ADC1<'static>>,
    mut adc2: Adc<'static, ADC1<'static>, Blocking>,
    btn: Input<'static>,
) {
    let mut prev_vrx: u16 = 0;
    let mut prev_vry: u16 = 0;
    let mut prev_btn_state = false;
    let mut print_vals = true;

    loop {
        let Ok(vry_value): Result<u16, _> = block!(adc2.read_oneshot(&mut vry)) else {
            continue;
        };
        let Ok(vrx_value): Result<u16, _> = block!(adc2.read_oneshot(&mut vrx)) else {
            continue;
        };

        if vrx_value.abs_diff(prev_vrx) > 100 {
            prev_vrx = vrx_value;
            print_vals = true;
        }

        if vry_value.abs_diff(prev_vry) > 100 {
            prev_vry = vry_value;
            print_vals = true;
        }

        let btn_state = btn.is_low();
        if btn_state && !prev_btn_state {
            info!("Button Pressed");
            print_vals = true;
        }
        prev_btn_state = btn_state;

        if print_vals {
            print_vals = false;
            let mut data = JOYSTICK_DATA.lock().await;
            *data = JoystickData {
                x: vrx_value,
                y: vry_value,
            };
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}
