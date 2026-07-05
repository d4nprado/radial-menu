use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{Emitter, Manager, PhysicalPosition, WebviewWindow};

const WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPosition {
    x: f64,
    y: f64,
}

fn command_error(context: &str, error: impl std::fmt::Display) -> String {
    format!("{context}: {error}")
}

fn validated_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("O caminho precisa ser absoluto.".into());
    }
    if !path.exists() {
        return Err(format!("O caminho não existe: {}", path.display()));
    }
    Ok(path)
}

fn main_window(app: &tauri::AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or_else(|| "A janela principal não foi encontrada.".into())
}

fn settings_window(app: &tauri::AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or_else(|| "A janela de configuração não foi encontrada.".into())
}

pub fn position_window_at_cursor(window: &WebviewWindow) -> Result<CursorPosition, String> {
    let cursor = window
        .cursor_position()
        .map_err(|error| command_error("Não foi possível obter a posição do cursor", error))?;
    let window_size = window
        .outer_size()
        .map_err(|error| command_error("Não foi possível obter o tamanho da janela", error))?;

    let monitor = window
        .monitor_from_point(cursor.x, cursor.y)
        .map_err(|error| command_error("Não foi possível localizar o monitor", error))?;

    let mut x = cursor.x.round() as i32 - window_size.width as i32 / 2;
    let mut y = cursor.y.round() as i32 - window_size.height as i32 / 2;

    if let Some(monitor) = monitor {
        let origin = monitor.position();
        let size = monitor.size();
        let max_x = origin.x + size.width as i32 - window_size.width as i32;
        let max_y = origin.y + size.height as i32 - window_size.height as i32;
        x = x.clamp(origin.x, max_x.max(origin.x));
        y = y.clamp(origin.y, max_y.max(origin.y));
    }

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| command_error("Não foi possível mover a janela", error))?;

    Ok(CursorPosition {
        x: cursor.x,
        y: cursor.y,
    })
}

pub fn show_window_at_cursor(window: &WebviewWindow) -> Result<CursorPosition, String> {
    let cursor = position_window_at_cursor(window)?;
    window
        .show()
        .map_err(|error| command_error("Não foi possível exibir o menu", error))?;
    window
        .set_focus()
        .map_err(|error| command_error("Não foi possível focar o menu", error))?;
    window
        .emit("menu:show", ())
        .map_err(|error| command_error("Não foi possível iniciar a animação", error))?;
    Ok(cursor)
}

pub fn hide_window(window: &WebviewWindow) -> Result<(), String> {
    window
        .emit("menu:hide", ())
        .map_err(|error| command_error("Não foi possível atualizar o menu", error))?;
    window
        .hide()
        .map_err(|error| command_error("Não foi possível esconder o menu", error))
}

#[tauri::command]
pub fn get_cursor_position(window: WebviewWindow) -> Result<CursorPosition, String> {
    let cursor = window
        .cursor_position()
        .map_err(|error| command_error("Não foi possível obter a posição do cursor", error))?;
    Ok(CursorPosition {
        x: cursor.x,
        y: cursor.y,
    })
}

#[tauri::command]
pub fn move_menu_to_cursor(window: WebviewWindow) -> Result<CursorPosition, String> {
    position_window_at_cursor(&window)
}

#[tauri::command]
pub fn show_menu(app: tauri::AppHandle) -> Result<CursorPosition, String> {
    show_window_at_cursor(&main_window(&app)?)
}

#[tauri::command]
pub fn hide_menu(app: tauri::AppHandle) -> Result<(), String> {
    hide_window(&main_window(&app)?)
}

#[tauri::command]
pub fn hide_settings(app: tauri::AppHandle) -> Result<(), String> {
    settings_window(&app)?
        .hide()
        .map_err(|error| command_error("Não foi possível esconder a configuração", error))
}

#[tauri::command]
pub fn open_program(path: String) -> Result<(), String> {
    let path = validated_path(&path)?;
    if !path.is_file() {
        return Err(format!("O programa não é um arquivo: {}", path.display()));
    }

    Command::new(&path)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir o programa", error))
}

#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    let path = validated_path(&path)?;
    if !path.is_dir() {
        return Err(format!("O caminho não é um diretório: {}", path.display()));
    }
    open_directory_native(&path)
}

#[cfg(target_os = "windows")]
fn open_directory_native(path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir o diretório", error))
}

#[cfg(target_os = "macos")]
fn open_directory_native(path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir o diretório", error))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_directory_native(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir o diretório", error))
}
