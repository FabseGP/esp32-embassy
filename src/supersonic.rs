use embassy_executor::{Spawner, task};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{GPIO18, GPIO19},
};

const INTRUSION_DISTANCE: f32 = 10.0;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum IntrusionStates {
    Danger,
    Safe,
}

pub static INTRUSION_STATE: Mutex<CriticalSectionRawMutex, IntrusionStates> =
    Mutex::new(IntrusionStates::Safe);
pub static DISTANCE_DATA: Mutex<CriticalSectionRawMutex, f32> = Mutex::new(0.0);

pub fn setup_ultrasonic(trigger_pin: GPIO18<'static>, echo_pin: GPIO19<'static>, spawner: Spawner) {
    let trig = Output::new(trigger_pin, Level::Low, OutputConfig::default());
    let echo = Input::new(echo_pin, InputConfig::default().with_pull(Pull::Down));

    spawner.spawn(ultrasonic_task(trig, echo)).ok();
}

#[task]
async fn ultrasonic_task(mut trig: Output<'static>, mut echo: Input<'static>) {
    let mut current_state = IntrusionStates::Safe;
    loop {
        trig.set_low();
        Timer::after(Duration::from_micros(2)).await;
        trig.set_high();
        Timer::after(Duration::from_micros(10)).await;
        trig.set_low();

        echo.wait_for_high().await;
        let start = Instant::now().as_micros();
        echo.wait_for_low().await;
        let pulse_width = Instant::now().as_micros() - start;

        let new_distance = (pulse_width as f32 * 0.0343) / 2.0;

        {
            let mut dist = DISTANCE_DATA.lock().await;
            *dist = new_distance;
        }

        let new_state = if new_distance < INTRUSION_DISTANCE
            || (current_state == IntrusionStates::Danger
                && new_distance < (INTRUSION_DISTANCE + 2.0))
        {
            IntrusionStates::Danger
        } else {
            IntrusionStates::Safe
        };

        if new_state != current_state {
            let mut state = INTRUSION_STATE.lock().await;
            *state = new_state;
        }

        current_state = new_state;

        Timer::after(Duration::from_millis(60)).await;
    }
}
