//! Wi-Fi + NTP clock sync (background thread) and the local-time clock used
//! by the overlay, mirroring main_cyd.cpp's clock state machine:
//! boot fallback -> Wi-Fi connect -> SNTP -> disconnect -> periodic resync.

use std::ffi::{c_char, c_int, c_long, CString};
use std::sync::{Arc, Mutex};

use log::{info, warn};

use crate::consts;
use crate::text::Civil;

// newlib time functions (64-bit time_t on ESP-IDF 5.x).
extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn tzset();
    fn time(t: *mut i64) -> i64;
    fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
}

#[repr(C)]
#[derive(Default)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    // newlib appends these when built with __TM_GMTOFF/__TM_ZONE; keeping
    // them makes the layout safe either way.
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

/// Shared result slot between the sync thread and the frame loop.
pub type SharedEpoch = Arc<Mutex<Option<i64>>>;

/// Clock with a compile-time fallback base, moved forward by NTP epochs.
/// Mirrors clockBaseEpoch / clockBaseMillis from main_cyd.cpp.
pub struct Clock {
    base_epoch: i64,
    base_millis: u64,
    shared: SharedEpoch,
}

impl Clock {
    pub fn new(now_ms: u64) -> Self {
        let build_epoch: i64 = env!("CYD_BUILD_EPOCH").parse().unwrap_or(0);
        info!("clock source=compile_time_fallback base_epoch={build_epoch}");
        Self {
            base_epoch: build_epoch,
            base_millis: now_ms,
            shared: Arc::new(Mutex::new(None)),
        }
    }

    pub fn shared(&self) -> SharedEpoch {
        Arc::clone(&self.shared)
    }

    /// Current local civil time. `now_ms` must be the frame's millis().
    pub fn now_local(&mut self, now_ms: u64) -> Civil {
        if let Some(epoch) = self.shared.lock().ok().and_then(|mut s| s.take()) {
            self.base_epoch = epoch;
            self.base_millis = now_ms;
            info!("clock source=ntp base_epoch={epoch}");
        }

        let elapsed_secs = now_ms.wrapping_sub(self.base_millis) / 1000;
        let epoch = self.base_epoch + elapsed_secs as i64;

        civil_from_localtime(epoch)
    }
}

fn civil_from_localtime(epoch: i64) -> Civil {
    let mut tm = Tm::default();
    unsafe {
        localtime_r(&epoch, &mut tm);
    }
    Civil {
        year: tm.tm_year + 1900,
        month: (tm.tm_mon + 1) as u8,
        day: tm.tm_mday as u8,
        hour: tm.tm_hour as u8,
        minute: tm.tm_min as u8,
        second: tm.tm_sec as u8,
        weekday: tm.tm_wday as u8,
    }
}

fn apply_timezone(tz: &str) {
    if let Ok(c_tz) = CString::new(tz) {
        unsafe {
            setenv(c"TZ".as_ptr(), c_tz.as_ptr(), 1);
            tzset();
        }
    }
}

fn system_time_is_ntp_valid() -> bool {
    // Same sanity check as the C++ firmware: year > 2016.
    let now = unsafe { time(std::ptr::null_mut()) };
    let mut tm = Tm::default();
    unsafe {
        localtime_r(&now, &mut tm);
    }
    tm.tm_year > 2016 - 1900
}

/// Spawn the background clock-sync thread. It owns the modem and retries
/// forever: connect -> SNTP -> disconnect -> sleep until resync.
pub fn spawn_clock_sync(
    modem: esp_idf_hal::modem::Modem,
    sysloop: esp_idf_svc::eventloop::EspSystemEventLoop,
    ssid: &'static str,
    password: &'static str,
    timezone: &'static str,
    shared: SharedEpoch,
) {
    std::thread::Builder::new()
        .name("clock-sync".into())
        .stack_size(8192)
        .spawn(move || clock_sync_thread(modem, sysloop, ssid, password, timezone, shared))
        .expect("failed to spawn clock-sync thread");
}

fn clock_sync_thread(
    modem: esp_idf_hal::modem::Modem,
    sysloop: esp_idf_svc::eventloop::EspSystemEventLoop,
    ssid: &'static str,
    password: &'static str,
    timezone: &'static str,
    shared: SharedEpoch,
) {
    use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
    use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

    let esp_wifi = match EspWifi::new(modem, sysloop.clone(), None) {
        Ok(wifi) => wifi,
        Err(err) => {
            warn!("clock ntp=failed reason=wifi_init err={err:?}");
            return;
        }
    };
    let mut wifi = match BlockingWifi::wrap(esp_wifi, sysloop) {
        Ok(wifi) => wifi,
        Err(err) => {
            warn!("clock ntp=failed reason=wifi_blocking_init err={err:?}");
            return;
        }
    };

    let auth_method = match crate::wifi::security_for_password(password) {
        crate::wifi::WifiSecurity::Open => AuthMethod::None,
        crate::wifi::WifiSecurity::Wpa2Personal => AuthMethod::WPA2Personal,
    };
    let client_config = ClientConfiguration {
        ssid: ssid.try_into().unwrap_or_default(),
        auth_method,
        password: password.try_into().unwrap_or_default(),
        ..Default::default()
    };
    if wifi
        .set_configuration(&Configuration::Client(client_config))
        .is_err()
    {
        warn!("clock ntp=failed reason=wifi_config");
        return;
    }

    let mut first_sync_done = false;
    loop {
        info!("clock ntp=start ssid=\"{ssid}\"");
        let connected = if wifi.is_connected().unwrap_or(false) {
            true
        } else {
            wifi.start()
                .and_then(|_| {
                    wifi.wifi_mut().connect()?;
                    wifi.wifi_wait_while(
                        || wifi.wifi().is_connected().map(|connected| !connected),
                        Some(std::time::Duration::from_millis(
                            consts::WIFI_CONNECT_TIMEOUT_MS,
                        )),
                    )
                })
                .and_then(|_| wifi.wait_netif_up())
                .is_ok()
        };

        if crate::wifi::should_apply_timezone(connected, first_sync_done) {
            apply_timezone(timezone);
        }

        let attempt_succeeded = if connected {
            info!("wifi connected");
            let conf = esp_idf_svc::sntp::SntpConf {
                servers: crate::wifi::NTP_SERVERS,
                ..Default::default()
            };
            let synced = match esp_idf_svc::sntp::EspSntp::new(&conf) {
                Ok(sntp) => wait_for_sntp(&sntp),
                Err(err) => {
                    warn!("clock ntp=failed reason=sntp_init err={err:?}");
                    false
                }
            };

            if synced {
                let epoch = unsafe { time(std::ptr::null_mut()) };
                if let Ok(mut slot) = shared.lock() {
                    *slot = Some(epoch);
                }
                first_sync_done = true;
                info!("clock source=ntp epoch={epoch}");
            } else {
                warn!("clock ntp=failed reason=sync_timeout fallback=compile_time");
            }

            synced
        } else {
            warn!("clock ntp=skipped reason=wifi_connect_failed");
            false
        };

        if consts::WIFI_DISCONNECT_AFTER_NTP {
            let _ = wifi.disconnect();
            let _ = wifi.stop();
        }

        let sleep_ms = match crate::wifi::next_sync_delay(first_sync_done, attempt_succeeded) {
            crate::wifi::SyncDelay::Retry => consts::NTP_RETRY_INTERVAL_MS,
            crate::wifi::SyncDelay::Resync => consts::NTP_RESYNC_INTERVAL_MS,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
}

fn wait_for_sntp(sntp: &esp_idf_svc::sntp::EspSntp) -> bool {
    use esp_idf_svc::sntp::SyncStatus;

    let deadline =
        std::time::Instant::now() + std::time::Duration::from_millis(consts::NTP_SYNC_TIMEOUT_MS);
    while std::time::Instant::now() < deadline {
        if sntp.get_sync_status() == SyncStatus::Completed && system_time_is_ntp_valid() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}
