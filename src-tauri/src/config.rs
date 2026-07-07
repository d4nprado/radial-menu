use crate::mouse_shortcut::MouseShortcutManager;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const LAUNCHER_CONFIG_FILE: &str = "launcher-config.json";
const APP_PREFERENCES_FILE: &str = "app-preferences.json";
pub const DEFAULT_SHORTCUT: &str = "Ctrl+Space";
pub const CONFIG_UPDATED_EVENT: &str = "launcher-config-updated";
pub const SHORTCUT_UPDATED_EVENT: &str = "launcher-shortcut-updated";
const MAX_MENU_ITEMS_PER_LEVEL: usize = 10;
const MAX_GROUP_DEPTH: usize = 3;
const MIN_RADIAL_MENU_SIZE: u8 = 0;
const MAX_RADIAL_MENU_SIZE: u8 = 100;

pub struct ShortcutRegistrationState(pub Mutex<String>);

impl ShortcutRegistrationState {
    pub fn new() -> Self {
        Self(Mutex::new(String::new()))
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfig {
    pub shortcut: String,
    #[serde(default = "default_radial_menu_size")]
    pub radial_menu_size: u8,
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
    Program {
        path: String,
    },
    Directory {
        path: String,
    },
    Url {
        #[serde(alias = "value", alias = "path")]
        url: String,
    },
    #[serde(rename = "windows_app")]
    WindowsApp {
        #[serde(rename = "appUserModelId")]
        app_user_model_id: String,
        #[serde(default)]
        label: String,
    },
    System {
        target: SystemActionTarget,
    },
    Stream {
        provider: crate::stream::StreamProvider,
        operation: crate::stream::StreamOperation,
        #[serde(default, rename = "sceneName")]
        scene_name: String,
        #[serde(default, rename = "inputName")]
        input_name: String,
        #[serde(default, rename = "sourceName")]
        source_name: String,
        #[serde(default)]
        muted: Option<bool>,
        #[serde(default)]
        visible: Option<bool>,
    },
    Group {
        items: Vec<LauncherMenuItem>,
    },
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
#[serde(default, rename_all = "camelCase")]
pub struct AppPreferences {
    pub start_with_windows: bool,
    pub open_menu_shortcut: OpenMenuShortcut,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenMenuShortcut {
    #[serde(rename = "type")]
    pub shortcut_type: ShortcutType,
    pub value: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutType {
    Keyboard,
    Mouse,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            open_menu_shortcut: OpenMenuShortcut {
                shortcut_type: ShortcutType::Keyboard,
                value: DEFAULT_SHORTCUT.into(),
            },
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

fn empty_launcher_config() -> LauncherConfig {
    LauncherConfig {
        shortcut: DEFAULT_SHORTCUT.into(),
        radial_menu_size: default_radial_menu_size(),
        items: Vec::new(),
    }
}

fn default_radial_menu_size() -> u8 {
    MIN_RADIAL_MENU_SIZE
}

fn normalize_launcher_config(config: &mut LauncherConfig) -> Result<(), String> {
    config.radial_menu_size = config
        .radial_menu_size
        .clamp(MIN_RADIAL_MENU_SIZE, MAX_RADIAL_MENU_SIZE);
    normalize_action_urls(&mut config.items)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| config_error("Não foi possível preparar a configuração", error))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|error| config_error("Não foi possível salvar a configuração", error))
}

fn validate_launcher_config(config: &LauncherConfig) -> Result<(), String> {
    validate_menu_items(&config.items, 0, "O menu principal")
}

fn validate_menu_items(
    items: &[LauncherMenuItem],
    depth: usize,
    level_name: &str,
) -> Result<(), String> {
    if items.len() > MAX_MENU_ITEMS_PER_LEVEL {
        return Err(format!(
            "{level_name} aceita no máximo {MAX_MENU_ITEMS_PER_LEVEL} itens."
        ));
    }

    let mut ids = HashSet::new();
    for item in items {
        if item.id.trim().is_empty() || item.label.trim().is_empty() || item.icon.trim().is_empty()
        {
            return Err("Cada item precisa ter id, label e ícone.".into());
        }
        if !ids.insert(&item.id) {
            return Err(format!("O id '{}' está duplicado.", item.id));
        }

        if let LauncherAction::Stream {
            provider,
            operation,
            scene_name,
            input_name,
            source_name,
            muted,
            visible,
        } = &item.action
        {
            crate::stream::validate_stream_action(&crate::stream::StreamAction {
                provider: provider.clone(),
                operation: operation.clone(),
                scene_name: scene_name.clone(),
                input_name: input_name.clone(),
                source_name: source_name.clone(),
                muted: *muted,
                visible: *visible,
            })?;
        }

        if let LauncherAction::WindowsApp {
            app_user_model_id, ..
        } = &item.action
        {
            if app_user_model_id.trim().is_empty() {
                return Err("Cada aplicativo do Windows precisa ter um identificador.".into());
            }
        }

        if let LauncherAction::Group { items } = &item.action {
            if depth >= MAX_GROUP_DEPTH {
                return Err("Limite de profundidade de grupos atingido.".into());
            }
            validate_menu_items(items, depth + 1, &format!("O grupo '{}'", item.label))?;
        }
    }
    Ok(())
}

pub fn load_launcher_config_internal(
    app: &tauri::AppHandle,
) -> Result<LauncherConfigResponse, String> {
    let path = launcher_config_path(app)?;
    if !path.exists() {
        let config = empty_launcher_config();
        write_json(&path, &config)?;
        return Ok(LauncherConfigResponse {
            config,
            warning: None,
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| config_error("Não foi possível ler a configuração salva", error))?;
    match serde_json::from_str::<LauncherConfig>(&contents) {
        Ok(mut config) if config.items.len() > MAX_MENU_ITEMS_PER_LEVEL => {
            config.items.truncate(MAX_MENU_ITEMS_PER_LEVEL);
            normalize_launcher_config(&mut config)?;
            validate_launcher_config(&config)?;
            Ok(LauncherConfigResponse {
                config,
                warning: Some(format!(
                    "A configuração salva excede o limite atual. Os primeiros \
                     {MAX_MENU_ITEMS_PER_LEVEL} itens foram carregados; salve para confirmar."
                )),
            })
        }
        Ok(mut config) => {
            normalize_launcher_config(&mut config)?;
            match validate_launcher_config(&config) {
                Ok(()) => Ok(LauncherConfigResponse {
                    config,
                    warning: None,
                }),
                Err(error) => {
                    let config = empty_launcher_config();
                    write_json(&path, &config)?;
                    Ok(LauncherConfigResponse {
                        config,
                        warning: Some(format!(
                        "O arquivo salvo estava inválido e uma configuração vazia foi carregada: {error}"
                        )),
                    })
                }
            }
        }
        Err(error) => {
            let config = empty_launcher_config();
            write_json(&path, &config)?;
            Ok(LauncherConfigResponse {
                config,
                warning: Some(format!(
                    "O arquivo salvo estava inválido e uma configuração vazia foi carregada: {error}"
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
    let mut config = config;
    normalize_launcher_config(&mut config)?;
    validate_launcher_config(&config)?;
    write_json(&launcher_config_path(&app)?, &config)?;
    app.emit(CONFIG_UPDATED_EVENT, config).map_err(|error| {
        config_error(
            "A configuração foi salva, mas o menu não foi atualizado",
            error,
        )
    })
}

fn normalize_action_urls(items: &mut [LauncherMenuItem]) -> Result<(), String> {
    for item in items {
        match &mut item.action {
            LauncherAction::Url { url } => {
                *url = crate::commands::normalize_http_url(url)?;
            }
            LauncherAction::Group { items } => normalize_action_urls(items)?,
            _ => {}
        }
    }
    Ok(())
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
    load_preferences_internal(&app)
}

#[tauri::command]
pub fn save_app_preferences(
    app: tauri::AppHandle,
    preferences: AppPreferences,
) -> Result<(), String> {
    let previous = load_preferences_internal(&app)?.preferences;
    let previous_shortcut = previous.open_menu_shortcut;
    let next_shortcut = normalize_shortcut(&preferences.open_menu_shortcut)?;

    if previous_shortcut != next_shortcut {
        apply_shortcut(&app, &previous_shortcut, &next_shortcut)?;
    }

    let mut preferences = preferences;
    preferences.open_menu_shortcut = next_shortcut.clone();
    if let Err(error) = write_json(&preferences_path(&app)?, &preferences) {
        if previous_shortcut != next_shortcut {
            let _ = apply_shortcut(&app, &next_shortcut, &previous_shortcut);
        }
        return Err(error);
    }

    if previous_shortcut != next_shortcut {
        if let Err(error) = app.emit(SHORTCUT_UPDATED_EVENT, next_shortcut.value) {
            eprintln!("O atalho foi salvo, mas a tela não foi atualizada: {error}");
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    if let Err(error) = result {
        if enabled || !is_missing_autostart_error(&error) {
            return Err(config_error(
                "Não foi possível alterar a inicialização com o sistema",
                error,
            ));
        }
    }

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

fn is_missing_autostart_error(error: &impl std::fmt::Display) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("(os error 2)")
        || message.contains("cannot find the file")
        || message.contains("não pode encontrar o arquivo")
        || message.contains("no such file or directory")
}

fn replace_global_shortcut(
    app: &tauri::AppHandle,
    previous: &str,
    next: &str,
) -> Result<(), String> {
    let state = app.state::<ShortcutRegistrationState>();
    let mut registered = state
        .0
        .lock()
        .map_err(|_| "Não foi possível acessar o estado do atalho.".to_string())?;
    let active = if registered.is_empty() {
        previous
    } else {
        registered.as_str()
    };

    app.global_shortcut()
        .unregister(active)
        .map_err(|error| config_error("Não foi possível liberar o atalho atual", error))?;

    if let Err(error) = app.global_shortcut().register(next) {
        let _ = app.global_shortcut().register(active);
        return Err(format!(
            "Não foi possível usar o atalho '{next}'. Ele pode ser inválido ou estar em uso: {error}"
        ));
    }

    *registered = next.to_string();
    Ok(())
}

pub fn register_initial_shortcut(app: &tauri::AppHandle) -> Result<(), String> {
    let mut response = load_preferences_internal(app)?;
    let requested = response.preferences.open_menu_shortcut.clone();
    let empty_keyboard = OpenMenuShortcut {
        shortcut_type: ShortcutType::Keyboard,
        value: String::new(),
    };

    if let Err(error) = apply_shortcut(app, &empty_keyboard, &requested) {
        let fallback = OpenMenuShortcut {
            shortcut_type: ShortcutType::Keyboard,
            value: DEFAULT_SHORTCUT.into(),
        };
        apply_shortcut(app, &empty_keyboard, &fallback).map_err(|fallback_error| {
            format!(
                "O atalho salvo falhou ({error}) e Ctrl+Space também não pôde ser registrado: {fallback_error}"
            )
        })?;
        response.preferences.open_menu_shortcut = fallback;
        write_json(&preferences_path(app)?, &response.preferences)?;
        eprintln!("O atalho salvo era inválido; Ctrl+Space foi restaurado: {error}");
    }
    Ok(())
}

fn normalize_shortcut(shortcut: &OpenMenuShortcut) -> Result<OpenMenuShortcut, String> {
    match shortcut.shortcut_type {
        ShortcutType::Keyboard => {
            let value = shortcut.value.trim();
            if value.is_empty() {
                return Err("O atalho de teclado não pode ficar vazio.".into());
            }
            Ok(OpenMenuShortcut {
                shortcut_type: ShortcutType::Keyboard,
                value: value.into(),
            })
        }
        ShortcutType::Mouse => {
            let button = parse_mouse_button(&shortcut.value)?;
            Ok(OpenMenuShortcut {
                shortcut_type: ShortcutType::Mouse,
                value: format!("Mouse{button}"),
            })
        }
    }
}

fn parse_mouse_button(value: &str) -> Result<u8, String> {
    match value.trim() {
        "Mouse3" => Ok(3),
        "Mouse4" => Ok(4),
        "Mouse5" => Ok(5),
        _ => Err("Escolha Mouse3, Mouse4 ou Mouse5.".into()),
    }
}

fn apply_shortcut(
    app: &tauri::AppHandle,
    previous: &OpenMenuShortcut,
    next: &OpenMenuShortcut,
) -> Result<(), String> {
    match (&previous.shortcut_type, &next.shortcut_type) {
        (ShortcutType::Keyboard, ShortcutType::Keyboard) => {
            if previous.value.is_empty() {
                register_keyboard_shortcut(app, &next.value)
            } else {
                replace_global_shortcut(app, &previous.value, &next.value)
            }
        }
        (ShortcutType::Keyboard, ShortcutType::Mouse) => {
            app.state::<MouseShortcutManager>()
                .set_button(parse_mouse_button(&next.value)?)?;
            if let Err(error) = unregister_keyboard_shortcut(app, &previous.value) {
                app.state::<MouseShortcutManager>().disable();
                return Err(error);
            }
            Ok(())
        }
        (ShortcutType::Mouse, ShortcutType::Keyboard) => {
            register_keyboard_shortcut(app, &next.value)?;
            app.state::<MouseShortcutManager>().disable();
            Ok(())
        }
        (ShortcutType::Mouse, ShortcutType::Mouse) => app
            .state::<MouseShortcutManager>()
            .set_button(parse_mouse_button(&next.value)?),
    }
}

fn register_keyboard_shortcut(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    app.global_shortcut().register(shortcut).map_err(|error| {
        format!("Não foi possível usar o atalho '{shortcut}'. Ele pode estar em uso: {error}")
    })?;
    *app.state::<ShortcutRegistrationState>()
        .0
        .lock()
        .map_err(|_| "Não foi possível atualizar o estado do atalho.".to_string())? =
        shortcut.into();
    Ok(())
}

fn unregister_keyboard_shortcut(app: &tauri::AppHandle, fallback: &str) -> Result<(), String> {
    let state = app.state::<ShortcutRegistrationState>();
    let mut registered = state
        .0
        .lock()
        .map_err(|_| "Não foi possível acessar o estado do atalho.".to_string())?;
    let active = if registered.is_empty() {
        fallback
    } else {
        registered.as_str()
    };
    if !active.is_empty() {
        app.global_shortcut()
            .unregister(active)
            .map_err(|error| config_error("Não foi possível liberar o atalho de teclado", error))?;
    }
    registered.clear();
    Ok(())
}
