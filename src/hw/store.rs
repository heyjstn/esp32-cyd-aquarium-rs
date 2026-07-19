//! NVS-backed aquarium state persistence, replacing the C++ LittleFS JSON
//! file. One small JSON document per fish fits NVS entry limits comfortably.

use anyhow::Result;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use log::warn;

use crate::fish::FishDefinition;
use crate::state::{fish_from_json, fish_to_json};

const NAMESPACE: &str = "aquarium";
const COUNT_KEY: &str = "count";
const MAX_FISH: usize = 24;
const ENTRY_BUF: usize = 1024;

pub struct NvsStore {
    nvs: EspNvs<NvsDefault>,
}

impl NvsStore {
    pub fn new() -> Result<Self> {
        let partition = EspNvsPartition::<NvsDefault>::take()?;
        let nvs = EspNvs::new(partition, NAMESPACE, true)?;
        Ok(Self { nvs })
    }

    pub fn save_fishes(&mut self, defs: &[FishDefinition]) -> Result<()> {
        let count = defs.len().min(MAX_FISH);
        for (i, def) in defs.iter().take(count).enumerate() {
            let json = fish_to_json(def);
            if json.len() >= ENTRY_BUF {
                warn!(
                    "nvs: fish {} json too large ({} bytes), skipped",
                    i,
                    json.len()
                );
                continue;
            }
            self.nvs.set_str(&key(i), &json)?;
        }
        self.nvs.set_u8(COUNT_KEY, count as u8)?;
        Ok(())
    }

    #[allow(dead_code)] // load path kept for parity with the C++ loadState()
    pub fn load_fishes(&self) -> Vec<FishDefinition> {
        let count = match self.nvs.get_u8(COUNT_KEY) {
            Ok(Some(count)) => count as usize,
            _ => return Vec::new(),
        };

        let mut defs = Vec::with_capacity(count);
        let mut buf = [0u8; ENTRY_BUF];
        for i in 0..count.min(MAX_FISH) {
            match self.nvs.get_str(&key(i), &mut buf) {
                Ok(Some(json)) => {
                    if let Some(def) = fish_from_json(json) {
                        defs.push(def);
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    warn!("nvs: read fish {i} failed: {err:?}");
                    break;
                }
            }
        }
        defs
    }
}

fn key(index: usize) -> String {
    format!("fish{index}")
}
