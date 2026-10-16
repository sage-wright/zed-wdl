use std::{env, fs};

use zed_extension_api::{
    self as zed, serde_json::Value, settings::LspSettings, LanguageServerId, Result,
};

/// Wrapper that drops diagnostics whose message matches a regex.
const FILTER_SCRIPT: &str = include_str!("../scripts/sprocket-filtered");

/// Sprocket's warning-level analysis rules (hidden unless `warnings: true`).
const WARNING_RULES: &[&str] = &[
    "UnusedImport",
    "UnusedInput",
    "UnusedDeclaration",
    "UnusedCall",
    "UnnecessaryFunctionCall",
    "UsingFallbackVersion",
];

/// Sprocket's "type mismatch: ..." errors (hidden when `type_mismatches: false`).
const TYPE_MISMATCH: &str = "^type mismatch";

struct WdlExtension;

fn get_bool(opts: Option<&Value>, key: &str, default: bool) -> bool {
    opts.and_then(|o| o.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

impl zed::Extension for WdlExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Everything is configured under `lsp.sprocket` in Zed settings.
        let (binary, opts) = match LspSettings::for_worktree(id.as_ref(), worktree) {
            Ok(s) => (s.binary, s.settings),
            Err(_) => (None, None),
        };
        let opts = opts.as_ref();

        let sprocket = binary
            .as_ref()
            .and_then(|b| b.path.clone())
            .or_else(|| worktree.which("sprocket"))
            .ok_or("`sprocket` not found on PATH (brew install sprocket / cargo install sprocket --locked)")?;

        // Explicit `binary.arguments` replace everything below.
        let mut args = binary
            .as_ref()
            .and_then(|b| b.arguments.clone())
            .unwrap_or_else(|| {
                let mut a = vec!["analyzer".to_string(), "--stdio".to_string()];
                if get_bool(opts, "lint", false) {
                    a.push("--lint".into());
                }
                let mut except: Vec<String> = Vec::new();
                if !get_bool(opts, "warnings", false) {
                    except.extend(WARNING_RULES.iter().map(|r| r.to_string()));
                }
                if let Some(list) = opts.and_then(|o| o.get("except")).and_then(Value::as_array) {
                    except.extend(list.iter().filter_map(Value::as_str).map(String::from));
                }
                for rule in except {
                    a.push("--except".into());
                    a.push(rule);
                }
                a
            });

        let mut env_vars = worktree.shell_env();

        // Messages to drop: type mismatches if `type_mismatches: false`,
        // plus any extra regex in `hide_diagnostics`.
        let mut patterns: Vec<String> = Vec::new();
        if !get_bool(opts, "type_mismatches", true) {
            patterns.push(TYPE_MISMATCH.to_string());
        }
        if let Some(extra) = opts
            .and_then(|o| o.get("hide_diagnostics"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            patterns.push(extra.to_string());
        }
        let hide = patterns
            .iter()
            .map(|p| format!("(?:{p})"))
            .collect::<Vec<_>>()
            .join("|");
        if hide.is_empty() {
            return Ok(zed::Command {
                command: sprocket,
                args,
                env: env_vars,
            });
        }

        let python = worktree.which("python3").ok_or(
            "python3 is needed to filter Sprocket diagnostics; install it, or set \
             lsp.sprocket.settings.hide_diagnostics to \"\" to turn filtering off",
        )?;
        let script = env::current_dir()
            .map_err(|e| e.to_string())?
            .join("sprocket-filtered.py");
        fs::write(&script, FILTER_SCRIPT).map_err(|e| e.to_string())?;

        env_vars.push(("SPROCKET_BIN".into(), sprocket));
        env_vars.push(("SPROCKET_HIDE_DIAGNOSTICS".into(), hide));
        args.insert(0, script.to_string_lossy().into_owned());

        Ok(zed::Command {
            command: python,
            args,
            env: env_vars,
        })
    }
}

zed::register_extension!(WdlExtension);
