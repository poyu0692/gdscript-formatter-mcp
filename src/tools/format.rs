use crate::formatter_manager::FormatterManager;
use crate::targets::{get_bool, get_optional_i64, resolve_target_files};
use serde_json::{Map, Value, json};
use std::path::Path;
use std::process::Command;

const DEFAULT_MAX_FAILURES_RETURNED: usize = 20;

pub struct FormatToolResult {
    pub success: bool,
    pub processed_count: usize,
    pub failures: Vec<FormatFailure>,
}

pub struct FormatFailure {
    pub file: String,
    pub reason: String,
}

#[allow(clippy::too_many_arguments)]
fn build_format_command(
    binary_path: &Path,
    check: bool,
    stdout: bool,
    use_spaces: bool,
    indent_size: Option<i64>,
    reorder_code: bool,
    safe: bool,
    files: &[String],
) -> Command {
    let mut command = Command::new(binary_path);

    if check {
        command.arg("--check");
    }
    if stdout {
        command.arg("--stdout");
    }
    if use_spaces {
        command.arg("--use-spaces");
    }
    if let Some(size) = indent_size {
        command.arg("--indent-size").arg(size.to_string());
    }
    if reorder_code {
        command.arg("--reorder-code");
    }
    if safe {
        command.arg("--safe");
    }
    command.args(files);
    command
}

fn normalize_reason(text: &str) -> String {
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();
    if normalized.is_empty() {
        "Unknown formatting error".to_owned()
    } else {
        normalized
    }
}

fn extract_format_failure_reason(stdout: &str, stderr: &str) -> String {
    for line in stderr.lines() {
        if let Some((_, quoted_error)) = line.split_once("Error: \"") {
            let trimmed = quoted_error.trim_end_matches('"');
            if let Some((_, reason)) = trimmed.split_once(": ") {
                return normalize_reason(reason);
            }
            return normalize_reason(trimmed);
        }
    }

    for line in stderr.lines() {
        if let Some((_, rest)) = line.split_once("Failed to format file ")
            && let Some((_, reason)) = rest.split_once(':')
        {
            return normalize_reason(reason.trim_matches('"'));
        }
    }

    let stderr_reason = normalize_reason(stderr);
    if stderr_reason != "Unknown formatting error" {
        return stderr_reason;
    }

    normalize_reason(stdout)
}

pub fn render_format_summary(result: &FormatToolResult) -> String {
    if result.success {
        "Format ok.".to_owned()
    } else {
        format!("Format failed. failed_count={}.", result.failures.len())
    }
}

pub fn format_structured_content(result: &FormatToolResult) -> Value {
    if result.success {
        return json!({
            "ok": true,
            "processed_count": result.processed_count
        });
    }

    let failures = result
        .failures
        .iter()
        .take(DEFAULT_MAX_FAILURES_RETURNED)
        .map(|f| {
            json!({
                "file": f.file,
                "reason": f.reason
            })
        })
        .collect::<Vec<_>>();
    let failures_truncated = result.failures.len() > DEFAULT_MAX_FAILURES_RETURNED;
    json!({
        "ok": false,
        "processed_count": result.processed_count,
        "failed_count": result.failures.len(),
        "failures_truncated": failures_truncated,
        "failures": failures
    })
}

pub fn call_gdscript_format(
    manager: &FormatterManager,
    arguments: &Map<String, Value>,
) -> Result<FormatToolResult, String> {
    let files = resolve_target_files(arguments, true)?;
    let check = get_bool(arguments, "check")?;
    let stdout = get_bool(arguments, "stdout")?;
    let use_spaces = get_bool(arguments, "use_spaces")?;
    let reorder_code = get_bool(arguments, "reorder_code")?;
    let safe = get_bool(arguments, "safe")?;
    let indent_size = get_optional_i64(arguments, "indent_size")?;

    if let Some(size) = indent_size
        && size < 1
    {
        return Err("`indent_size` must be at least 1".to_owned());
    }

    let binary = manager.ensure_binary()?;
    let mut failures = Vec::new();

    for file in &files {
        let single_file = vec![file.clone()];
        let output = build_format_command(
            binary.as_path(),
            check,
            stdout,
            use_spaces,
            indent_size,
            reorder_code,
            safe,
            &single_file,
        )
        .output();

        match output {
            Ok(output) => {
                let file_stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let file_stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !output.status.success() {
                    failures.push(FormatFailure {
                        file: file.clone(),
                        reason: extract_format_failure_reason(&file_stdout, &file_stderr),
                    });
                }
            }
            Err(err) => {
                failures.push(FormatFailure {
                    file: file.clone(),
                    reason: normalize_reason(&format!("Failed to execute formatter: {err}")),
                });
            }
        }
    }

    let success = failures.is_empty();
    let processed_count = files.len();
    Ok(FormatToolResult {
        success,
        processed_count,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_format_failure_reason_from_stderr() {
        let stderr = "Formatting 1 file...Error: \"Failed to format file /tmp/bad.gd: Topiary formatting failed\"";
        let reason = extract_format_failure_reason("", stderr);
        assert_eq!(reason, "Topiary formatting failed");
    }

    #[test]
    fn extract_format_failure_reason_from_read_error() {
        let stderr = "Formatting 1 file...Error: \"Failed to read file /tmp/missing.gd: No such file or directory (os error 2)\"";
        let reason = extract_format_failure_reason("", stderr);
        assert_eq!(reason, "No such file or directory (os error 2)");
    }

    #[test]
    fn render_format_summary_is_minimal() {
        let success = FormatToolResult {
            success: true,
            processed_count: 5,
            failures: Vec::new(),
        };
        assert_eq!(render_format_summary(&success), "Format ok.");

        let failed = FormatToolResult {
            success: false,
            processed_count: 5,
            failures: vec![FormatFailure {
                file: "a.gd".to_owned(),
                reason: "reason".to_owned(),
            }],
        };
        assert_eq!(
            render_format_summary(&failed),
            "Format failed. failed_count=1."
        );
    }

    #[test]
    fn format_structured_content_success_is_minimal() {
        let success = FormatToolResult {
            success: true,
            processed_count: 10,
            failures: Vec::new(),
        };
        let structured = format_structured_content(&success);
        assert_eq!(structured, json!({"ok": true, "processed_count": 10}));
    }

    #[test]
    fn format_structured_content_truncates_failures() {
        let failures = (0..(DEFAULT_MAX_FAILURES_RETURNED + 1))
            .map(|i| FormatFailure {
                file: format!("f{i}.gd"),
                reason: "reason".to_owned(),
            })
            .collect::<Vec<_>>();
        let failed = FormatToolResult {
            success: false,
            processed_count: DEFAULT_MAX_FAILURES_RETURNED + 1,
            failures,
        };
        let structured = format_structured_content(&failed);
        assert_eq!(
            structured["failed_count"],
            json!(DEFAULT_MAX_FAILURES_RETURNED + 1)
        );
        assert_eq!(structured["failures_truncated"], json!(true));
        assert_eq!(
            structured["failures"].as_array().map(Vec::len),
            Some(DEFAULT_MAX_FAILURES_RETURNED)
        );
    }
}
