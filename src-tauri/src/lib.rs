mod commands;
mod config;
mod mouse_shortcut;
mod stream;
mod system_stats;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::ShortcutState;

const MAIN_WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const CONFIGURE_MENU_ID: &str = "configure";
const OPEN_MENU_ID: &str = "open";
const EXIT_MENU_ID: &str = "exit";

fn show_settings(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or_else(|| "A janela de configuração não foi encontrada.".to_string())?;

    window
        .show()
        .map_err(|error| format!("Não foi possível exibir a configuração: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("Não foi possível focar a configuração: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(system_stats::SystemStatsState::new())
        .manage(config::ShortcutRegistrationState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
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
            let mouse_manager =
                match mouse_shortcut::MouseShortcutManager::start(app.handle().clone()) {
                    Ok(manager) => manager,
                    Err(error) => {
                        eprintln!("Listener global do mouse indisponível: {error}");
                        mouse_shortcut::MouseShortcutManager::unavailable(error)
                    }
                };
            let _ = app.manage(mouse_manager);
            config::register_initial_shortcut(app.handle()).map_err(std::io::Error::other)?;

            let configure_item = MenuItem::with_id(
                app,
                CONFIGURE_MENU_ID,
                "Configurar launcher",
                true,
                None::<&str>,
            )?;
            let open_item =
                MenuItem::with_id(app, OPEN_MENU_ID, "Abrir menu radial", true, None::<&str>)?;
            let exit_item = MenuItem::with_id(app, EXIT_MENU_ID, "Sair", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&configure_item, &open_item, &exit_item])?;

            let mut tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Orbit Launcher")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    CONFIGURE_MENU_ID => {
                        if let Err(error) = show_settings(app) {
                            eprintln!("{error}");
                        }
                    }
                    OPEN_MENU_ID => {
                        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                            if let Err(error) = commands::show_window_at_cursor(&window) {
                                eprintln!("Falha ao abrir o menu radial: {error}");
                            }
                        }
                    }
                    EXIT_MENU_ID => app.exit(0),
                    _ => {}
                });

            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }

            tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == SETTINGS_WINDOW_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("Falha ao esconder a configuração: {error}");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_cursor_position,
            commands::move_menu_to_cursor,
            commands::show_menu,
            commands::hide_menu,
            commands::hide_settings,
            commands::open_program,
            commands::open_directory,
            commands::extract_program_icon,
            commands::open_url,
            commands::execute_system_action,
            config::load_launcher_config,
            config::save_launcher_config,
            config::get_config_path,
            config::open_config_directory,
            config::get_app_preferences,
            config::save_app_preferences,
            config::set_autostart_enabled,
            config::get_autostart_enabled,
            stream::get_stream_preferences,
            stream::save_stream_preferences,
            stream::test_obs_connection,
            stream::list_obs_scenes,
            stream::execute_stream_action,
            mouse_shortcut::start_mouse_shortcut_capture,
            mouse_shortcut::cancel_mouse_shortcut_capture,
            system_stats::get_system_stats,
        ])
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Orbit Launcher");

    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            app.state::<mouse_shortcut::MouseShortcutManager>().stop();
        }
    });
}
