use crate::formatter_manager::{FormatterManager, SERVER_NAME};
use crate::protocol::{error_response, success_response};
use crate::targets::as_object;
use crate::tools::format::{
    call_gdscript_format, format_structured_content, render_format_summary,
};
use crate::tools::lint::{
    DEFAULT_MAX_DIAGNOSTICS, call_gdscript_lint, project_lint_diagnostics, render_lint_summary,
};
use serde_json::{Value, json};

pub const PROTOCOL_VERSION: &str = "2024-11-05";

fn tools_definition() -> Value {
    json!([
        {
            "name": "gdscript_format",
            "description": "Format one or more GDScript files using the latest GDQuest formatter binary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "minItems": 1,
                        "description": "Paths to .gd files to format."
                    },
                    "dir": {
                        "type": "string",
                        "description": "Root directory to scan for files."
                    },
                    "include": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns relative to dir to include (default: [\"**/*.gd\"])."
                    },
                    "exclude": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns relative to dir to exclude."
                    },
                    "check": {
                        "type": "boolean",
                        "description": "Check formatting only; do not modify files."
                    },
                    "stdout": {
                        "type": "boolean",
                        "description": "Print formatted output to stdout instead of modifying files."
                    },
                    "use_spaces": {
                        "type": "boolean",
                        "description": "Use spaces for indentation."
                    },
                    "indent_size": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Number of spaces for indentation when use_spaces is true."
                    },
                    "reorder_code": {
                        "type": "boolean",
                        "description": "Reorder code declarations according to the style guide."
                    },
                    "safe": {
                        "type": "boolean",
                        "description": "Enable safe mode."
                    },
                    "continue_on_error": {
                        "type": "boolean",
                        "description": "Deprecated compatibility flag. Formatting always continues per file."
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "gdscript_lint",
            "description": "Lint GDScript files using the latest GDQuest formatter binary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Paths to .gd files to lint."
                    },
                    "dir": {
                        "type": "string",
                        "description": "Root directory to scan for files."
                    },
                    "include": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns relative to dir to include (default: [\"**/*.gd\"])."
                    },
                    "exclude": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns relative to dir to exclude."
                    },
                    "disable_rules": {
                        "type": "string",
                        "description": "Comma-separated lint rule names to disable."
                    },
                    "max_line_length": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum allowed line length."
                    },
                    "list_rules": {
                        "type": "boolean",
                        "description": "List available lint rules."
                    },
                    "pretty": {
                        "type": "boolean",
                        "description": "Use pretty lint output."
                    },
                    "include_raw_output": {
                        "type": "boolean",
                        "description": "Include raw stdout/stderr in structuredContent."
                    },
                    "max_diagnostics": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Maximum number of diagnostics to return."
                    }
                },
                "additionalProperties": false
            }
        }
    ])
}

pub fn handle_request(request: &Value, manager: &FormatterManager) -> Option<Value> {
    let id = request.get("id")?.clone();
    let method = request.get("method")?.as_str()?;
    let params = request.get("params");

    match method {
        "initialize" => {
            let client_protocol = params
                .and_then(|v| v.get("protocolVersion"))
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION);

            Some(success_response(
                id,
                json!({
                    "protocolVersion": client_protocol,
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": SERVER_NAME,
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            ))
        }
        "ping" => Some(success_response(id, json!({}))),
        "tools/list" => Some(success_response(
            id,
            json!({
                "tools": tools_definition()
            }),
        )),
        "tools/call" => {
            let name = params
                .and_then(|v| v.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default();

            let arguments = match as_object(params.and_then(|v| v.get("arguments"))) {
                Ok(args) => args,
                Err(msg) => return Some(error_response(id, -32602, &msg)),
            };

            match name {
                "gdscript_format" => {
                    return match call_gdscript_format(manager, &arguments) {
                        Ok(result) => {
                            let summary = render_format_summary(&result);
                            let structured = format_structured_content(&result);
                            Some(success_response(
                                id,
                                json!({
                                    "isError": !result.success,
                                    "content": [
                                        {"type": "text", "text": summary}
                                    ],
                                    "structuredContent": structured
                                }),
                            ))
                        }
                        Err(text) => Some(success_response(
                            id,
                            json!({
                                "isError": true,
                                "content": [
                                    {"type": "text", "text": "Format failed. failed_count=1."}
                                ],
                                "structuredContent": {
                                    "ok": false,
                                    "failed_count": 1,
                                    "failures_truncated": false,
                                    "failures": [
                                        {
                                            "file": "<internal>",
                                            "reason": text
                                        }
                                    ]
                                }
                            }),
                        )),
                    };
                }
                "gdscript_lint" => {
                    return match call_gdscript_lint(manager, &arguments) {
                        Ok(result) => {
                            let summary = render_lint_summary(&result);
                            let (diagnostics, diagnostics_truncated) = project_lint_diagnostics(
                                &result.diagnostics,
                                result.max_diagnostics,
                            );
                            let mut structured = json!({
                                "ok": result.success,
                                "exit_code": result.exit_code,
                                "total_diagnostics": result.diagnostics.len(),
                                "error_count": result.error_count,
                                "warning_count": result.warning_count,
                                "max_diagnostics": result.max_diagnostics,
                                "diagnostics_truncated": diagnostics_truncated,
                                "diagnostics": diagnostics
                            });
                            if result.include_raw_output {
                                if let Some(map) = structured.as_object_mut() {
                                    map.insert(
                                        "raw_stdout".to_owned(),
                                        Value::String(result.stdout),
                                    );
                                    map.insert(
                                        "raw_stderr".to_owned(),
                                        Value::String(result.stderr),
                                    );
                                }
                            }
                            Some(success_response(
                                id,
                                json!({
                                    "isError": !result.success,
                                    "content": [
                                        {"type": "text", "text": summary}
                                    ],
                                    "structuredContent": structured
                                }),
                            ))
                        }
                        Err(text) => Some(success_response(
                            id,
                            json!({
                                "isError": true,
                                "content": [
                                    {"type": "text", "text": text}
                                ],
                                "structuredContent": {
                                    "ok": false,
                                    "exit_code": -1,
                                    "total_diagnostics": 0,
                                    "error_count": 0,
                                    "warning_count": 0,
                                    "max_diagnostics": DEFAULT_MAX_DIAGNOSTICS,
                                    "diagnostics_truncated": false,
                                    "diagnostics": []
                                }
                            }),
                        )),
                    };
                }
                _ => return Some(error_response(id, -32602, "Unknown tool name")),
            }
        }
        _ => Some(error_response(id, -32601, "Method not found")),
    }
}
