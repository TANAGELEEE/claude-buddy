use claude_buddy_changer::buddy::{
    Buddy, SearchFilters, SearchMatch, SearchParams, parse_min_stat, render_blink_sprite,
    render_face, render_sprite, render_sprite_frames, roll_with_salt, search_salts,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PreviewFixture {
    #[serde(rename = "userId")]
    user_id: String,
    salt: String,
    buddy: Buddy,
    face: String,
    sprite: Vec<String>,
    #[serde(rename = "spriteFrames")]
    sprite_frames: Vec<Vec<String>>,
    #[serde(rename = "blinkFrame")]
    blink_frame: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SearchFixture {
    name: String,
    #[serde(rename = "userId")]
    user_id: String,
    total: usize,
    prefix: String,
    length: usize,
    filters: SearchFilters,
    matches: Vec<SearchMatch>,
}

#[test]
fn preview_fixtures_match_rust_output() {
    let fixtures: Vec<PreviewFixture> =
        serde_json::from_str(include_str!("../fixtures/golden-cases.json")).unwrap();

    for fixture in fixtures {
        let buddy = roll_with_salt(&fixture.user_id, &fixture.salt);
        assert_eq!(buddy, fixture.buddy, "buddy mismatch for {}", fixture.salt);
        assert_eq!(
            render_face(&buddy),
            fixture.face,
            "face mismatch for {}",
            fixture.salt
        );
        assert_eq!(
            render_sprite(&buddy, 0),
            fixture.sprite,
            "sprite mismatch for {}",
            fixture.salt
        );
        assert_eq!(
            render_sprite_frames(&buddy),
            fixture.sprite_frames,
            "sprite frame mismatch for {}",
            fixture.salt
        );
        assert_eq!(
            render_blink_sprite(&buddy, 0),
            fixture.blink_frame,
            "blink frame mismatch for {}",
            fixture.salt
        );
    }
}

#[test]
fn search_fixtures_match_rust_output() {
    let fixtures: Vec<SearchFixture> =
        serde_json::from_str(include_str!("../fixtures/search-cases.json")).unwrap();

    for fixture in fixtures {
        let matches = search_salts(SearchParams {
            user_id: fixture.user_id.clone(),
            total: fixture.total,
            prefix: fixture.prefix.clone(),
            length: fixture.length,
            filters: fixture.filters.clone(),
            max_matches: 20,
        });
        assert_eq!(
            matches, fixture.matches,
            "search mismatch for {}",
            fixture.name
        );
    }
}

#[test]
fn invalid_min_stat_matches_js_error_behavior() {
    let error = parse_min_stat("CHAOS:not-a-number").unwrap_err();
    assert_eq!(error, "Invalid min stat value: CHAOS:not-a-number");
}
