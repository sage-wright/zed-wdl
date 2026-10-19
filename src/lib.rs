//! WDL extension for Zed: runs the Sprocket language server.
//!
//! The automatic Sprocket download (`managed_sprocket`) is adapted from
//! St. Jude Rust Labs' `sprocket-zed` extension
//! (https://github.com/stjude-rust-labs/sprocket-zed), used under the MIT
//! license; see `licenses/sprocket-zed-LICENSE-MIT`.

use std::{env, fs, path::Path};

use zed_extension_api::{
    self as zed, serde_json::Value, settings::LspSettings, LanguageServerId, Result,
};

/// Wrapper that drops diagnostics whose message matches a regex.
const FILTER_SCRIPT: &str = include_str!("../scripts/sprocket-filtered");

/// Language server that runs ShellCheck on command blocks and Ruff on their
/// Python heredocs.
const LINT_SCRIPT: &str = include_str!("../scripts/wdl-command-lint");

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

/// Config file the extension writes for Sprocket 0.32+, which dropped
/// `analyzer --except` in favor of `[check] disable` in a config file.
const RULES_CONFIG: &str = "zed-wdl-sprocket.toml";

struct WdlExtension {
    cached_binary: Option<String>,
}

fn get_bool(opts: Option<&Value>, key: &str, default: bool) -> bool {
    opts.and_then(|o| o.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

/// Parses "sprocket v0.32.0 (...)" or "v0.32.0" into (major, minor).
fn parse_version(text: &str) -> Option<(u32, u32)> {
    let word = text
        .split_whitespace()
        .find(|w| w.trim_start_matches('v').starts_with(|c: char| c.is_ascii_digit()))?;
    let mut parts = word.trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// Asks a Sprocket binary for its version (needs the `process:exec` capability).
fn binary_version(path: &str) -> Option<(u32, u32)> {
    let output = zed::process::Command::new(path).arg("--version").output().ok()?;
    parse_version(&String::from_utf8_lossy(&output.stdout))
}

/// Makes a path relative to the extension's working directory absolute.
fn absolute(path: &str) -> Result<String> {
    if Path::new(path).is_absolute() {
        return Ok(path.to_string());
    }
    Ok(env::current_dir()
        .map_err(|e| e.to_string())?
        .join(path)
        .to_string_lossy()
        .into_owned())
}

impl WdlExtension {
    /// A previously downloaded Sprocket in the extension's working directory.
    fn installed_managed_binary(&self, exe: &str) -> Option<String> {
        if let Some(path) = &self.cached_binary {
            if fs::metadata(path).is_ok_and(|m| m.is_file()) {
                return Some(path.clone());
            }
        }
        fs::read_dir(".").ok()?.flatten().find_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with("sprocket-") {
                return None;
            }
            let path = format!("{name}/{exe}");
            fs::metadata(&path).is_ok_and(|m| m.is_file()).then_some(path)
        })
    }

    /// Downloads (or reuses) a Sprocket release managed by the extension.
    /// Returns the binary path (relative to the working directory) and its version.
    fn managed_sprocket(
        &mut self,
        id: &LanguageServerId,
        check_for_updates: bool,
    ) -> Result<(String, Option<(u32, u32)>)> {
        let (platform, arch) = zed::current_platform();
        let exe = if matches!(platform, zed::Os::Windows) {
            "sprocket.exe"
        } else {
            "sprocket"
        };
        let installed = self.installed_managed_binary(exe);
        let version_of = |path: &str| parse_version(path.trim_start_matches("sprocket-"));

        if !check_for_updates {
            if let Some(path) = installed {
                self.cached_binary = Some(path.clone());
                let version = version_of(&path);
                return Ok((path, version));
            }
        }

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "stjude-rust-labs/sprocket",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let arch = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => {
                return Err("Sprocket has no 32-bit x86 builds; install it yourself and \
                            set lsp.sprocket.binary.path"
                    .into())
            }
        };
        let (os, ext, file_type) = match platform {
            zed::Os::Mac => ("apple-darwin", "tar.gz", zed::DownloadedFileType::GzipTar),
            zed::Os::Linux => ("unknown-linux-gnu", "tar.gz", zed::DownloadedFileType::GzipTar),
            zed::Os::Windows => ("pc-windows-msvc", "zip", zed::DownloadedFileType::Zip),
        };

        let version_dir = format!("sprocket-{}", release.version);
        let binary_path = format!("{version_dir}/{exe}");
        let version = parse_version(&release.version);

        if installed.as_deref() == Some(binary_path.as_str()) {
            self.cached_binary = Some(binary_path.clone());
            return Ok((binary_path, version));
        }

        let asset_name = format!("sprocket-{}-{arch}-{os}.{ext}", release.version);
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no Sprocket release asset named {asset_name}"))?;

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(&asset.download_url, &version_dir, file_type)
            .map_err(|e| format!("failed to download Sprocket: {e}"))?;
        zed::make_file_executable(&binary_path)?;

        // Remove older downloaded versions.
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Some(name) = name.to_str() {
                    if name != version_dir && name.starts_with("sprocket-") {
                        fs::remove_dir_all(entry.path()).ok();
                    }
                }
            }
        }

        self.cached_binary = Some(binary_path.clone());
        Ok((binary_path, version))
    }
}

impl zed::Extension for WdlExtension {
    fn new() -> Self {
        Self {
            cached_binary: None,
        }
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if id.as_ref() == "wdl-command-lint" {
            return lint_command(id, worktree);
        }

        // Everything is configured under `lsp.sprocket` in Zed settings.
        let (binary, opts) = match LspSettings::for_worktree(id.as_ref(), worktree) {
            Ok(s) => (s.binary, s.settings),
            Err(_) => (None, None),
        };
        let opts = opts.as_ref();

        // 1. `binary.path`, 2. `sprocket` on PATH, 3. a Sprocket the extension downloads.
        let user_binary = binary
            .as_ref()
            .and_then(|b| b.path.clone())
            .or_else(|| worktree.which("sprocket"));
        let (sprocket, version) = match user_binary {
            Some(path) => {
                let version = binary_version(&path);
                (path, version)
            }
            None => {
                let check = get_bool(opts, "check_for_updates", true);
                let (path, version) = self.managed_sprocket(id, check)?;
                (absolute(&path)?, version)
            }
        };
        // Unknown version: assume a current release.
        let new_config_style = version.map_or(true, |v| v >= (0, 32));

        // Explicit `binary.arguments` replace everything generated below.
        let mut args = match binary.as_ref().and_then(|b| b.arguments.clone()) {
            Some(args) => args,
            None => {
                let mut a = vec!["analyzer".to_string(), "--stdio".to_string()];
                if get_bool(opts, "lint", false) {
                    a.push("--lint".into());
                }
                match opts
                    .and_then(|o| o.get("output_level"))
                    .and_then(Value::as_str)
                    .unwrap_or("quiet")
                {
                    "verbose" => a.push("-v".into()),
                    "trace" => a.push("-vv".into()),
                    _ => {}
                }

                let mut disabled: Vec<String> = Vec::new();
                if !get_bool(opts, "warnings", false) {
                    disabled.extend(WARNING_RULES.iter().map(|r| r.to_string()));
                }
                if let Some(list) = opts.and_then(|o| o.get("except")).and_then(Value::as_array) {
                    disabled.extend(list.iter().filter_map(Value::as_str).map(String::from));
                }

                if !disabled.is_empty() {
                    if new_config_style {
                        // Sprocket 0.32+: rules are disabled in a config file,
                        // merged with any sprocket.toml in the project.
                        let quoted: Vec<String> =
                            disabled.iter().map(|r| format!("\"{r}\"")).collect();
                        let config = format!("[check]\ndisable = [{}]\n", quoted.join(", "));
                        fs::write(RULES_CONFIG, config).map_err(|e| e.to_string())?;
                        a.push("--config".into());
                        a.push(absolute(RULES_CONFIG)?);
                    } else {
                        for rule in disabled {
                            a.push("--except".into());
                            a.push(rule);
                        }
                    }
                }
                a
            }
        };

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
             lsp.sprocket.settings.type_mismatches to true and hide_diagnostics to \"\"",
        )?;
        let script = absolute("sprocket-filtered.py")?;
        fs::write("sprocket-filtered.py", FILTER_SCRIPT).map_err(|e| e.to_string())?;

        env_vars.push(("SPROCKET_BIN".into(), sprocket));
        env_vars.push(("SPROCKET_HIDE_DIAGNOSTICS".into(), hide));
        args.insert(0, script);

        Ok(zed::Command {
            command: python,
            args,
            env: env_vars,
        })
    }
}

/// A linter the extension can download from GitHub releases.
struct Tool {
    name: &'static str,
    repo: &'static str,
}

const SHELLCHECK: Tool = Tool {
    name: "shellcheck",
    repo: "koalaman/shellcheck",
};

const RUFF: Tool = Tool {
    name: "ruff",
    repo: "astral-sh/ruff",
};

impl Tool {
    /// Release asset name for `tag` on this platform.
    fn asset(&self, tag: &str) -> Result<(String, zed::DownloadedFileType)> {
        let (platform, arch) = zed::current_platform();
        let arch = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => {
                return Err(format!(
                    "{} has no 32-bit x86 download; install it yourself and set its path",
                    self.name
                ))
            }
        };
        let tar = zed::DownloadedFileType::GzipTar;
        let zip = zed::DownloadedFileType::Zip;
        Ok(match (self.name, platform) {
            // ShellCheck's Windows zip is x86_64 only; ARM Windows runs it emulated.
            ("shellcheck", zed::Os::Windows) => (format!("shellcheck-{tag}.zip"), zip),
            ("shellcheck", zed::Os::Mac) => (format!("shellcheck-{tag}.darwin.{arch}.tar.gz"), tar),
            ("shellcheck", zed::Os::Linux) => (format!("shellcheck-{tag}.linux.{arch}.tar.gz"), tar),
            (_, zed::Os::Windows) => (format!("ruff-{arch}-pc-windows-msvc.zip"), zip),
            (_, zed::Os::Mac) => (format!("ruff-{arch}-apple-darwin.tar.gz"), tar),
            // The musl build is static, so it also runs on glibc and Alpine hosts.
            (_, zed::Os::Linux) => (format!("ruff-{arch}-unknown-linux-musl.tar.gz"), tar),
        })
    }

    fn exe(&self) -> String {
        match zed::current_platform().0 {
            zed::Os::Windows => format!("{}.exe", self.name),
            _ => self.name.to_string(),
        }
    }

    /// The executable inside `dir` or one directory below it (archives differ).
    fn find_in(&self, dir: &str) -> Option<String> {
        let exe = self.exe();
        let direct = format!("{dir}/{exe}");
        if fs::metadata(&direct).is_ok_and(|m| m.is_file()) {
            return Some(direct);
        }
        fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
            let path = format!("{dir}/{}/{exe}", entry.file_name().to_str()?);
            fs::metadata(&path).is_ok_and(|m| m.is_file()).then_some(path)
        })
    }

    /// Version directories (`<name>-<tag>`) in the extension's working directory.
    fn version_dirs(&self) -> Vec<String> {
        let prefix = format!("{}-", self.name);
        fs::read_dir(".")
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .filter(|n| n.starts_with(&prefix))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Downloads (or reuses) the latest release. Returns an absolute path.
    fn managed(&self, id: &LanguageServerId, check_for_updates: bool) -> Result<String> {
        let installed = self.version_dirs().iter().find_map(|d| self.find_in(d));
        if let (false, Some(path)) = (check_for_updates, &installed) {
            return absolute(path);
        }

        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = match zed::latest_github_release(
            self.repo,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        ) {
            Ok(r) => r,
            // Offline or rate-limited: keep using what's there.
            Err(e) => return installed.map_or(Err(e), |p| absolute(&p)),
        };

        let version_dir = format!("{}-{}", self.name, release.version);
        if let Some(path) = self.find_in(&version_dir) {
            return absolute(&path);
        }

        let (asset_name, file_type) = self.asset(&release.version)?;
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no {} release asset named {asset_name}", self.name))?;
        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(&asset.download_url, &version_dir, file_type)
            .map_err(|e| format!("failed to download {}: {e}", self.name))?;
        let path = self
            .find_in(&version_dir)
            .ok_or_else(|| format!("{asset_name} has no {}", self.exe()))?;
        zed::make_file_executable(&path)?;

        for dir in self.version_dirs() {
            if dir != version_dir {
                fs::remove_dir_all(&dir).ok();
            }
        }
        absolute(&path)
    }
}

/// Runs ShellCheck on command blocks and Ruff on their Python heredocs,
/// configured under `lsp.wdl-command-lint`. With both tools disabled the
/// server still starts but publishes nothing, so WDL users who never turn it
/// on don't see a failed language server.
fn lint_command(id: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
    let opts = LspSettings::for_worktree(id.as_ref(), worktree)
        .ok()
        .and_then(|s| s.settings);
    let opts = opts.as_ref();
    let check = get_bool(opts, "check_for_updates", true);

    let python = worktree.which("python3").ok_or(
        "python3 is needed for the wdl-command-lint language server; install it, or add \
         \"!wdl-command-lint\" to languages.WDL.language_servers",
    )?;
    let script = absolute("wdl-command-lint.py")?;
    fs::write("wdl-command-lint.py", LINT_SCRIPT).map_err(|e| e.to_string())?;

    let mut env_vars = worktree.shell_env();
    // 1. the tool's `path` setting, 2. the tool on PATH, 3. a download.
    let binary = |tool: &Tool, tool_opts: Option<&Value>| -> Result<String> {
        match tool_opts
            .and_then(|o| o.get("path"))
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| worktree.which(tool.name))
        {
            Some(path) => Ok(path),
            None => tool.managed(id, check),
        }
    };
    let list = |tool_opts: Option<&Value>, key: &str| -> Option<String> {
        let items = tool_opts?.get(key)?.as_array()?;
        Some(items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(","))
    };

    let sc = opts.and_then(|o| o.get("shellcheck"));
    if get_bool(sc, "enabled", false) {
        env_vars.push(("SHELLCHECK_BIN".into(), binary(&SHELLCHECK, sc)?));
        if let Some(codes) = list(sc, "exclude") {
            env_vars.push(("SHELLCHECK_EXCLUDE".into(), codes));
        }
        if let Some(level) = sc.and_then(|o| o.get("severity")).and_then(Value::as_str) {
            env_vars.push(("SHELLCHECK_SEVERITY".into(), level.to_string()));
        }
    }
    let ruff = opts.and_then(|o| o.get("ruff"));
    if get_bool(ruff, "enabled", false) {
        env_vars.push(("RUFF_BIN".into(), binary(&RUFF, ruff)?));
        if let Some(codes) = list(ruff, "ignore") {
            env_vars.push(("RUFF_IGNORE".into(), codes));
        }
    }

    Ok(zed::Command {
        command: python,
        args: vec![script],
        env: env_vars,
    })
}

zed::register_extension!(WdlExtension);
