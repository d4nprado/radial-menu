use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

const LAUNCHER_CONFIG_FILE: &str = "launcher-config.json";
const APP_PREFERENCES_FILE: &str = "app-preferences.json";
const DEFAULT_CONFIG_JSON: &str = include_str!("../../src/config/menu.json");
pub const CONFIG_UPDATED_EVENT: &str = "launcher-config-updated";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfig {
    pub shortcut: String,
    pub items: Vec<LauncherMenuItem>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMenuItem {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub icon: String,
    pub accent: String,
    pub action: LauncherAction,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LauncherAction {
    Program { path: String },
    Directory { path: String },
    Url { url: String },
    System { target: SystemActionTarget },
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemActionTarget {
    Explorer,
    DefaultBrowser,
    Terminal,
    Calculator,
    Notepad,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub start_with_windows: bool,
    pub open_config_on_startup: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            open_config_on_startup: false,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfigResponse {
    pub config: LauncherConfig,
    pub warning: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferencesResponse {
    pub preferences: AppPreferences,
    pub warning: Option<String>,
}

fn config_error(context: &str, error: impl std::fmt::Display) -> String {
    format!("{context}: {error}")
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(|error| {
        config_error("Não foi possível localizar o AppData do aplicativo", error)
    })?;
    fs::create_dir_all(&directory)
        .map_err(|error| config_error("Não foi possível criar a pasta de configuração", error))?;
    Ok(directory)
}

fn launcher_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(LAUNCHER_CONFIG_FILE))
}

fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(APP_PREFERENCES_FILE))
}

fn default_launcher_config() -> Result<LauncherConfig, String> {
    serde_json::from_str(DEFAULT_CONFIG_JSON)
        .map_err(|error| config_error("A configuração padrão embutida é inválida", error))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| config_error("Não foi possível preparar a configuração", error))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|error| config_error("Não foi possível salvar a configuração", error))
}

fn validate_launcher_config(config: &LauncherConfig) -> Result<(), String> {
    let mut ids = HashSet::new();
    for item in &config.items {
        if item.id.trim().is_empty() || item.label.trim().is_empty() || item.icon.trim().is_empty()
        {
            return Err("Cada item precisa ter id, label e ícone.".into());
        }
        if !ids.insert(&item.id) {
            return Err(format!("O id '{}' está duplicado.", item.id));
        }
    }
    Ok(())
}

pub fn load_launcher_config_internal(
    app: &tauri::AppHandle,
) -> Result<LauncherConfigResponse, String> {
    let path = launcher_config_path(app)?;
    if !path.exists() {
        let config = default_launcher_config()?;
        write_json(&path, &config)?;
        return Ok(LauncherConfigResponse {
            config,
            warning: None,
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| config_error("Não foi possível ler a configuração salva", error))?;
    match serde_json::from_str::<LauncherConfig>(&contents).and_then(|config| {
        validate_launcher_config(&config)
            .map(|_| config)
            .map_err(serde::de::Error::custom)
    }) {
        Ok(config) => Ok(LauncherConfigResponse {
            config,
            warning: None,
        }),
        Err(error) => {
            let config = default_launcher_config()?;
            write_json(&path, &config)?;
            Ok(LauncherConfigResponse {
                config,
                warning: Some(format!(
                    "O arquivo salvo estava inválido e o padrão foi carregado: {error}"
                )),
            })
        }
    }
}

pub fn load_preferences_internal(app: &tauri::AppHandle) -> Result<AppPreferencesResponse, String> {
    let path = preferences_path(app)?;
    if !path.exists() {
        let preferences = AppPreferences::default();
        write_json(&path, &preferences)?;
        return Ok(AppPreferencesResponse {
            preferences,
            warning: None,
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| config_error("Não foi possível ler as preferências", error))?;
    match serde_json::from_str::<AppPreferences>(&contents) {
        Ok(preferences) => Ok(AppPreferencesResponse {
            preferences,
            warning: None,
        }),
        Err(error) => {
            let preferences = AppPreferences::default();
            write_json(&path, &preferences)?;
            Ok(AppPreferencesResponse {
                preferences,
                warning: Some(format!(
                    "As preferências estavam inválidas e foram restauradas: {error}"
                )),
            })
        }
    }
}

#[tauri::command]
pub fn load_launcher_config(app: tauri::AppHandle) -> Result<LauncherConfigResponse, String> {
    load_launcher_config_internal(&app)
}

#[tauri::command]
pub fn save_launcher_config(app: tauri::AppHandle, config: LauncherConfig) -> Result<(), String> {
    validate_launcher_config(&config)?;
    write_json(&launcher_config_path(&app)?, &config)?;
    app.emit(CONFIG_UPDATED_EVENT, config).map_err(|error| {
        config_error(
            "A configuração foi salva, mas o menu não foi atualizado",
            error,
        )
    })
}

#[tauri::command]
pub fn reset_launcher_config(app: tauri::AppHandle) -> Result<LauncherConfig, String> {
    let config = default_launcher_config()?;
    write_json(&launcher_config_path(&app)?, &config)?;
    app.emit(CONFIG_UPDATED_EVENT, config.clone())
        .map_err(|error| {
            config_error("O padrão foi salvo, mas o menu não foi atualizado", error)
        })?;
    Ok(config)
}

#[tauri::command]
pub fn get_config_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(launcher_config_path(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn open_config_directory(app: tauri::AppHandle) -> Result<(), String> {
    let directory = app_data_dir(&app)?;
    crate::commands::open_directory_path(&directory)
}

#[tauri::command]
pub fn get_app_preferences(app: tauri::AppHandle) -> Result<AppPreferencesResponse, String> {
    let mut response = load_preferences_internal(&app)?;
    response.preferences.start_with_windows = app.autolaunch().is_enabled().map_err(|error| {
        config_error(
            "Não foi possível consultar a inicialização com o sistema",
            error,
        )
    })?;
    write_json(&preferences_path(&app)?, &response.preferences)?;
    Ok(response)
}

#[tauri::command]
pub fn save_app_preferences(
    app: tauri::AppHandle,
    preferences: AppPreferences,
) -> Result<(), String> {
    write_json(&preferences_path(&app)?, &preferences)
}

#[tauri::command]
pub fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| {
        config_error(
            "Não foi possível alterar a inicialização com o sistema",
            error,
        )
    })?;

    let mut response = load_preferences_internal(&app)?;
    response.preferences.start_with_windows = enabled;
    write_json(&preferences_path(&app)?, &response.preferences)
}

#[tauri::command]
pub fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|error| {
        config_error(
            "Não foi possível consultar a inicialização com o sistema",
            error,
        )
    })
}
