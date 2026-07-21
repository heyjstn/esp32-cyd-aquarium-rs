use esp32_nimble::{uuid128, BLEAdvertisementData, BLEDevice, BLEScan, NimbleProperties};
use esp_idf_hal::modem::BluetoothModem;

const DEVICE_NAME: &str = "Angstrom Monitor";

pub fn create_bluetooth_conn(_bluetooth_modem: BluetoothModem<'static>) -> anyhow::Result<()> {
    // let ble_device = BLEDevice::take();
    //
    // BLEDevice::set_device_name(DEVICE_NAME)?;
    //
    // let server = ble_device.get_server();
    //
    // server.on_connect(|_server, connection| {
    //     log::info!(
    //         "BLE client connected: address={}",
    //         connection.address()
    //     );
    // });
    //
    // server.on_disconnect(|connection, reason| {
    //     log::info!(
    //         "BLE client disconnected: address={} reason={reason:?}",
    //         connection.address()
    //     );
    // });
    //
    // let service_uuid = uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa");
    // let service = server.create_service(service_uuid);
    //
    // let characteristic = service.lock().create_characteristic(
    //     uuid128!("d4e0e0d0-1a2b-11e9-ab14-d663bd873d93"),
    //     NimbleProperties::READ,
    // );
    //
    // characteristic
    //     .lock()
    //     .set_value(b"Hello from CYD Aquarium");
    //
    // let advertising = ble_device.get_advertising();
    //
    // advertising.lock().set_data(
    //     BLEAdvertisementData::new()
    //         .name(DEVICE_NAME)
    //         .add_service_uuid(service_uuid),
    // )?;
    //
    // advertising.lock().start()?;
    //
    // log::info!("BLE advertising started: name={DEVICE_NAME}");
    //
    Ok(())
}
