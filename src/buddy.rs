use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::assets::assets;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Buddy {
    pub rarity: String,
    pub species: String,
    pub eye: String,
    pub hat: String,
    pub shiny: bool,
    pub stats: Stats,
    #[serde(rename = "inspirationSeed")]
    pub inspiration_seed: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Stats {
    #[serde(rename = "DEBUGGING")]
    pub debugging: u8,
    #[serde(rename = "PATIENCE")]
    pub patience: u8,
    #[serde(rename = "CHAOS")]
    pub chaos: u8,
    #[serde(rename = "WISDOM")]
    pub wisdom: u8,
    #[serde(rename = "SNARK")]
    pub snark: u8,
}

impl Stats {
    pub fn get(&self, name: &str) -> Option<u8> {
        match name {
            "DEBUGGING" => Some(self.debugging),
            "PATIENCE" => Some(self.patience),
            "CHAOS" => Some(self.chaos),
            "WISDOM" => Some(self.wisdom),
            "SNARK" => Some(self.snark),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SearchFilters {
    #[serde(default)]
    pub species: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub eye: Option<String>,
    #[serde(default)]
    pub hat: Option<String>,
    #[serde(default)]
    pub shiny: bool,
    #[serde(rename = "minStat", default)]
    pub min_stat: Option<MinStat>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MinStat {
    pub name: String,
    pub threshold: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SearchMatch {
    pub salt: String,
    pub buddy: Buddy,
}

#[derive(Clone, Debug)]
pub struct SearchParams {
    pub user_id: String,
    pub total: usize,
    pub prefix: String,
    pub length: usize,
    pub filters: SearchFilters,
    pub max_matches: usize,
}

pub fn default_salt() -> &'static str {
    &assets().default_salt
}

pub fn detect_user_id() -> Result<String, String> {
    let home = home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let candidates = vec![
        home.join(".claude").join(".config.json"),
        home.join(".claude.json"),
    ];
    detect_user_id_from_paths(&candidates)
}

pub fn parse_min_stat(value: &str) -> Result<Option<MinStat>, String> {
    if value.is_empty() {
        return Ok(None);
    }

    let mut parts = value.splitn(3, ':');
    let raw_name = parts.next().unwrap_or_default();
    let raw_threshold = parts.next().unwrap_or_default();
    let name = raw_name.trim().to_uppercase();
    let threshold = raw_threshold
        .parse::<f64>()
        .map_err(|_| format!("Invalid min stat value: {value}"))?;

    if !threshold.is_finite() || !assets().stat_names.iter().any(|entry| entry == &name) {
        return Err(format!("Invalid min stat value: {value}"));
    }

    Ok(Some(MinStat { name, threshold }))
}

pub fn roll_with_salt(user_id: &str, salt: &str) -> Buddy {
    let mut rng = Mulberry32::new(hash_string(&format!("{user_id}{salt}")));
    let rarity = roll_rarity(&mut rng);
    let species = pick(&mut rng, &assets().species).to_string();
    let eye = pick(&mut rng, &assets().eyes).to_string();
    let hat = if rarity == "common" {
        "none".to_string()
    } else {
        pick(&mut rng, &assets().hats).to_string()
    };
    let shiny = rng.next() < 0.01;
    let stats = roll_stats(&mut rng, &rarity);
    let inspiration_seed = (rng.next() * 1_000_000_000.0).floor() as u32;

    Buddy {
        rarity,
        species,
        eye,
        hat,
        shiny,
        stats,
        inspiration_seed,
    }
}

pub fn render_sprite(buddy: &Buddy, frame: usize) -> Vec<String> {
    let frames = assets()
        .bodies
        .get(&buddy.species)
        .expect("species body must exist");
    let body = &frames[frame % frames.len()];
    let mut lines: Vec<String> = body
        .iter()
        .map(|line| line.replace("{E}", &buddy.eye))
        .collect();

    if buddy.hat != "none"
        && lines.first().is_some_and(|line| line.trim().is_empty())
        && let Some(line) = assets().hat_lines.get(&buddy.hat)
    {
        lines[0] = line.clone();
    }

    let all_first_lines_blank = frames
        .iter()
        .all(|candidate| candidate.first().is_some_and(|line| line.trim().is_empty()));

    if lines.first().is_some_and(|line| line.trim().is_empty()) && all_first_lines_blank {
        lines.remove(0);
    }

    lines
}

pub fn render_blink_sprite(buddy: &Buddy, frame: usize) -> Vec<String> {
    render_sprite(buddy, frame)
        .into_iter()
        .map(|line| line.replace(&buddy.eye, "-"))
        .collect()
}

pub fn render_sprite_frames(buddy: &Buddy) -> Vec<Vec<String>> {
    let count = sprite_frame_count(&buddy.species);
    (0..count)
        .map(|index| render_sprite(buddy, index))
        .collect()
}

pub fn sprite_frame_count(species: &str) -> usize {
    assets()
        .bodies
        .get(species)
        .expect("species body must exist")
        .len()
}

pub fn render_face(buddy: &Buddy) -> String {
    let eye = buddy.eye.as_str();
    match buddy.species.as_str() {
        "duck" | "goose" => format!("({eye}>"),
        "blob" => format!("({eye}{eye})"),
        "cat" => format!("={eye}ω{eye}="),
        "dragon" => format!("<{eye}~{eye}>"),
        "octopus" => format!("~({eye}{eye})~"),
        "owl" => format!("({eye})({eye})"),
        "penguin" => format!("({eye}>)"),
        "turtle" => format!("[{eye}_{eye}]"),
        "snail" => format!("{eye}(@)"),
        "ghost" => format!("/{eye}{eye}\\"),
        "axolotl" => format!("}}{eye}.{eye}{{"),
        "capybara" => format!("({eye}oo{eye})"),
        "cactus" | "mushroom" => format!("|{eye}  {eye}|"),
        "robot" => format!("[{eye}{eye}]"),
        "rabbit" => format!("({eye}..{eye})"),
        "chonk" => format!("({eye}.{eye})"),
        _ => buddy.species.clone(),
    }
}

pub fn search_salts(params: SearchParams) -> Vec<SearchMatch> {
    let mut matches = Vec::new();

    for index in 0..params.total {
        let salt = generate_salt(&params.prefix, index, params.length)
            .expect("prefix length must not exceed target length");
        let buddy = roll_with_salt(&params.user_id, &salt);
        if matches_filters(&buddy, &params.filters) {
            matches.push(SearchMatch { salt, buddy });
            if matches.len() >= params.max_matches {
                break;
            }
        }
    }

    matches
}

fn detect_user_id_from_paths(paths: &[PathBuf]) -> Result<String, String> {
    for candidate in paths {
        if !candidate.exists() {
            continue;
        }

        let raw = fs::read_to_string(candidate).map_err(|error| error.to_string())?;
        if let Ok(config) = serde_json::from_str::<Value>(&raw) {
            if let Some(user_id) = config
                .get("oauthAccount")
                .and_then(|entry| entry.get("accountUuid"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                return Ok(user_id.to_string());
            }

            if let Some(user_id) = config
                .get("userID")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                return Ok(user_id.to_string());
            }
        }
    }

    let joined = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!("Could not detect userId from {joined}"))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn generate_salt(prefix: &str, index: usize, length: usize) -> Result<String, String> {
    if prefix.len() > length {
        return Err(format!(
            "salt prefix length {} exceeds target length {length}",
            prefix.len()
        ));
    }

    let suffix_length = length.saturating_sub(prefix.len());
    let suffix = format!("{index:0width$}", width = suffix_length);
    Ok(format!("{prefix}{suffix}"))
}

fn matches_filters(result: &Buddy, filters: &SearchFilters) -> bool {
    if filters
        .species
        .as_ref()
        .is_some_and(|species| result.species != *species)
    {
        return false;
    }
    if filters
        .rarity
        .as_ref()
        .is_some_and(|rarity| result.rarity != *rarity)
    {
        return false;
    }
    if filters.eye.as_ref().is_some_and(|eye| result.eye != *eye) {
        return false;
    }
    if filters.hat.as_ref().is_some_and(|hat| result.hat != *hat) {
        return false;
    }
    if filters.shiny && !result.shiny {
        return false;
    }
    if let Some(min_stat) = &filters.min_stat
        && result
            .stats
            .get(&min_stat.name)
            .map(f64::from)
            .unwrap_or_default()
            < min_stat.threshold
    {
        return false;
    }
    true
}

fn roll_rarity(rng: &mut Mulberry32) -> String {
    let total: u32 = assets().rarity_weights.values().sum();
    let mut roll = rng.next() * f64::from(total);

    for rarity in &assets().rarities {
        roll -= f64::from(*assets().rarity_weights.get(rarity).unwrap_or(&0));
        if roll < 0.0 {
            return rarity.clone();
        }
    }

    "common".to_string()
}

fn roll_stats(rng: &mut Mulberry32, rarity: &str) -> Stats {
    let floor = *assets().rarity_floor.get(rarity).unwrap_or(&0);
    let peak = pick(rng, &assets().stat_names).to_string();
    let mut dump = pick(rng, &assets().stat_names).to_string();
    while dump == peak {
        dump = pick(rng, &assets().stat_names).to_string();
    }

    let mut stats = Stats::default();
    for name in &assets().stat_names {
        let value = if *name == peak {
            (u32::from(floor) + 50 + (rng.next() * 30.0).floor() as u32).min(100) as u8
        } else if *name == dump {
            let raw = i32::from(floor) - 10 + (rng.next() * 15.0).floor() as i32;
            raw.max(1) as u8
        } else {
            (u32::from(floor) + (rng.next() * 40.0).floor() as u32) as u8
        };

        match name.as_str() {
            "DEBUGGING" => stats.debugging = value,
            "PATIENCE" => stats.patience = value,
            "CHAOS" => stats.chaos = value,
            "WISDOM" => stats.wisdom = value,
            "SNARK" => stats.snark = value,
            _ => {}
        }
    }

    stats
}

fn hash_string(value: &str) -> u32 {
    let mut hash = 2_166_136_261_u32;
    for code_unit in value.encode_utf16() {
        hash ^= u32::from(code_unit);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn pick<'a>(rng: &mut Mulberry32, values: &'a [String]) -> &'a str {
    let index = (rng.next() * values.len() as f64).floor() as usize;
    &values[index.min(values.len().saturating_sub(1))]
}

struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b79f5);
        let mut t = (self.state ^ (self.state >> 15)).wrapping_mul(1 | self.state);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_min_stat_rejects_invalid_input() {
        let error = parse_min_stat("NOTREAL:10").expect_err("should fail");
        assert!(error.contains("Invalid min stat value"));
    }

    #[test]
    fn render_frame_count_matches_assets() {
        let buddy = roll_with_salt("user-1", default_salt());
        assert_eq!(
            render_sprite_frames(&buddy).len(),
            sprite_frame_count(&buddy.species)
        );
    }
}
