use serde::{Deserialize, Serialize};

pub const DEFAULT_UTF8_LOCALE: &str = "en_US.UTF-8";

fn is_utf8_locale(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    upper.contains("UTF-8") || upper.contains("UTF8")
}

pub fn preferred_utf8_locale(env_pairs: &[(String, String)]) -> String {
    env_pairs
        .iter()
        .find_map(|(key, value)| {
            matches!(key.as_str(), "LC_ALL" | "LC_CTYPE" | "LANG")
                .then_some(value.as_str())
                .filter(|value| is_utf8_locale(value))
        })
        .map(str::to_string)
        .or_else(|| {
            ["LC_ALL", "LC_CTYPE", "LANG"].into_iter().find_map(|key| {
                std::env::var(key)
                    .ok()
                    .filter(|value| is_utf8_locale(value))
            })
        })
        .unwrap_or_else(|| DEFAULT_UTF8_LOCALE.to_string())
}

pub fn ensure_utf8_locale_env(env_pairs: &mut Vec<(String, String)>) {
    let locale = preferred_utf8_locale(env_pairs);

    if !env_pairs.iter().any(|(key, _)| key == "LANG") {
        env_pairs.push(("LANG".to_string(), locale.clone()));
    }

    if !env_pairs.iter().any(|(key, _)| key == "LC_CTYPE") {
        env_pairs.push(("LC_CTYPE".to_string(), locale));
    }
}

/// Terminal backend trait - abstracts local PTY and SSH backends
pub trait TerminalBackend: Send {
    fn write(&self, data: Vec<u8>);
    fn resize(&self, size: TerminalSize);
    fn shutdown(&self);
}

/// Local terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    /// Shell command (default: system default shell)
    pub shell: Option<String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        let mut env = vec![
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("COLORTERM".to_string(), "truecolor".to_string()),
            ("CLICOLOR".to_string(), "1".to_string()),
            ("CLICOLOR_FORCE".to_string(), "1".to_string()),
        ];
        ensure_utf8_locale_env(&mut env);

        Self {
            shell: None,
            working_dir: None,
            env,
        }
    }
}

/// Terminal dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_utf8_locale_env, is_utf8_locale, preferred_utf8_locale};

    #[test]
    fn preferred_utf8_locale_prefers_existing_utf8_values() {
        let env = vec![("LANG".to_string(), "zh_CN.UTF-8".to_string())];

        assert_eq!(preferred_utf8_locale(&env), "zh_CN.UTF-8");
    }

    #[test]
    fn ensure_utf8_locale_env_adds_missing_locale_vars() {
        let mut env = vec![("TERM".to_string(), "xterm-256color".to_string())];

        ensure_utf8_locale_env(&mut env);

        assert!(env
            .iter()
            .any(|(key, value)| key == "LANG" && is_utf8_locale(value)));
        assert!(env
            .iter()
            .any(|(key, value)| key == "LC_CTYPE" && is_utf8_locale(value)));
    }
}
