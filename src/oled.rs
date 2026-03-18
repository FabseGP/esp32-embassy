use embassy_executor::{Spawner, task};
use embassy_sync::lazy_lock::LazyLock;
use embassy_time::{Duration, Ticker};
use esp_hal::{Async, i2c::master::I2c};

use heapless::String;
use ssd1306::{I2CDisplayInterface, Ssd1306Async, mode::BufferedGraphicsModeAsync, prelude::*};

use core::fmt::Write;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::{Point, *},
    text::{Baseline, Text},
};
use tinybmp::Bmp;

use crate::{
    joystick::JOYSTICK_DATA,
    supersonic::{DISTANCE_DATA, INTRUSION_STATE, IntrusionStates},
};

pub struct RawImage<'a> {
    pub bytes: &'a [u8],
    pub width: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Copy, Clone)]
pub struct BmpImage<'a> {
    pub bytes: Bmp<'a, BinaryColor>,
    pub x: i32,
    pub y: i32,
}

pub static FERRIS: LazyLock<BmpImage> = LazyLock::new(|| BmpImage {
    bytes: Bmp::from_slice(include_bytes!("../assets/ferris.bmp")).unwrap(),
    x: 32,
    y: 0,
});

type OledDisplay<'a> = Ssd1306Async<
    I2CInterface<I2c<'a, Async>>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

pub async fn setup_display_async(i2c_bus: I2c<'static, Async>, spawner: Spawner) {
    let interface = I2CDisplayInterface::new(i2c_bus);
    let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().await.unwrap();
    display.clear_buffer();
    display.flush().await.unwrap();

    spawner.spawn(display_task(display)).ok();
}

pub async fn draw_image(image: RawImage<'_>, display: &mut OledDisplay<'_>) {
    display.clear_buffer();

    let raw_image = ImageRaw::<BinaryColor>::new(image.bytes, image.width);
    let image = Image::new(&raw_image, Point::new(image.x, image.y));

    image.draw(display).unwrap();
    display.flush().await.unwrap();
}

pub async fn draw_bmp(bmp: BmpImage<'_>, display: &mut OledDisplay<'_>) {
    display.clear_buffer();
    let image = Image::new(&bmp.bytes, Point::new(bmp.x, bmp.y));

    image.draw(display).unwrap();
    display.flush().await.unwrap();
}

pub async fn draw_text(display: &mut OledDisplay<'_>, text: &str) {
    display.clear_buffer();
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline(text, Point::new(0, 16), text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    display.flush().await.unwrap();
}

#[task]
async fn display_task(mut display: OledDisplay<'static>) {
    let mut text_buf: String<32> = String::new();
    let mut ui_timer = Ticker::every(Duration::from_millis(500));
    let mut showing_bmp = false;

    loop {
        ui_timer.next().await;
        let state = *INTRUSION_STATE.lock().await;

        match state {
            IntrusionStates::Danger => {
                if !showing_bmp {
                    draw_bmp(*FERRIS.get(), &mut display).await;
                    showing_bmp = true;
                }
            }
            IntrusionStates::Safe => {
                showing_bmp = false;
                text_buf.clear();
                let distance = *DISTANCE_DATA.lock().await;
                let joystick_data = *JOYSTICK_DATA.lock().await;
                write!(
                    text_buf,
                    "Dist: {:.1}cm\nCoords: {} ; {}",
                    distance, joystick_data.x, joystick_data.y
                )
                .unwrap();
                draw_text(&mut display, &text_buf).await;
            }
        }
    }
}
