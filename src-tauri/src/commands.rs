use crate::config::SystemActionTarget;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{Emitter, Manager, PhysicalPosition, WebviewWindow};
#[cfg(target_os = "windows")]
use windows::{
    core::PCWSTR,
    Win32::{
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
        },
        UI::{
            Shell::ExtractIconExW,
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON},
        },
    },
};

const WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPosition {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramIcon {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
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
pub fn extract_program_icon(path: String) -> Result<Option<ProgramIcon>, String> {
    let path = validated_path(&path)?;
    if !path.is_file() {
        return Ok(None);
    }
    extract_program_icon_native(&path)
}

#[cfg(target_os = "windows")]
fn extract_program_icon_native(path: &Path) -> Result<Option<ProgramIcon>, String> {
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut icon = HICON::default();
    let extracted =
        unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), 0, Some(&mut icon), None, 1) };
    if extracted == 0 || icon.0.is_null() {
        return Ok(None);
    }

    let result = icon_to_rgba(icon);
    unsafe {
        let _ = DestroyIcon(icon);
    }
    result.map(Some)
}

#[cfg(target_os = "windows")]
fn icon_to_rgba(icon: HICON) -> Result<ProgramIcon, String> {
    const ICON_SIZE: i32 = 48;
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return Err("Não foi possível preparar o ícone do programa.".into());
    }

    let mut bitmap_info = BITMAPINFO::default();
    bitmap_info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: ICON_SIZE,
        biHeight: -ICON_SIZE,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };

    let mut pixels = std::ptr::null_mut();
    let bitmap =
        unsafe { CreateDIBSection(Some(dc), &bitmap_info, DIB_RGB_COLORS, &mut pixels, None, 0) }
            .map_err(|error| {
            unsafe {
                let _ = DeleteDC(dc);
            }
            command_error("Não foi possível criar a imagem do ícone", error)
        })?;

    let old_bitmap = unsafe { SelectObject(dc, HGDIOBJ(bitmap.0)) };
    let draw_result =
        unsafe { DrawIconEx(dc, 0, 0, icon, ICON_SIZE, ICON_SIZE, 0, None, DI_NORMAL) };

    let byte_count = (ICON_SIZE * ICON_SIZE * 4) as usize;
    let mut rgba = if draw_result.is_ok() && !pixels.is_null() {
        unsafe { std::slice::from_raw_parts(pixels.cast::<u8>(), byte_count) }.to_vec()
    } else {
        Vec::new()
    };

    unsafe {
        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
    }

    draw_result.map_err(|error| command_error("Não foi possível desenhar o ícone", error))?;
    if rgba.len() != byte_count {
        return Err("O Windows retornou um ícone vazio.".into());
    }

    let has_alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] != 0);
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        if !has_alpha && pixel[..3].iter().any(|channel| *channel != 0) {
            pixel[3] = 255;
        } else if has_alpha && pixel[3] > 0 && pixel[3] < 255 {
            let alpha = pixel[3] as u16;
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u16 * 255) / alpha).min(255) as u8;
            }
        }
    }

    Ok(ProgramIcon {
        width: ICON_SIZE as u32,
        height: ICON_SIZE as u32,
        rgba,
    })
}

#[cfg(not(target_os = "windows"))]
fn extract_program_icon_native(_path: &Path) -> Result<Option<ProgramIcon>, String> {
    Ok(None)
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
    #[cfg(target_os = "windows")]
    use super::extract_program_icon_native;
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

    #[test]
    #[cfg(target_os = "windows")]
    fn extracts_rgba_icon_from_windows_executable() {
        let executable = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap()
            .join("explorer.exe");
        let icon = extract_program_icon_native(&executable)
            .unwrap()
            .expect("explorer.exe should provide an icon");

        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] != 0));
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
