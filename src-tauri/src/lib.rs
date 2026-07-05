mod commands;
mod system_stats;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const GLOBAL_SHORTCUT: &str = "Ctrl+Space";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(system_stats::SystemStatsState::new())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    if let Some(window) = app.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(false);
                        let result = if is_visible {
                            commands::hide_window(&window)
                        } else {
                            commands::show_window_at_cursor(&window).map(|_| ())
                        };

                        if let Err(error) = result {
                            eprintln!("Falha ao alternar o menu: {error}");
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.global_shortcut().register(GLOBAL_SHORTCUT)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_cursor_position,
            commands::move_menu_to_cursor,
            commands::show_menu,
            commands::hide_menu,
            commands::open_program,
            commands::open_directory,
            system_stats::get_system_stats,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Orbit Launcher");
}
