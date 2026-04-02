use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedSalt {
    pub salt: String,
    pub length: usize,
    pub file_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyResult {
    pub file_path: String,
    pub patch_count: usize,
    pub old_salt: String,
    pub new_salt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreResult {
    pub file_path: String,
    pub patch_count: usize,
    pub previous_salt: String,
    pub restored_salt: String,
}

pub fn detect_binary_salt(binary_path: Option<&str>) -> Option<DetectedSalt> {
    let file_path = resolve_binary_path(binary_path)?;
    detect_binary_salt_from_file(&file_path)
}

pub fn find_claude_binary() -> Option<PathBuf> {
    let candidates = get_claude_binary_candidates();
    for candidate in &candidates {
        if detect_binary_salt_from_file(candidate).is_some() {
            return Some(candidate.clone());
        }
    }
    candidates.into_iter().next()
}

pub fn resolve_binary_path(binary_path: Option<&str>) -> Option<PathBuf> {
    if let Some(binary_path) = binary_path {
        if let Some(direct) = normalize_existing_path(binary_path) {
            return Some(direct);
        }

        let candidates = get_command_candidates(binary_path);
        for candidate in &candidates {
            if detect_binary_salt_from_file(candidate).is_some() {
                return Some(candidate.clone());
            }
        }
        return candidates.into_iter().next();
    }

    find_claude_binary()
}

pub fn apply_binary(new_salt: &str, binary_path: Option<&str>) -> Result<ApplyResult, String> {
    let detected = detect_binary_salt(binary_path).ok_or_else(|| {
        "Could not detect the current salt in the Claude Code binary.".to_string()
    })?;
    let (file_path, patch_count) =
        replace_salt_in_binary(&detected.salt, new_salt, &detected.file_path)?;
    Ok(ApplyResult {
        file_path,
        patch_count,
        old_salt: detected.salt,
        new_salt: new_salt.to_string(),
    })
}

pub fn restore_binary(
    original_salt: &str,
    binary_path: Option<&str>,
) -> Result<RestoreResult, String> {
    let detected = detect_binary_salt(binary_path).ok_or_else(|| {
        "Could not detect the current salt in the Claude Code binary.".to_string()
    })?;
    if detected.salt == original_salt {
        return Ok(RestoreResult {
            file_path: detected.file_path.display().to_string(),
            patch_count: 0,
            previous_salt: detected.salt,
            restored_salt: original_salt.to_string(),
        });
    }

    let (file_path, patch_count) =
        replace_salt_in_binary(&detected.salt, original_salt, &detected.file_path)?;
    Ok(RestoreResult {
        file_path,
        patch_count,
        previous_salt: detected.salt,
        restored_salt: original_salt.to_string(),
    })
}

pub fn detect_binary_salt_from_file(file_path: &Path) -> Option<DetectedSalt> {
    let buffer = fs::read(file_path).ok()?;

    for matcher in [
        find_friend_salt as fn(&[u8]) -> Option<(usize, usize)>,
        find_ccbf_salt,
        find_lab_salt,
    ] {
        if let Some((start, end)) = matcher(&buffer) {
            return Some(DetectedSalt {
                salt: String::from_utf8_lossy(&buffer[start..end]).into_owned(),
                length: end - start,
                file_path: file_path.to_path_buf(),
            });
        }
    }

    None
}

fn replace_salt_in_binary(
    search_salt: &str,
    new_salt: &str,
    binary_path: &Path,
) -> Result<(String, usize), String> {
    if !binary_path.exists() {
        return Err(
            "Could not find claude binary. Use a valid binary path or install Claude Code first."
                .to_string(),
        );
    }
    if search_salt.len() != new_salt.len() {
        return Err(format!(
            "Salt length mismatch: \"{search_salt}\" is {}, \"{new_salt}\" is {}.",
            search_salt.len(),
            new_salt.len()
        ));
    }

    let mut buffer = fs::read(binary_path).map_err(|error| error.to_string())?;
    let search_bytes = search_salt.as_bytes();
    let replace_bytes = new_salt.as_bytes();
    let offsets = find_all_offsets(&buffer, search_bytes);
    if offsets.is_empty() {
        return Err(format!("Could not find \"{search_salt}\" in binary bytes."));
    }

    for offset in &offsets {
        let end = offset + replace_bytes.len();
        buffer[*offset..end].copy_from_slice(replace_bytes);
    }

    fs::write(binary_path, buffer).map_err(|error| error.to_string())?;
    resign_binary_if_needed(binary_path)?;
    Ok((binary_path.display().to_string(), offsets.len()))
}

fn resign_binary_if_needed(file_path: &Path) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        let output = Command::new("codesign")
            .args(["--force", "--sign", "-", &file_path.display().to_string()])
            .output()
            .map_err(|error| {
                format!(
                    "Binary patch succeeded but macOS ad-hoc signing failed for {}: {error}",
                    file_path.display()
                )
            })?;
        if !output.status.success() {
            return Err(format!(
                "Binary patch succeeded but macOS ad-hoc signing failed for {}: {}",
                file_path.display(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    Ok(())
}

fn get_claude_binary_candidates() -> Vec<PathBuf> {
    let mut candidates = get_command_candidates("claude");
    if cfg!(windows)
        && let Some(home) = home_dir()
    {
        candidates.push(home.join(".local").join("bin").join("claude.exe"));
        candidates.push(home.join(".local").join("bin").join("claude"));
    }
    dedupe_paths(candidates)
}

fn get_command_candidates(command_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let (command, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("where.exe", vec![command_name])
    } else {
        ("which", vec!["-a", command_name])
    };

    if let Ok(output) = Command::new(command).args(args).output()
        && output.status.success()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(path) = normalize_existing_path(line.trim()) {
                candidates.push(path);
            }
        }
    }

    dedupe_paths(candidates)
}

fn normalize_existing_path(file_path: &str) -> Option<PathBuf> {
    if file_path.is_empty() {
        return None;
    }
    let path = PathBuf::from(file_path);
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    path.exists().then_some(path)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = Vec::<String>::new();
    let mut unique = Vec::new();
    for path in paths {
        let key = path.display().to_string();
        if !seen.iter().any(|entry| entry == &key) {
            seen.push(key);
            unique.push(path);
        }
    }
    unique
}

fn find_friend_salt(buffer: &[u8]) -> Option<(usize, usize)> {
    find_prefixed_digits(buffer, b"friend-", 4, true)
}

fn find_ccbf_salt(buffer: &[u8]) -> Option<(usize, usize)> {
    find_prefixed_digits(buffer, b"ccbf-", 10, false)
}

fn find_lab_salt(buffer: &[u8]) -> Option<(usize, usize)> {
    find_prefixed_digits(buffer, b"lab-", 11, false)
}

fn find_prefixed_digits(
    buffer: &[u8],
    prefix: &[u8],
    digits: usize,
    trailing_dash_and_digits: bool,
) -> Option<(usize, usize)> {
    for start in 0..buffer.len().saturating_sub(prefix.len()) {
        if &buffer[start..start + prefix.len()] != prefix {
            continue;
        }
        let mut cursor = start + prefix.len();
        if !has_digits(buffer, cursor, digits) {
            continue;
        }
        cursor += digits;

        if trailing_dash_and_digits {
            if buffer.get(cursor) != Some(&b'-') {
                continue;
            }
            cursor += 1;
            let tail_start = cursor;
            while buffer.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
            if cursor == tail_start {
                continue;
            }
        }

        return Some((start, cursor));
    }
    None
}

fn has_digits(buffer: &[u8], start: usize, count: usize) -> bool {
    (0..count).all(|offset| buffer.get(start + offset).is_some_and(u8::is_ascii_digit))
}

fn find_all_offsets(buffer: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut position = 0usize;
    while position + needle.len() <= buffer.len() {
        if &buffer[position..position + needle.len()] == needle {
            offsets.push(position);
        }
        position += 1;
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_friend_salt_from_bytes() {
        let root = std::env::temp_dir().join("cbc-binary-detect");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = root.join("claude.bin");
        fs::write(&file, b"prefix friend-2026-401 suffix").unwrap();

        let detected = detect_binary_salt_from_file(&file).expect("salt should be found");
        assert_eq!(detected.salt, "friend-2026-401");
    }

    #[test]
    fn replace_salt_updates_all_offsets() {
        let root = std::env::temp_dir().join("cbc-binary-replace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = root.join("claude.bin");
        fs::write(
            &file,
            b"friend-2026-401 and friend-2026-401 and friend-2026-401",
        )
        .unwrap();

        let (path, patch_count) =
            replace_salt_in_binary("friend-2026-401", "friend-2026-999", &file).unwrap();
        assert_eq!(path, file.display().to_string());
        assert_eq!(patch_count, 3);
        let updated = fs::read_to_string(&file).unwrap();
        assert!(!updated.contains("friend-2026-401"));
        assert_eq!(updated.matches("friend-2026-999").count(), 3);
    }
}
