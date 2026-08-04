use esp_hal::peripherals::FLASH;
use esp_hal_ota::Ota;
use esp_storage::FlashStorage;

pub fn setup_ota(flash: FLASH<'static>) {
    let ota_flash = FlashStorage::new(flash);
    let mut ota = Ota::new(ota_flash).unwrap();
    ota.ota_mark_app_valid().unwrap();
}
