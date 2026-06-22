use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, ConfigAccessError, ConfigCorruptError};
use crate::settings::get_config_path;

// Disk shape:
// { "profiles": { "<slug>": { "name": "...", "provider": "<id>", "env": {...} } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub provider: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
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

    // Ensure profiles key exists
    if !parsed.get("profiles").map(|v| v.is_object()).unwrap_or(false) {
        parsed["profiles"] = serde_json::json!({});
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

pub fn get_all_profiles() -> Result<HashMap<String, Profile>, AppError> {
    Ok(load_config()?.profiles)
}

pub fn get_profile(slug: &str) -> Result<Option<Profile>, AppError> {
    Ok(load_config()?.profiles.remove(slug))
}

pub fn save_profile(slug: &str, profile: Profile) -> Result<(), AppError> {
    let mut config = load_config()?;
    config.profiles.insert(slug.to_string(), profile);
    save_config(&config)
}

pub fn remove_profile(slug: &str) -> Result<(), AppError> {
    let mut config = load_config()?;
    config.profiles.remove(slug);
    save_config(&config)
}

pub fn rename_profile(old_slug: &str, new_slug: &str, profile: Profile) -> Result<(), AppError> {
    let mut config = load_config()?;
    if old_slug != new_slug {
        config.profiles.remove(old_slug);
    }
    config.profiles.insert(new_slug.to_string(), profile);
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
