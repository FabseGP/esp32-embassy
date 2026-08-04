use embassy_executor::{Spawner, task};
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer as AsyncTimer;
use embedded_hal::pwm::SetDutyCycle;
use esp_hal::{
    gpio::DriveMode,
    ledc::{
        HighSpeed, Ledc,
        channel::{
            ChannelIFace,
            Number::{Channel0, Channel1},
            config::Config as ChannelConfig,
        },
        timer::{
            HSClockSource::APBClk,
            Number::Timer0,
            Timer, TimerIFace,
            config::{Config as TimerConfig, Duty::Duty12Bit},
        },
    },
    peripherals::{GPIO32, GPIO33, LEDC},
    time::Rate,
};
use picoserve::{extract::Json, response::IntoResponse};
use serde::Deserialize;

use crate::server::SuccessResponse;

#[derive(Deserialize)]
pub struct MotorsRequest {
    value: u32,
}

pub static MOTOR_STATE: Signal<CriticalSectionRawMutex, u32> = Signal::new();

pub async fn motor_handler(input: Json<MotorsRequest>) -> impl IntoResponse {
    MOTOR_STATE.signal(input.0.value);

    Json(SuccessResponse { success: true })
}

pub static SERVO_STATE: Signal<CriticalSectionRawMutex, u32> = Signal::new();

pub async fn servo_handler(input: Json<MotorsRequest>) -> impl IntoResponse {
    SERVO_STATE.signal(input.0.value);

    Json(SuccessResponse { success: true })
}

pub fn setup_motors(
    ledc_pin: LEDC<'static>,
    motor: GPIO32<'static>,
    servo: GPIO33<'static>,
    spawner: Spawner,
) {
    let ledc = Ledc::new(ledc_pin);
    let mut hstimer0 = ledc.timer::<HighSpeed>(Timer0);
    hstimer0
        .configure(TimerConfig {
            duty: Duty12Bit,
            clock_source: APBClk,
            frequency: Rate::from_hz(50),
        })
        .unwrap();

    spawner.spawn(motors_task(ledc, hstimer0, motor, servo).unwrap());
}

#[task]
async fn motors_task(
    ledc: Ledc<'static>,
    hstimer: Timer<'static, HighSpeed>,
    motor: GPIO32<'static>,
    servo: GPIO33<'static>,
) {
    let mut channel0 = ledc.channel(Channel0, servo);
    channel0
        .configure(ChannelConfig {
            timer: &hstimer,
            duty_pct: 10,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    let mut channel1 = ledc.channel(Channel1, motor);
    channel1
        .configure(ChannelConfig {
            timer: &hstimer,
            duty_pct: 10,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    let channel0_max = channel0.max_duty_cycle() as u32;
    let channel1_max = channel1.max_duty_cycle() as u32;

    let servo_min = (800 * channel0_max) / 20000;
    let servo_max = (2200 * channel0_max) / 20000;
    let servo_gap = servo_max - servo_min;

    let servo_duty = duty_from_angle(90, servo_min, servo_gap);
    channel0.set_duty_cycle(servo_duty).unwrap();

    let motor_min = (1000 * channel1_max) / 20000;
    let motor_max = (2000 * channel1_max) / 20000;
    let neutral = (1500 * channel1_max) / 20000;

    channel1.set_duty_cycle(motor_min as u16).unwrap();
    AsyncTimer::after_millis(2000).await;
    channel1.set_duty_cycle(neutral as u16).unwrap();
    AsyncTimer::after_millis(2000).await;
    channel1.set_duty_cycle(150).unwrap();

    loop {
        match select(SERVO_STATE.wait(), MOTOR_STATE.wait()).await {
            Either::First(servo_value) => {
                let duty = duty_from_angle(servo_value, servo_min, servo_gap);
                channel0.set_duty_cycle(duty).unwrap();
            }
            Either::Second(motor_value) => {
                //  let duty = motor_min + (motor_value * (motor_max - motor_min) / 10);
                channel1.set_duty_cycle(motor_value as u16).unwrap();
            }
        }
    }
}

fn duty_from_angle(deg: u32, min_duty: u32, duty_gap: u32) -> u16 {
    let duty = min_duty + ((deg * duty_gap) / 180);
    duty as u16
}
