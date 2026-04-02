use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BinaryStateEntry {
    #[serde(rename = "originalSalt")]
    pub original_salt: String,
    #[serde(rename = "recordedAt")]
    pub recorded_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateFile {
    pub version: u32,
    pub binaries: BTreeMap<String, BinaryStateEntry>,
}

impl Default for StateFile {
    fn default() -> Self {
        Self {
            version: 1,
            binaries: BTreeMap::new(),
        }
    }
}

pub fn get_state_file_path() -> PathBuf {
    if let Ok(path) = env::var("CLAUDE_BUDDY_CHANGER_STATE_FILE") {
        return PathBuf::from(path);
    }
    home_dir().join(".claude-buddy-changer.json")
}

pub fn read_state() -> StateFile {
    let path = get_state_file_path();
    if !path.exists() {
        return StateFile::default();
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<StateFile>(&raw).ok())
        .unwrap_or_default()
}

pub fn write_state(state: &StateFile) -> Result<(), String> {
    let path = get_state_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    fs::write(path, format!("{contents}\n")).map_err(|error| error.to_string())
}

pub fn get_recorded_original_salt(binary_path: &str) -> Option<String> {
    read_state()
        .binaries
        .get(binary_path)
        .map(|entry| entry.original_salt.clone())
}

pub fn record_original_salt(binary_path: &str, original_salt: &str) -> Result<bool, String> {
    let mut state = read_state();
    if state.binaries.contains_key(binary_path) {
        return Ok(false);
    }
    state.binaries.insert(
        binary_path.to_string(),
        BinaryStateEntry {
            original_salt: original_salt.to_string(),
            recorded_at: current_timestamp(),
        },
    );
    write_state(&state)?;
    Ok(true)
}

fn current_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
