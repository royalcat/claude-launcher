use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, ConfigAccessError, ConfigCorruptError};
use crate::settings::get_config_path;

// Disk shape:
// { "credentials": { "<slug>": { "name": "...", "provider": "<id>", "env": {...} } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub name: String,
    pub provider: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub credentials: HashMap<String, Credential>,
}

pub fn load_config() -> Result<Config, AppError> {
    let path_str = get_config_path();
    let path = Path::new(&path_str);

    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(e) => {
            return Err(AppError::Access(ConfigAccessError {
                path: path_str,
                cause: e.to_string(),
            }));
        }
    };

    #[cfg(unix)]
    fix_permissions_if_needed(path);

    let mut parsed: serde_json::Value = serde_json::from_str(&data).map_err(|e| ConfigCorruptError {
        path: path_str.clone(),
        cause: e.to_string(),
    })?;

    // Ensure credentials key exists
    if !parsed.get("credentials").map(|v| v.is_object()).unwrap_or(false) {
        parsed["credentials"] = serde_json::json!({});
    }

    let config: Config = serde_json::from_value(parsed).map_err(|e| ConfigCorruptError {
        path: path_str,
        cause: e.to_string(),
    })?;

    Ok(config)
}

#[cfg(unix)]
fn fix_permissions_if_needed(path: &Path) {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        if meta.mode() & 0o077 != 0 {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
    }
}

pub fn save_config(config: &Config) -> Result<(), AppError> {
    let path_str = get_config_path();
    let path = Path::new(&path_str);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigAccessError {
            path: path_str.clone(),
            cause: e.to_string(),
        })?;
    }

    let data = serde_json::to_string_pretty(config).expect("serialization never fails");
    fs::write(path, &data).map_err(|e| ConfigAccessError {
        path: path_str.clone(),
        cause: e.to_string(),
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

pub fn get_all_credentials() -> Result<HashMap<String, Credential>, AppError> {
    Ok(load_config()?.credentials)
}

pub fn get_credential(slug: &str) -> Result<Option<Credential>, AppError> {
    Ok(load_config()?.credentials.remove(slug))
}

pub fn save_credential(slug: &str, cred: Credential) -> Result<(), AppError> {
    let mut config = load_config()?;
    config.credentials.insert(slug.to_string(), cred);
    save_config(&config)
}

pub fn remove_credential(slug: &str) -> Result<(), AppError> {
    let mut config = load_config()?;
    config.credentials.remove(slug);
    save_config(&config)
}

pub fn rename_credential(old_slug: &str, new_slug: &str, cred: Credential) -> Result<(), AppError> {
    let mut config = load_config()?;
    if old_slug != new_slug {
        config.credentials.remove(old_slug);
    }
    config.credentials.insert(new_slug.to_string(), cred);
    save_config(&config)
}

// Slugify a display name into a CLI-friendly identifier
pub fn slugify_name(name: &str) -> String {
    let lower = name.to_lowercase();
    let replaced: String = lower.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' }).collect();
    replaced.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

pub fn mask_secret(value: &str) -> String {
    if value.len() <= 4 {
        "****".to_string()
    } else {
        format!("****{}", &value[value.len() - 4..])
    }
}
