use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;
use walkdir::WalkDir;

pub fn as_object(arguments: Option<&Value>) -> Result<Map<String, Value>, String> {
    match arguments {
        None => Ok(Map::new()),
        Some(Value::Object(map)) => Ok(map.clone()),
        Some(_) => Err("`arguments` must be a JSON object".to_owned()),
    }
}

pub fn get_bool(arguments: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match arguments.get(key) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("`{key}` must be a boolean")),
    }
}

pub fn get_optional_i64(arguments: &Map<String, Value>, key: &str) -> Result<Option<i64>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("`{key}` must be an integer")),
        Some(_) => Err(format!("`{key}` must be an integer")),
    }
}

pub fn get_optional_usize(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<Option<usize>, String> {
    let value = get_optional_i64(arguments, key)?;
    let Some(value) = value else {
        return Ok(None);
    };
    if value < 0 {
        return Err(format!("`{key}` must be >= 0"));
    }
    usize::try_from(value)
        .map(Some)
        .map_err(|_| format!("`{key}` is too large"))
}

pub fn get_optional_string(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("`{key}` must be a string")),
    }
}

fn get_optional_string_array(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("`{key}` must be an array of strings"))?;

    let mut values = Vec::with_capacity(array.len());
    for item in array {
        let value = item
            .as_str()
            .ok_or_else(|| format!("`{key}` must be an array of strings"))?;
        values.push(value.to_owned());
    }
    Ok(Some(values))
}

fn build_globset(patterns: &[String], key_name: &str) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .map_err(|e| format!("Invalid glob in `{key_name}`: '{pattern}' ({e})"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| format!("Failed to build glob set from `{key_name}`: {e}"))
}

fn collect_dir_files(
    dir: &str,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<String>, String> {
    let dir_path = Path::new(dir);
    if !dir_path.exists() {
        return Err(format!("`dir` does not exist: {dir}"));
    }
    if !dir_path.is_dir() {
        return Err(format!("`dir` is not a directory: {dir}"));
    }

    let include_set = build_globset(include, "include")?;
    let exclude_set = build_globset(exclude, "exclude")?;

    let mut files = Vec::new();
    for entry in WalkDir::new(dir_path) {
        let entry = entry.map_err(|e| format!("Failed to walk directory '{dir}': {e}"))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let relative = path.strip_prefix(dir_path).map_err(|e| {
            format!(
                "Failed to compute relative path for {}: {}",
                path.display(),
                e
            )
        })?;

        if !include_set.is_match(relative) {
            continue;
        }
        if exclude_set.is_match(relative) {
            continue;
        }

        files.push(path.to_string_lossy().to_string());
    }

    Ok(files)
}

pub fn resolve_target_files(
    arguments: &Map<String, Value>,
    required: bool,
) -> Result<Vec<String>, String> {
    let direct_files = get_optional_string_array(arguments, "files")?.unwrap_or_default();
    let dir = get_optional_string(arguments, "dir")?;
    let include = get_optional_string_array(arguments, "include")?
        .unwrap_or_else(|| vec!["**/*.gd".to_owned()]);
    let exclude = get_optional_string_array(arguments, "exclude")?.unwrap_or_default();

    let mut unique_files = BTreeSet::new();
    for file in direct_files {
        unique_files.insert(file);
    }

    if let Some(dir) = dir {
        let dir_files = collect_dir_files(&dir, &include, &exclude)?;
        for file in dir_files {
            unique_files.insert(file);
        }
    } else if arguments.contains_key("include") || arguments.contains_key("exclude") {
        return Err("`include`/`exclude` can only be used with `dir`".to_owned());
    }

    if required && unique_files.is_empty() {
        return Err("Either `files` or `dir` must resolve to at least one file".to_owned());
    }

    Ok(unique_files.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;
    use std::fs;

    fn map_from_json(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn resolve_target_files_from_dir_include_exclude() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let root = temp.path();
        fs::create_dir_all(root.join("sub")).expect("create sub dir");
        fs::write(root.join("a.gd"), "extends Node\n").expect("write a.gd");
        fs::write(root.join("b.txt"), "x\n").expect("write b.txt");
        fs::write(root.join("sub").join("c.gd"), "extends Node\n").expect("write c.gd");
        fs::write(root.join("sub").join("d.gd"), "extends Node\n").expect("write d.gd");

        let args = map_from_json(json!({
            "dir": root.to_string_lossy().to_string(),
            "include": ["**/*.gd"],
            "exclude": ["sub/d.gd"]
        }));

        let files = resolve_target_files(&args, true).expect("resolve files");
        let files: BTreeSet<_> = files.into_iter().collect();

        assert!(files.contains(&root.join("a.gd").to_string_lossy().to_string()));
        assert!(files.contains(&root.join("sub").join("c.gd").to_string_lossy().to_string()));
        assert!(!files.contains(&root.join("sub").join("d.gd").to_string_lossy().to_string()));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn resolve_target_files_deduplicates_files_and_dir_results() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let root = temp.path();
        fs::write(root.join("a.gd"), "extends Node\n").expect("write a.gd");

        let file_path = root.join("a.gd").to_string_lossy().to_string();
        let args = map_from_json(json!({
            "files": [file_path],
            "dir": root.to_string_lossy().to_string(),
            "include": ["a.gd"]
        }));

        let files = resolve_target_files(&args, true).expect("resolve files");
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn resolve_target_files_rejects_include_without_dir() {
        let args = map_from_json(json!({
            "include": ["**/*.gd"]
        }));
        let err = resolve_target_files(&args, false).expect_err("should fail");
        assert_eq!(err, "`include`/`exclude` can only be used with `dir`");
    }
}
