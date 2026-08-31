use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xdg::BaseDirectories;

use crate::error::{AppError, ConfigAccessError};

pub const DEFAULT_WORKSPACE_LABEL: &str = "default";

fn xdg_dirs() -> BaseDirectories {
    BaseDirectories::with_prefix("claude-launcher")
}

pub fn settings_path() -> PathBuf {
    xdg_dirs().get_config_file("settings.json").unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claude-launcher")
            .join("settings.json")
    })
}

pub fn default_config_path() -> PathBuf {
    workspace_config_path("default")
}

pub fn workspace_config_path(name: &str) -> PathBuf {
    xdg_dirs().get_config_file(format!("workspaces/{name}.json")).unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claude-launcher")
            .join("workspaces")
            .join(format!("{name}.json"))
    })
}

// ---- Settings file shape -------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RawSettings {
    pub active_workspace: Option<String>,
    pub workspaces: Option<HashMap<String, String>>,
    pub last_launched_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub active_workspace: String,
    pub workspaces: HashMap<String, String>,
    pub last_launched_profile: Option<String>,
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
        active_workspace: Some(settings.active_workspace.clone()),
        workspaces: Some(settings.workspaces.clone()),
        last_launched_profile: settings.last_launched_profile.clone(),
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

    let mut workspaces: HashMap<String, String> = raw.workspaces.clone().unwrap_or_default();
    let mut active_workspace: Option<String> = raw.active_workspace.clone();
    let last_launched_profile: Option<String> = raw.last_launched_profile.clone();

    // Fresh install: seed default workspace
    if workspaces.is_empty() {
        workspaces.insert(DEFAULT_WORKSPACE_LABEL.to_string(), default_config_path().to_string_lossy().into_owned());
        active_workspace = Some(DEFAULT_WORKSPACE_LABEL.to_string());
        dirty = dirty || raw.workspaces.is_none();
    }

    // Remove non-string/empty entries
    workspaces.retain(|_, v| !v.is_empty());
    if workspaces.is_empty() {
        workspaces.insert(DEFAULT_WORKSPACE_LABEL.to_string(), default_config_path().to_string_lossy().into_owned());
        active_workspace = Some(DEFAULT_WORKSPACE_LABEL.to_string());
        dirty = true;
    }

    // Ensure active_workspace points at an existing workspace
    let active = active_workspace.unwrap_or_default();
    let active = if workspaces.contains_key(&active) {
        active
    } else {
        dirty = true;
        workspaces.keys().next().unwrap().clone()
    };

    (
        Settings {
            active_workspace: active,
            workspaces,
            last_launched_profile,
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

// ---- Workspace API --------------------------------------------------------

pub fn list_workspaces() -> HashMap<String, String> {
    load_settings().workspaces
}

pub fn get_active_workspace() -> (String, String) {
    let s = load_settings();
    let path = s.workspaces[&s.active_workspace].clone();
    (s.active_workspace, path)
}

pub fn get_last_launched_profile() -> Option<String> {
    load_settings().last_launched_profile
}

pub fn update_last_launched_profile(slug: &str) {
    let mut s = load_settings();
    s.last_launched_profile = Some(slug.to_string());
    let _ = save_settings(&s);
}

pub fn set_active_workspace(label: &str) -> Result<(), AppError> {
    let mut s = load_settings();
    if !s.workspaces.contains_key(label) {
        return Err(AppError::Other(format!("Workspace \"{label}\" does not exist")));
    }
    s.active_workspace = label.to_string();
    save_settings(&s)
}

pub fn add_workspace(label: &str, workspace_path: &str) -> Result<String, AppError> {
    let slug = slugify_label(label);
    if slug.is_empty() {
        return Err(AppError::Other("Workspace label must contain at least one alphanumeric character".into()));
    }
    let mut s = load_settings();
    if s.workspaces.contains_key(&slug) {
        return Err(AppError::Other(format!("Workspace \"{slug}\" already exists")));
    }
    s.workspaces.insert(slug.clone(), workspace_path.to_string());
    save_settings(&s)?;
    Ok(slug)
}

pub fn rename_workspace(old_label: &str, new_label: &str) -> Result<String, AppError> {
    let new_slug = slugify_label(new_label);
    if new_slug.is_empty() {
        return Err(AppError::Other("Workspace label must contain at least one alphanumeric character".into()));
    }
    let mut s = load_settings();
    if !s.workspaces.contains_key(old_label) {
        return Err(AppError::Other(format!("Workspace \"{old_label}\" does not exist")));
    }
    if new_slug == old_label {
        return Ok(new_slug);
    }
    if s.workspaces.contains_key(&new_slug) {
        return Err(AppError::Other(format!("Workspace \"{new_slug}\" already exists")));
    }
    let path = s.workspaces.remove(old_label).unwrap();
    s.workspaces.insert(new_slug.clone(), path);
    if s.active_workspace == old_label {
        s.active_workspace = new_slug.clone();
    }
    save_settings(&s)?;
    Ok(new_slug)
}

pub fn update_workspace_path(label: &str, new_path: &str) -> Result<(), AppError> {
    let mut s = load_settings();
    if !s.workspaces.contains_key(label) {
        return Err(AppError::Other(format!("Workspace \"{label}\" does not exist")));
    }
    s.workspaces.insert(label.to_string(), new_path.to_string());
    save_settings(&s)
}

pub fn remove_workspace(label: &str) -> Result<(), AppError> {
    let mut s = load_settings();
    if !s.workspaces.contains_key(label) {
        return Err(AppError::Other(format!("Workspace \"{label}\" does not exist")));
    }
    if s.active_workspace == label {
        return Err(AppError::Other(format!(
            "Cannot delete active workspace \"{label}\". Switch to another workspace first."
        )));
    }
    s.workspaces.remove(label);
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
    let (_, path) = get_active_workspace();
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
