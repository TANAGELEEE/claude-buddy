use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assets {
    pub default_salt: String,
    pub rarities: Vec<String>,
    pub rarity_weights: HashMap<String, u32>,
    pub species: Vec<String>,
    pub eyes: Vec<String>,
    pub hats: Vec<String>,
    pub stat_names: Vec<String>,
    pub rarity_floor: HashMap<String, u8>,
    pub bodies: HashMap<String, Vec<Vec<String>>>,
    pub hat_lines: HashMap<String, String>,
}

static ASSETS: OnceLock<Assets> = OnceLock::new();

pub fn assets() -> &'static Assets {
    ASSETS.get_or_init(|| {
        serde_json::from_str(include_str!("../fixtures/buddy-assets.json"))
            .expect("fixtures/buddy-assets.json must be valid")
    })
}
