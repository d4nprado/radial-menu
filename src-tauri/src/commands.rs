use crate::config::SystemActionTarget;
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

pub(crate) fn normalize_http_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
        || value.contains('\\')
    {
        return Err("URL inválida".into());
    }

    let lowercase = value.to_ascii_lowercase();
    let normalized = if lowercase.starts_with("http://") {
        format!("http://{}", &value["http://".len()..])
    } else if lowercase.starts_with("https://") {
        format!("https://{}", &value["https://".len()..])
    } else if value.contains("://") {
        return Err("URL inválida".into());
    } else {
        format!("https://{value}")
    };

    let authority_and_path = normalized
        .split_once("://")
        .map(|(_, remainder)| remainder)
        .ok_or_else(|| "URL inválida".to_string())?;
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();

    if authority.is_empty() || authority.contains('@') || !valid_http_authority(authority) {
        return Err("URL inválida".into());
    }

    Ok(normalized)
}

fn valid_http_authority(authority: &str) -> bool {
    if authority.starts_with('[') {
        let Some(ipv6_end) = authority.find(']') else {
            return false;
        };
        let address = &authority[1..ipv6_end];
        let suffix = &authority[ipv6_end + 1..];
        return !address.is_empty()
            && address.contains(':')
            && (suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_port));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if port.is_some_and(|value| !valid_port(value)) {
        return false;
    }

    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && label
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
                && label
                    .chars()
                    .last()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
        })
}

fn valid_port(port: &str) -> bool {
    !port.is_empty() && port.parse::<u16>().is_ok()
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
    open_directory_path(&path)
}

pub fn open_directory_path(path: &Path) -> Result<(), String> {
    open_directory_native(path)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    let url = normalize_http_url(&url)?;
    open_uri_native(&url)
}

#[tauri::command]
pub fn execute_system_action(target: SystemActionTarget) -> Result<(), String> {
    execute_system_action_native(target)
}

#[cfg(target_os = "windows")]
fn execute_system_action_native(target: SystemActionTarget) -> Result<(), String> {
    match target {
        SystemActionTarget::Explorer => {
            spawn_fixed_program("explorer.exe", "o Explorador de Arquivos")
        }
        SystemActionTarget::DefaultBrowser => {
            Err("A ação Navegador padrão foi substituída por Abrir URL. \
             Edite este item e escolha uma URL."
                .into())
        }
        SystemActionTarget::Terminal => Command::new("wt.exe")
            .spawn()
            .or_else(|_| Command::new("cmd.exe").spawn())
            .map(|_| ())
            .map_err(|error| command_error("Não foi possível abrir o terminal", error)),
        SystemActionTarget::Calculator => spawn_fixed_program("calc.exe", "a Calculadora"),
        SystemActionTarget::Notepad => spawn_fixed_program("notepad.exe", "o Bloco de Notas"),
    }
}

#[cfg(not(target_os = "windows"))]
fn execute_system_action_native(_target: SystemActionTarget) -> Result<(), String> {
    Err("Ações padrão do sistema estão disponíveis apenas no Windows.".into())
}

#[cfg(target_os = "windows")]
fn spawn_fixed_program(program: &str, label: &str) -> Result<(), String> {
    Command::new(program)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error(&format!("Não foi possível abrir {label}"), error))
}

#[cfg(target_os = "windows")]
fn open_uri_native(uri: &str) -> Result<(), String> {
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", uri])
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir o navegador padrão", error))
}

#[cfg(test)]
mod tests {
    use super::normalize_http_url;

    #[test]
    fn normalizes_url_without_scheme() {
        assert_eq!(
            normalize_http_url("github.com").unwrap(),
            "https://github.com"
        );
    }

    #[test]
    fn preserves_http_urls_and_paths() {
        assert_eq!(
            normalize_http_url("http://localhost:3000/docs").unwrap(),
            "http://localhost:3000/docs"
        );
    }

    #[test]
    fn rejects_unsupported_or_malformed_urls() {
        assert_eq!(
            normalize_http_url("ftp://example.com").unwrap_err(),
            "URL inválida"
        );
        assert_eq!(normalize_http_url("https://").unwrap_err(), "URL inválida");
        assert_eq!(
            normalize_http_url("https://example.com/a b").unwrap_err(),
            "URL inválida"
        );
    }
}

#[cfg(target_os = "macos")]
fn open_uri_native(uri: &str) -> Result<(), String> {
    Command::new("open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir a URL", error))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_uri_native(uri: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| command_error("Não foi possível abrir a URL", error))
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
