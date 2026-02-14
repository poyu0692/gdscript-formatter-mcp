use crate::formatter_manager::FormatterManager;
use crate::targets::{
    get_bool, get_optional_i64, get_optional_string, get_optional_usize, resolve_target_files,
};
use serde_json::{Map, Value, json};
use std::process::Command;

pub const DEFAULT_MAX_DIAGNOSTICS: usize = 500;

pub struct LintToolResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub diagnostics: Vec<Value>,
    pub include_raw_output: bool,
    pub max_diagnostics: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

fn parse_lint_diagnostics(stdout: &str) -> Vec<Value> {
    let mut diagnostics = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((header, message)) = trimmed.split_once(": ") else {
            continue;
        };

        let mut parts = header.rsplitn(4, ':');
        let Some(severity) = parts.next() else {
            continue;
        };
        let Some(rule) = parts.next() else {
            continue;
        };
        let Some(line_no) = parts.next() else {
            continue;
        };
        let Some(file_path) = parts.next() else {
            continue;
        };

        let Ok(line_number) = line_no.parse::<u64>() else {
            continue;
        };

        diagnostics.push(json!({
            "file": file_path,
            "line": line_number,
            "column": Value::Null,
            "rule": rule,
            "severity": severity,
            "message": message
        }));
    }

    diagnostics
}

pub fn project_lint_diagnostics(
    diagnostics: &[Value],
    max_diagnostics: usize,
) -> (Vec<Value>, bool) {
    let projected = diagnostics
        .iter()
        .take(max_diagnostics)
        .cloned()
        .collect::<Vec<_>>();
    let truncated = diagnostics.len() > projected.len();
    (projected, truncated)
}

pub fn render_lint_summary(result: &LintToolResult) -> String {
    format!(
        "Lint {}. diagnostics: total={}, errors={}, warnings={}",
        if result.success {
            "completed successfully"
        } else {
            "failed"
        },
        result.diagnostics.len(),
        result.error_count,
        result.warning_count
    )
}

pub fn call_gdscript_lint(
    manager: &FormatterManager,
    arguments: &Map<String, Value>,
) -> Result<LintToolResult, String> {
    let files = resolve_target_files(arguments, false)?;
    let disable_rules = get_optional_string(arguments, "disable_rules")?;
    let max_line_length = get_optional_i64(arguments, "max_line_length")?;
    let list_rules = get_bool(arguments, "list_rules")?;
    let pretty = get_bool(arguments, "pretty")?;
    let include_raw_output = get_bool(arguments, "include_raw_output")?;
    let max_diagnostics =
        get_optional_usize(arguments, "max_diagnostics")?.unwrap_or(DEFAULT_MAX_DIAGNOSTICS);

    if let Some(value) = max_line_length
        && value < 1
    {
        return Err("`max_line_length` must be at least 1".to_owned());
    }
    if files.is_empty() && !list_rules {
        return Err(
            "Either `files` or `dir` must resolve to at least one file unless `list_rules` is true"
                .to_owned(),
        );
    }

    let binary = manager.ensure_binary()?;
    let mut command = Command::new(binary);
    command.arg("lint");

    if let Some(disable) = disable_rules {
        command.arg("--disable").arg(disable);
    }
    if let Some(value) = max_line_length {
        command.arg("--max-line-length").arg(value.to_string());
    }
    if list_rules {
        command.arg("--list-rules");
    }
    if pretty {
        command.arg("--pretty");
    }
    command.args(&files);

    let output = command
        .output()
        .map_err(|e| format!("Failed to execute linter: {e}"))?;
    let stdout_text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
    let diagnostics = parse_lint_diagnostics(&stdout_text);
    let error_count = diagnostics
        .iter()
        .filter(|d| d.get("severity").and_then(Value::as_str) == Some("error"))
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| d.get("severity").and_then(Value::as_str) == Some("warning"))
        .count();
    let success = output.status.success();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(LintToolResult {
        success,
        exit_code,
        stdout: stdout_text,
        stderr: stderr_text,
        diagnostics,
        include_raw_output,
        max_diagnostics,
        error_count,
        warning_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lint_diagnostics_parses_standard_output() {
        let stdout = "/tmp/a.gd:10:class-name:error: bad class name\n/tmp/a.gd:20:max-line-length:warning: too long\n";
        let diagnostics = parse_lint_diagnostics(stdout);
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0]["file"], "/tmp/a.gd");
        assert_eq!(diagnostics[0]["line"], 10);
        assert_eq!(diagnostics[0]["rule"], "class-name");
        assert_eq!(diagnostics[0]["severity"], "error");
        assert_eq!(diagnostics[1]["rule"], "max-line-length");
        assert_eq!(diagnostics[1]["severity"], "warning");
    }

    #[test]
    fn project_lint_diagnostics_respects_max() {
        let diagnostics = vec![
            json!({"file":"a.gd","line":1,"severity":"warning","rule":"x","message":"m"}),
            json!({"file":"b.gd","line":2,"severity":"error","rule":"y","message":"m"}),
        ];
        let (projected, truncated) = project_lint_diagnostics(&diagnostics, 1);
        assert_eq!(projected.len(), 1);
        assert!(truncated);
    }
}
