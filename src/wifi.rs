#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiSecurity {
    Open,
    Wpa2Personal,
}

pub const NTP_SERVERS: [&str; 3] = ["pool.ntp.org", "time.nist.gov", "time.google.com"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncDelay {
    Retry,
    Resync,
}

pub fn security_for_password(password: &str) -> WifiSecurity {
    if password.is_empty() {
        return WifiSecurity::Open;
    }

    WifiSecurity::Wpa2Personal
}

pub fn should_apply_timezone(wifi_connected: bool, ntp_synced: bool) -> bool {
    wifi_connected && !ntp_synced
}

pub fn next_sync_delay(_ever_synced: bool, attempt_succeeded: bool) -> SyncDelay {
    if attempt_succeeded {
        return SyncDelay::Resync;
    }

    SyncDelay::Retry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_password_selects_open_wifi() {
        assert_eq!(security_for_password(""), WifiSecurity::Open);
    }

    #[test]
    fn password_selects_wpa2_personal() {
        assert_eq!(
            security_for_password("aquarium-secret"),
            WifiSecurity::Wpa2Personal
        );
    }

    #[test]
    fn timezone_is_applied_before_sntp_succeeds() {
        assert!(should_apply_timezone(true, false));
        assert!(!should_apply_timezone(false, false));
        assert!(!should_apply_timezone(true, true));
    }

    #[test]
    fn uses_original_ntp_servers() {
        assert_eq!(
            NTP_SERVERS,
            ["pool.ntp.org", "time.nist.gov", "time.google.com"]
        );
    }

    #[test]
    fn failed_resync_uses_retry_delay() {
        assert_eq!(next_sync_delay(true, false), SyncDelay::Retry);
    }

    #[test]
    fn successful_attempt_uses_resync_delay() {
        assert_eq!(next_sync_delay(false, true), SyncDelay::Resync);
        assert_eq!(next_sync_delay(true, true), SyncDelay::Resync);
    }
}
