//! JSON application config with a discoverable `configuration` folder.
//!
//! On startup the app looks for `configuration/ccg.json`, starting at the current
//! working directory and walking up each ancestor to the filesystem root. If none
//! is found the app errors and dies — intentional, so a misdeploy fails loudly.
//!
//! Any string value may contain the token `|ConfigPath|`, replaced with the
//! absolute path of the discovered `configuration` folder **with a trailing
//! slash**, e.g. `"|ConfigPath|../data/logs/ccguard.log"`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

const CONFIG_DIRNAME: &str = "configuration";
pub const CONFIG_FILENAME: &str = "ccg.json";
const CONFIG_TOKEN: &str = "|ConfigPath|";
const OVERRIDE_ENV: &str = "CCGUARD_CONFIG_DIR";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not locate {CONFIG_DIRNAME}/{CONFIG_FILENAME} from {0} up to the filesystem root")]
    NotFound(String),
    #[error("{OVERRIDE_ENV}={0:?} does not contain {CONFIG_FILENAME}")]
    OverrideMissing(String),
    #[error("reading {0}: {1}")]
    Io(String, std::io::Error),
    #[error("parsing {0}: {1}")]
    Parse(String, serde_json::Error),
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: Option<String>,
    pub path: Option<String>,
    pub max_bytes: Option<u64>,
    pub backup_count: Option<usize>,
    #[serde(default = "default_true")]
    pub to_stdout: bool,
    #[serde(default)]
    pub quiet_targets: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: None,
            path: None,
            max_bytes: None,
            backup_count: None,
            to_stdout: true,
            quiet_targets: Vec::new(),
        }
    }
}

/// Locate the `configuration` folder: `CCGUARD_CONFIG_DIR` override, else cwd then
/// each ancestor. `start` defaults to the current working directory.
pub fn find_config_dir(start: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    if let Ok(override_dir) = std::env::var(OVERRIDE_ENV) {
        let d = PathBuf::from(&override_dir);
        if d.join(CONFIG_FILENAME).is_file() {
            return Ok(d);
        }
        if d.join(CONFIG_DIRNAME).join(CONFIG_FILENAME).is_file() {
            return Ok(d.join(CONFIG_DIRNAME));
        }
        return Err(ConfigError::OverrideMissing(override_dir));
    }

    let start = match start {
        Some(s) => s,
        None => std::env::current_dir().map_err(|e| ConfigError::Io(".".into(), e))?,
    };
    let start = start.canonicalize().unwrap_or(start);
    for dir in std::iter::once(start.as_path()).chain(start.ancestors()) {
        let candidate = dir.join(CONFIG_DIRNAME);
        if candidate.join(CONFIG_FILENAME).is_file() {
            return Ok(candidate);
        }
    }
    Err(ConfigError::NotFound(start.to_string_lossy().into_owned()))
}

/// Recursively replace `|ConfigPath|` in every string within `value`.
fn substitute(value: &mut serde_json::Value, replacement: &str) {
    match value {
        serde_json::Value::String(s) => {
            if s.contains(CONFIG_TOKEN) {
                *s = s.replace(CONFIG_TOKEN, replacement);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| substitute(v, replacement)),
        serde_json::Value::Object(map) => {
            map.values_mut().for_each(|v| substitute(v, replacement))
        }
        _ => {}
    }
}

/// Read + `|ConfigPath|`-resolve + deserialize the config in an explicit folder.
pub fn load_from(config_dir: &Path) -> Result<(AppConfig, PathBuf), ConfigError> {
    let file = config_dir.join(CONFIG_FILENAME);
    let text = std::fs::read_to_string(&file)
        .map_err(|e| ConfigError::Io(file.to_string_lossy().into_owned(), e))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| ConfigError::Parse(file.to_string_lossy().into_owned(), e))?;
    // posix path + trailing slash, so "|ConfigPath|../data" -> "<dir>/../data".
    let mut replacement = config_dir.to_string_lossy().replace('\\', "/");
    if !replacement.ends_with('/') {
        replacement.push('/');
    }
    substitute(&mut value, &replacement);
    let cfg: AppConfig = serde_json::from_value(value)
        .map_err(|e| ConfigError::Parse(file.to_string_lossy().into_owned(), e))?;
    Ok((cfg, config_dir.to_path_buf()))
}

/// Discover and load the config from cwd/ancestors (the startup entry point).
pub fn load() -> Result<(AppConfig, PathBuf), ConfigError> {
    let dir = find_config_dir(None)?;
    load_from(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn substitutes_token_in_nested_strings() {
        let mut v = serde_json::json!({
            "a": "|ConfigPath|../data/x",
            "b": { "c": ["|ConfigPath|y", 1, true] }
        });
        substitute(&mut v, "/root/configuration/");
        assert_eq!(v["a"], "/root/configuration/../data/x");
        assert_eq!(v["b"]["c"][0], "/root/configuration/y");
        assert_eq!(v["b"]["c"][1], 1);
    }

    #[test]
    fn finds_config_dir_in_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        let cfgdir = tmp.path().join("configuration");
        fs::create_dir_all(&cfgdir).unwrap();
        fs::write(cfgdir.join(CONFIG_FILENAME), "{}").unwrap();
        let deep = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();

        let found = find_config_dir(Some(deep)).unwrap();
        assert_eq!(found, cfgdir.canonicalize().unwrap());
    }

    #[test]
    fn missing_config_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = find_config_dir(Some(tmp.path().to_path_buf()));
        assert!(err.is_err());
    }

    #[test]
    fn parses_and_substitutes_full_config() {
        let tmp = tempfile::tempdir().unwrap();
        let cfgdir = tmp.path().join("configuration");
        fs::create_dir_all(&cfgdir).unwrap();
        fs::write(
            cfgdir.join(CONFIG_FILENAME),
            r#"{ "database": { "url": "postgres://x" },
                "logging": { "level": "DEBUG", "path": "|ConfigPath|../data/logs/a.log" } }"#,
        )
        .unwrap();

        let (cfg, dir) = load_from(&cfgdir).unwrap();
        assert_eq!(cfg.database.url, "postgres://x");
        assert_eq!(cfg.logging.level.as_deref(), Some("DEBUG"));
        let want = format!("{}/../data/logs/a.log", dir.to_string_lossy().replace('\\', "/"));
        assert_eq!(cfg.logging.path.as_deref(), Some(want.as_str()));
        assert!(cfg.logging.to_stdout); // defaults to true
    }
}
