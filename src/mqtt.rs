use defmt::info;
use embassy_executor::task;
use embassy_net::Stack;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver},
};
use smoltcp::wire::DnsQueryType;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SensorSource {
    Temperature,
    Moisture,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Unit {
    Celsius,
    Percentage,
    Raw,
}

#[derive(Copy, Clone)]
pub struct SensorReading {
    pub source: SensorSource,
    pub unit: Unit,
    pub value: i32,
}

pub type TelemetryReceiver<'a> = Receiver<'a, CriticalSectionRawMutex, SensorReading, 8>;

pub static TELEMETRY_CHANNEL: Channel<CriticalSectionRawMutex, SensorReading, 8> = Channel::new();

#[task]
pub async fn mqtt_task(stack: Stack<'static>, receiver: TelemetryReceiver<'static>) {
    info!("MQTT task: preparing discovery payload");
    let query_res = stack
        .dns_query("http://192.168.0.249", DnsQueryType::A)
        .await;
    if let Ok(addrs) = query_res {
        info!("weeee");
    }
}
