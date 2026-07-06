use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct TerminalInputHandle {
    write_fn: Arc<dyn Fn(Vec<u8>) + Send + Sync>,
}

impl TerminalInputHandle {
    pub fn new(write_fn: impl Fn(Vec<u8>) + Send + Sync + 'static) -> Self {
        Self {
            write_fn: Arc::new(write_fn),
        }
    }

    pub fn write(&self, data: impl Into<Vec<u8>>) {
        (self.write_fn)(data.into());
    }
}

#[derive(Clone)]
pub struct TerminalExecHandle {
    exec_fn: Arc<dyn Fn(TerminalExecRequest) -> anyhow::Result<TerminalExecOutput> + Send + Sync>,
}

impl TerminalExecHandle {
    pub fn new(
        exec_fn: impl Fn(TerminalExecRequest) -> anyhow::Result<TerminalExecOutput>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            exec_fn: Arc::new(exec_fn),
        }
    }

    pub fn exec(&self, request: TerminalExecRequest) -> anyhow::Result<TerminalExecOutput> {
        (self.exec_fn)(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalExecRequest {
    pub command: String,
    pub submit: bool,
    pub wait_for_output: bool,
    pub timeout: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalExecCompletion {
    ObservedOutput,
    ShellIntegrationExit,
    SubmittedOnly,
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalExecOutput {
    pub completion: TerminalExecCompletion,
    pub exit_code: Option<i32>,
    pub output: String,
    pub duration_ms: u64,
}

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

    fn input_handle(&self) -> Option<TerminalInputHandle> {
        None
    }

    fn exec_handle(&self) -> Option<TerminalExecHandle> {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::{
        TerminalExecCompletion, TerminalExecHandle, TerminalExecOutput, TerminalExecRequest,
        TerminalInputHandle,
    };
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn terminal_input_handle_forwards_bytes_to_writer() {
        let written = Arc::new(Mutex::new(Vec::new()));
        let sink = written.clone();
        let handle = TerminalInputHandle::new(move |bytes| {
            sink.lock().expect("written lock").push(bytes);
        });

        handle.write(b"df -h\n".to_vec());

        assert_eq!(vec![b"df -h\n".to_vec()], *written.lock().unwrap());
    }

    #[test]
    fn terminal_exec_handle_delegates_to_executor() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let sink = requests.clone();
        let handle = TerminalExecHandle::new(move |request| {
            sink.lock().expect("requests lock").push(request.command);
            Ok(TerminalExecOutput {
                completion: TerminalExecCompletion::SubmittedOnly,
                exit_code: None,
                output: String::new(),
                duration_ms: 0,
            })
        });

        let result = handle
            .exec(TerminalExecRequest {
                command: "df -h".to_string(),
                submit: true,
                wait_for_output: false,
                timeout: Duration::from_millis(1),
            })
            .expect("exec handle should delegate");

        assert_eq!(TerminalExecCompletion::SubmittedOnly, result.completion);
        assert_eq!(vec!["df -h".to_string()], *requests.lock().unwrap());
    }
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

        assert!(
            env.iter()
                .any(|(key, value)| key == "LANG" && is_utf8_locale(value))
        );
        assert!(
            env.iter()
                .any(|(key, value)| key == "LC_CTYPE" && is_utf8_locale(value))
        );
    }
}
