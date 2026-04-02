use claude_buddy_changer::buddy::{
    SearchFilters, SearchParams, default_salt, detect_user_id, parse_min_stat, roll_with_salt,
    search_salts,
};
use serde_json::{Value, json};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args().skip(1).collect())?;
    let command = args.positionals.first().map(String::as_str).unwrap_or("");
    match command {
        "" | "help" | "--help" => {
            print_usage();
            Ok(())
        }
        "preview" => run_preview(&args),
        "search" => run_search(&args),
        _ => Err(format!("Unknown command: {command}")),
    }
}

fn run_preview(args: &ParsedArgs) -> Result<(), String> {
    let user_id = args
        .flags
        .get("user-id")
        .cloned()
        .unwrap_or(detect_user_id()?);
    let salt = args
        .flags
        .get("salt")
        .cloned()
        .unwrap_or_else(|| default_salt().to_string());
    let buddy = roll_with_salt(&user_id, &salt);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "userId": user_id,
            "salt": salt,
            "buddy": buddy,
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_search(args: &ParsedArgs) -> Result<(), String> {
    let user_id = args
        .flags
        .get("user-id")
        .cloned()
        .unwrap_or(detect_user_id()?);
    let total = args
        .flags
        .get("total")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("Invalid --total value: {value}"))
        })
        .transpose()?
        .unwrap_or(100_000);
    if total == 0 {
        return Err(format!(
            "Invalid --total value: {}",
            args.flags.get("total").cloned().unwrap_or_default()
        ));
    }

    let prefix = args
        .flags
        .get("salt-prefix")
        .cloned()
        .unwrap_or_else(|| "lab-".to_string());
    let length = args
        .flags
        .get("length")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("Invalid --length value: {value}"))
        })
        .transpose()?
        .unwrap_or_else(|| {
            args.flags
                .get("salt")
                .map(|salt| salt.len())
                .unwrap_or_else(|| default_salt().len())
        });
    if length == 0 {
        return Err(format!(
            "Invalid --length value: {}",
            args.flags.get("length").cloned().unwrap_or_default()
        ));
    }

    let min_stat = args
        .flags
        .get("min-stat")
        .map(|value| parse_min_stat(value))
        .transpose()?
        .flatten();
    let filters = SearchFilters {
        species: args.flags.get("species").cloned(),
        rarity: args.flags.get("rarity").cloned(),
        eye: args.flags.get("eye").cloned(),
        hat: args.flags.get("hat").cloned(),
        shiny: args.switches.contains(&"shiny".to_string()),
        min_stat,
    };
    let matches = search_salts(SearchParams {
        user_id: user_id.clone(),
        total,
        prefix,
        length,
        filters: filters.clone(),
        max_matches: 20,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "userId": user_id,
            "searched": total,
            "filters": strip_nulls(serde_json::to_value(&filters).map_err(|error| error.to_string())?),
            "matches": matches,
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn print_usage() {
    println!(
        "buddy-lab\n\nUsage:\n  cargo run --bin buddy-lab -- preview [--user-id <id>] [--salt <salt>]\n  cargo run --bin buddy-lab -- search [filters] [--total <n>] [--salt-prefix <p>]\n"
    );
}

#[derive(Default)]
struct ParsedArgs {
    positionals: Vec<String>,
    flags: std::collections::HashMap<String, String>,
    switches: Vec<String>,
}

fn parse_args(tokens: Vec<String>) -> Result<ParsedArgs, String> {
    let mut args = ParsedArgs::default();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = &tokens[index];
        if !token.starts_with("--") {
            args.positionals.push(token.clone());
            index += 1;
            continue;
        }
        let key = token.trim_start_matches("--").to_string();
        if key == "shiny" {
            args.switches.push(key);
            index += 1;
            continue;
        }
        let value = tokens
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("Missing value for --{key}"))?;
        args.flags.insert(key, value.clone());
        index += 2;
    }
    Ok(args)
}

fn strip_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key, strip_nulls(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}
