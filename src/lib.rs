use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

struct WdlExtension;

impl zed::Extension for WdlExtension {
    fn new() -> Self { Self }

    fn language_server_command(&mut self, id: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
        let binary = LspSettings::for_worktree(id.as_ref(), worktree).ok().and_then(|s| s.binary);
        let path = binary.as_ref().and_then(|b| b.path.clone())
            .or_else(|| worktree.which("sprocket"))
            .ok_or("`sprocket` not found on PATH")?;
        let args = binary.as_ref().and_then(|b| b.arguments.clone())
            .unwrap_or_else(|| vec!["analyzer".into(), "--stdio".into(), "--lint".into()]);
        Ok(zed::Command { command: path, args, env: worktree.shell_env() })
    }
}

zed::register_extension!(WdlExtension);
