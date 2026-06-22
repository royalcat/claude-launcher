use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xdg::BaseDirectories;

use crate::error::{AppError, ConfigAccessError};

pub const DEFAULT_PROFILE_LABEL: &str = "default";

fn xdg_dirs() -> BaseDirectories {
    BaseDirectories::with_prefix("claude-launcher")
}

pub fn settings_path() -> PathBuf {
    xdg_dirs()
        .get_config_file("settings.json")
        .unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("claude-launcher")
                .join("settings.json")
        })
}

pub fn default_config_path() -> PathBuf {
    xdg_dirs()
        .get_config_file("providers.json")
        .unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("claude-launcher")
                .join("providers.json")
        })
}

// ---- Settings file shape -------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RawSettings {
    pub active_profile: Option<String>,
    pub profiles: Option<HashMap<String, String>>,
    pub last_launched_credential: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub active_profile: String,
    pub profiles: HashMap<String, String>,
    pub last_launched_credential: Option<String>,
}

// ---- Slug helpers ---------------------------------------------------------

pub fn slugify_label(name: &str) -> String {
    let lower = name.to_lowercase();
    let replaced: String = lower.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' }).collect();
    let deduped = replaced.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    deduped
}

// ---- File I/O ------------------------------------------------------------

fn read_raw() -> RawSettings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => RawSettings::default(),
    }
}

fn write_raw(settings: &Settings) -> Result<(), ConfigAccessError> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigAccessError {
            path: path.to_string_lossy().into_owned(),
            cause: e.to_string(),
        })?;
    }
    let raw = RawSettings {
        active_profile: Some(settings.active_profile.clone()),
        profiles: Some(settings.profiles.clone()),
        last_launched_credential: settings.last_launched_credential.clone(),
    };
    let data = serde_json::to_string_pretty(&raw).expect("serialization never fails");
    fs::write(&path, &data).map_err(|e| ConfigAccessError {
        path: path.to_string_lossy().into_owned(),
        cause: e.to_string(),
    })?;
    #[cfg(unix)]
    set_permissions_600(&path);
    Ok(())
}

#[cfg(unix)]
fn set_permissions_600(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_permissions_600(_path: &Path) {}

fn normalize(raw: RawSettings) -> (Settings, bool) {
    let mut dirty = false;

    let mut profiles: HashMap<String, String> = raw.profiles.clone().unwrap_or_default();
    let mut active_profile: Option<String> = raw.active_profile.clone();
    let last_launched_credential: Option<String> = raw.last_launched_credential.clone();

    // Fresh install: seed default profile
    if profiles.is_empty() {
        profiles.insert(DEFAULT_PROFILE_LABEL.to_string(), default_config_path().to_string_lossy().into_owned());
        active_profile = Some(DEFAULT_PROFILE_LABEL.to_string());
        dirty = dirty || raw.profiles.is_none();
    }

    // Remove non-string/empty entries
    profiles.retain(|_, v| !v.is_empty());
    if profiles.is_empty() {
        profiles.insert(DEFAULT_PROFILE_LABEL.to_string(), default_config_path().to_string_lossy().into_owned());
        active_profile = Some(DEFAULT_PROFILE_LABEL.to_string());
        dirty = true;
    }

    // Ensure active_profile points at an existing profile
    let active = active_profile.unwrap_or_default();
    let active = if profiles.contains_key(&active) {
        active
    } else {
        dirty = true;
        profiles.keys().next().unwrap().clone()
    };

    (
        Settings {
            active_profile: active,
            profiles,
            last_launched_credential,
        },
        dirty,
    )
}

pub fn load_settings() -> Settings {
    let raw = read_raw();
    let (settings, dirty) = normalize(raw);
    if dirty {
        let _ = write_raw(&settings);
    }
    settings
}

pub fn save_settings(settings: &Settings) -> Result<(), AppError> {
    write_raw(settings).map_err(AppError::Access)
}

// ---- Profile API ---------------------------------------------------------

pub fn list_profiles() -> HashMap<String, String> {
    load_settings().profiles
}

pub fn get_active_profile() -> (String, String) {
    let s = load_settings();
    let path = s.profiles[&s.active_profile].clone();
    (s.active_profile, path)
}

pub fn get_last_launched_credential() -> Option<String> {
    load_settings().last_launched_credential
}

pub fn update_last_launched_credential(slug: &str) {
    let mut s = load_settings();
    s.last_launched_credential = Some(slug.to_string());
    let _ = save_settings(&s);
}

pub fn set_active_profile(label: &str) -> Result<(), AppError> {
    let mut s = load_settings();
    if !s.profiles.contains_key(label) {
        return Err(AppError::Other(format!("Profile \"{label}\" does not exist")));
    }
    s.active_profile = label.to_string();
    save_settings(&s)
}

pub fn add_profile(label: &str, profile_path: &str) -> Result<String, AppError> {
    let slug = slugify_label(label);
    if slug.is_empty() {
        return Err(AppError::Other("Profile label must contain at least one alphanumeric character".into()));
    }
    let mut s = load_settings();
    if s.profiles.contains_key(&slug) {
        return Err(AppError::Other(format!("Profile \"{slug}\" already exists")));
    }
    s.profiles.insert(slug.clone(), profile_path.to_string());
    save_settings(&s)?;
    Ok(slug)
}

pub fn rename_profile(old_label: &str, new_label: &str) -> Result<String, AppError> {
    let new_slug = slugify_label(new_label);
    if new_slug.is_empty() {
        return Err(AppError::Other("Profile label must contain at least one alphanumeric character".into()));
    }
    let mut s = load_settings();
    if !s.profiles.contains_key(old_label) {
        return Err(AppError::Other(format!("Profile \"{old_label}\" does not exist")));
    }
    if new_slug == old_label {
        return Ok(new_slug);
    }
    if s.profiles.contains_key(&new_slug) {
        return Err(AppError::Other(format!("Profile \"{new_slug}\" already exists")));
    }
    let path = s.profiles.remove(old_label).unwrap();
    s.profiles.insert(new_slug.clone(), path);
    if s.active_profile == old_label {
        s.active_profile = new_slug.clone();
    }
    save_settings(&s)?;
    Ok(new_slug)
}

pub fn update_profile_path(label: &str, new_path: &str) -> Result<(), AppError> {
    let mut s = load_settings();
    if !s.profiles.contains_key(label) {
        return Err(AppError::Other(format!("Profile \"{label}\" does not exist")));
    }
    s.profiles.insert(label.to_string(), new_path.to_string());
    save_settings(&s)
}

pub fn remove_profile(label: &str) -> Result<(), AppError> {
    let mut s = load_settings();
    if !s.profiles.contains_key(label) {
        return Err(AppError::Other(format!("Profile \"{label}\" does not exist")));
    }
    if s.active_profile == label {
        return Err(AppError::Other(format!(
            "Cannot delete active profile \"{label}\". Switch to another profile first."
        )));
    }
    s.profiles.remove(label);
    save_settings(&s)
}

// ---- Runtime override (CLI flags) ----------------------------------------

use std::sync::OnceLock;

static RUNTIME_OVERRIDE: OnceLock<std::sync::Mutex<Option<String>>> = OnceLock::new();

fn runtime_override() -> &'static std::sync::Mutex<Option<String>> {
    RUNTIME_OVERRIDE.get_or_init(|| std::sync::Mutex::new(None))
}

pub fn set_runtime_config_path(path: String) {
    *runtime_override().lock().unwrap() = Some(path);
}

pub fn get_config_path() -> String {
    if let Some(ref p) = *runtime_override().lock().unwrap() {
        return p.clone();
    }
    let (_, path) = get_active_profile();
    path
}

// ---- Path expansion -------------------------------------------------------

pub fn expand_path(input: &str) -> String {
    let trimmed = input.trim();
    let expanded = if trimmed == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).to_string_lossy().into_owned()
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest)
            .to_string_lossy()
            .into_owned()
    } else {
        trimmed.to_string()
    };
    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
            .to_string_lossy()
            .into_owned()
    }
}

