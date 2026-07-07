use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    net::{TcpStream, ToSocketAddrs},
    path::PathBuf,
    sync::{Mutex, TryLockError},
    time::{Duration, Instant, SystemTime},
};
use tauri::Manager;
use tungstenite::{
    client as websocket_client, connect, stream::MaybeTlsStream, Message, WebSocket,
};

const STREAM_PREFERENCES_FILE: &str = "stream-preferences.json";
const OBS_IDLE_TIMEOUT: Duration = Duration::from_secs(180);
const OBS_STATUS_CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const OBS_STATUS_INITIAL_COOLDOWN: Duration = Duration::from_secs(10);
const OBS_STATUS_MAX_COOLDOWN: Duration = Duration::from_secs(30);

type ObsSocket = WebSocket<MaybeTlsStream<TcpStream>>;

pub struct ObsClientState {
    client: Mutex<Option<ObsClient>>,
    status: Mutex<ObsStatusState>,
}

struct ObsClient {
    preferences: ObsPreferences,
    socket: ObsSocket,
    last_used: Instant,
}

#[derive(Default)]
struct ObsStatusState {
    cached: Option<CachedObsStatus>,
    cooldown_until: Option<Instant>,
    consecutive_failures: u32,
}

struct CachedObsStatus {
    status: ObsStreamStatus,
}

impl ObsClientState {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(None),
            status: Mutex::new(ObsStatusState::default()),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPreferences {
    pub obs: ObsPreferences,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsPreferences {
    pub host: String,
    pub port: u16,
    pub password: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamAction {
    pub provider: StreamProvider,
    pub operation: StreamOperation,
    #[serde(default)]
    pub scene_name: String,
    #[serde(default)]
    pub input_name: String,
    #[serde(default)]
    pub source_name: String,
    #[serde(default)]
    pub muted: Option<bool>,
    #[serde(default)]
    pub visible: Option<bool>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamProvider {
    Obs,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamOperation {
    SetScene,
    StartRecording,
    StopRecording,
    ToggleRecording,
    StartStreaming,
    StopStreaming,
    ToggleStreaming,
    SetInputMute,
    ToggleInputMute,
    SetSourceVisibility,
    ToggleSourceVisibility,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPreferencesResponse {
    pub preferences: StreamPreferences,
    pub warning: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsConnectionStatus {
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsOutputStatus {
    pub active: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsStreamStatus {
    pub recording: ObsOutputStatus,
    pub streaming: ObsOutputStatus,
    pub input_mutes: HashMap<String, bool>,
    pub source_visibilities: HashMap<String, bool>,
    pub available: bool,
    pub stale: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsSourceStatusTarget {
    pub scene_name: String,
    pub source_name: String,
}

impl Default for StreamPreferences {
    fn default() -> Self {
        Self {
            obs: ObsPreferences {
                host: "127.0.0.1".into(),
                port: 4455,
                password: String::new(),
            },
        }
    }
}

fn stream_error(context: &str, error: impl std::fmt::Display) -> String {
    format!("{context}: {error}")
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(|error| {
        stream_error("Nao foi possivel localizar o AppData do aplicativo", error)
    })?;
    fs::create_dir_all(&directory)
        .map_err(|error| stream_error("Nao foi possivel criar a pasta de configuracao", error))?;
    Ok(directory)
}

fn preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(STREAM_PREFERENCES_FILE))
}

fn write_preferences(
    app: &tauri::AppHandle,
    preferences: &StreamPreferences,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(preferences)
        .map_err(|error| stream_error("Nao foi possivel preparar as preferencias Stream", error))?;
    fs::write(preferences_path(app)?, format!("{json}\n"))
        .map_err(|error| stream_error("Nao foi possivel salvar as preferencias Stream", error))
}

pub fn load_stream_preferences_internal(
    app: &tauri::AppHandle,
) -> Result<StreamPreferencesResponse, String> {
    let path = preferences_path(app)?;
    if !path.exists() {
        let preferences = StreamPreferences::default();
        write_preferences(app, &preferences)?;
        return Ok(StreamPreferencesResponse {
            preferences,
            warning: None,
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| stream_error("Nao foi possivel ler as preferencias Stream", error))?;
    match serde_json::from_str::<StreamPreferences>(&contents) {
        Ok(preferences) => Ok(StreamPreferencesResponse {
            preferences: normalize_preferences(preferences),
            warning: None,
        }),
        Err(error) => {
            let preferences = StreamPreferences::default();
            write_preferences(app, &preferences)?;
            Ok(StreamPreferencesResponse {
                preferences,
                warning: Some(format!(
                    "As preferencias Stream estavam invalidas e foram restauradas: {error}"
                )),
            })
        }
    }
}

fn normalize_preferences(mut preferences: StreamPreferences) -> StreamPreferences {
    preferences.obs.host = preferences.obs.host.trim().to_string();
    preferences.obs.password = preferences.obs.password.trim().to_string();
    if preferences.obs.host.is_empty() {
        preferences.obs.host = "127.0.0.1".into();
    }
    if preferences.obs.port == 0 {
        preferences.obs.port = 4455;
    }
    preferences
}

#[tauri::command]
pub fn get_stream_preferences(app: tauri::AppHandle) -> Result<StreamPreferencesResponse, String> {
    load_stream_preferences_internal(&app)
}

#[tauri::command]
pub fn save_stream_preferences(
    app: tauri::AppHandle,
    preferences: StreamPreferences,
) -> Result<(), String> {
    write_preferences(&app, &normalize_preferences(preferences))?;
    invalidate_obs_client(&app);
    Ok(())
}

#[tauri::command]
pub fn test_obs_connection(app: tauri::AppHandle) -> Result<ObsConnectionStatus, String> {
    with_obs_connection(&app, |socket| {
        send_obs_request(socket, "GetVersion", json!({}))?;
        Ok(())
    })?;
    Ok(ObsConnectionStatus {
        ok: true,
        message: "Conexao com OBS Studio realizada.".into(),
    })
}

#[tauri::command]
pub fn list_obs_scenes(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    with_obs_connection(&app, |socket| {
        let response = send_obs_request(socket, "GetSceneList", json!({}))?;
        let scenes = response
            .get("scenes")
            .and_then(Value::as_array)
            .ok_or_else(|| "O OBS nao retornou a lista de cenas.".to_string())?;

        Ok(scenes
            .iter()
            .filter_map(|scene| scene.get("sceneName").and_then(Value::as_str))
            .map(str::to_string)
            .collect())
    })
}

#[tauri::command]
pub fn list_obs_inputs(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    with_obs_connection(&app, |socket| {
        let response = send_obs_request(socket, "GetInputList", json!({}))?;
        let inputs = response
            .get("inputs")
            .and_then(Value::as_array)
            .ok_or_else(|| "O OBS nao retornou a lista de inputs.".to_string())?;

        Ok(inputs
            .iter()
            .filter_map(|input| input.get("inputName").and_then(Value::as_str))
            .map(str::to_string)
            .collect())
    })
}

#[tauri::command]
pub fn list_obs_sources_for_scene(
    app: tauri::AppHandle,
    scene_name: String,
) -> Result<Vec<String>, String> {
    let scene_name = scene_name.trim();
    if scene_name.is_empty() {
        return Err("Escolha uma cena do OBS para carregar as fontes.".into());
    }

    with_obs_connection(&app, |socket| list_sources_for_scene(socket, scene_name))
}

#[tauri::command]
pub fn get_obs_stream_status(app: tauri::AppHandle) -> Result<ObsStreamStatus, String> {
    if let Some(status) = status_cooldown_response(&app) {
        return Ok(status);
    }

    let result = with_obs_status_connection(&app, |socket| {
        Ok(ObsStreamStatus {
            recording: ObsOutputStatus {
                active: output_is_active(socket, "GetRecordStatus")?,
            },
            streaming: ObsOutputStatus {
                active: output_is_active(socket, "GetStreamStatus")?,
            },
            input_mutes: HashMap::new(),
            source_visibilities: HashMap::new(),
            available: true,
            stale: false,
        })
    });

    match result {
        Ok(status) => {
            remember_status_success(&app, status.clone());
            eprintln!("Atualizacao de status OBS concluida.");
            Ok(status)
        }
        Err(ObsStatusAttemptError::Busy) => Ok(cached_status_or_empty(&app, true)),
        Err(ObsStatusAttemptError::Failed(error)) => {
            remember_status_failure(&app);
            eprintln!("Atualizacao de status OBS falhou: {error}");
            Ok(cached_status_or_empty(&app, true))
        }
    }
}

#[tauri::command]
pub fn get_obs_input_mute_statuses(
    app: tauri::AppHandle,
    input_names: Vec<String>,
) -> Result<HashMap<String, bool>, String> {
    if is_status_in_cooldown(&app) {
        eprintln!("Pulando consulta OBS de audio durante cooldown.");
        return Ok(cached_input_mutes(&app));
    }

    let result = with_obs_status_connection(&app, |socket| {
        let mut statuses = HashMap::new();

        for input_name in input_names {
            let input_name = input_name.trim();
            if input_name.is_empty() || statuses.contains_key(input_name) {
                continue;
            }
            match input_is_muted(socket, input_name) {
                Ok(muted) => {
                    statuses.insert(input_name.to_string(), muted);
                }
                Err(error) if is_obs_connection_error(&error) => return Err(error),
                Err(_) => {}
            }
        }

        Ok(statuses)
    });

    match result {
        Ok(statuses) => {
            remember_input_mute_success(&app, &statuses);
            eprintln!("Atualizacao de status OBS de audio concluida.");
            Ok(statuses)
        }
        Err(ObsStatusAttemptError::Busy) => Ok(cached_input_mutes(&app)),
        Err(ObsStatusAttemptError::Failed(error)) => {
            remember_status_failure(&app);
            eprintln!("Atualizacao de status OBS de audio falhou: {error}");
            Ok(cached_input_mutes(&app))
        }
    }
}

#[tauri::command]
pub fn get_obs_source_visibility_statuses(
    app: tauri::AppHandle,
    sources: Vec<ObsSourceStatusTarget>,
) -> Result<HashMap<String, bool>, String> {
    if is_status_in_cooldown(&app) {
        eprintln!("Pulando consulta OBS de fontes durante cooldown.");
        return Ok(cached_source_visibilities(&app));
    }

    let result = with_obs_status_connection(&app, |socket| {
        let mut statuses = HashMap::new();

        for source in sources {
            let scene_name = source.scene_name.trim();
            let source_name = source.source_name.trim();
            let key = source_visibility_key(scene_name, source_name);
            if scene_name.is_empty() || source_name.is_empty() || statuses.contains_key(&key) {
                continue;
            }

            match scene_item_id(socket, scene_name, source_name)
                .and_then(|scene_item_id| scene_item_is_enabled(socket, scene_name, scene_item_id))
            {
                Ok(visible) => {
                    statuses.insert(key, visible);
                }
                Err(error) if is_obs_connection_error(&error) => return Err(error),
                Err(_) => {}
            }
        }

        Ok(statuses)
    });

    match result {
        Ok(statuses) => {
            remember_source_visibility_success(&app, &statuses);
            eprintln!("Atualizacao de status OBS de fontes concluida.");
            Ok(statuses)
        }
        Err(ObsStatusAttemptError::Busy) => Ok(cached_source_visibilities(&app)),
        Err(ObsStatusAttemptError::Failed(error)) => {
            remember_status_failure(&app);
            eprintln!("Atualizacao de status OBS de fontes falhou: {error}");
            Ok(cached_source_visibilities(&app))
        }
    }
}

#[tauri::command]
pub fn execute_stream_action(app: tauri::AppHandle, action: StreamAction) -> Result<(), String> {
    validate_stream_action(&action)?;
    let result = with_obs_connection(&app, |socket| match (&action.provider, &action.operation) {
        (StreamProvider::Obs, StreamOperation::SetScene) => {
            let scene_name = action.scene_name.trim();
            send_obs_request(
                socket,
                "SetCurrentProgramScene",
                json!({ "sceneName": scene_name }),
            )?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StartRecording) => {
            ensure_output_inactive(socket, "GetRecordStatus", "A gravacao ja esta ativa.")?;
            send_obs_request(socket, "StartRecord", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StopRecording) => {
            ensure_output_active(socket, "GetRecordStatus", "A gravacao ja esta inativa.")?;
            send_obs_request(socket, "StopRecord", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleRecording) => {
            if output_is_active(socket, "GetRecordStatus")? {
                send_obs_request(socket, "StopRecord", json!({}))?;
            } else {
                send_obs_request(socket, "StartRecord", json!({}))?;
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StartStreaming) => {
            ensure_output_inactive(socket, "GetStreamStatus", "A transmissao ja esta ativa.")?;
            send_obs_request(socket, "StartStream", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StopStreaming) => {
            ensure_output_active(socket, "GetStreamStatus", "A transmissao ja esta inativa.")?;
            send_obs_request(socket, "StopStream", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleStreaming) => {
            if output_is_active(socket, "GetStreamStatus")? {
                send_obs_request(socket, "StopStream", json!({}))?;
            } else {
                send_obs_request(socket, "StartStream", json!({}))?;
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::SetInputMute) => {
            send_obs_request(
                socket,
                "SetInputMute",
                json!({
                    "inputName": action.input_name.trim(),
                    "inputMuted": action.muted.unwrap_or(false),
                }),
            )?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleInputMute) => {
            let input_name = action.input_name.trim();
            let muted = input_is_muted(socket, input_name)?;
            send_obs_request(
                socket,
                "SetInputMute",
                json!({ "inputName": input_name, "inputMuted": !muted }),
            )?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::SetSourceVisibility) => {
            let scene_name = action.scene_name.trim();
            let source_name = action.source_name.trim();
            let scene_item_id = scene_item_id(socket, scene_name, source_name)?;
            send_obs_request(
                socket,
                "SetSceneItemEnabled",
                json!({
                    "sceneName": scene_name,
                    "sceneItemId": scene_item_id,
                    "sceneItemEnabled": action.visible.unwrap_or(false),
                }),
            )?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleSourceVisibility) => {
            let scene_name = action.scene_name.trim();
            let source_name = action.source_name.trim();
            let scene_item_id = scene_item_id(socket, scene_name, source_name)?;
            let visible = scene_item_is_enabled(socket, scene_name, scene_item_id)?;
            send_obs_request(
                socket,
                "SetSceneItemEnabled",
                json!({
                    "sceneName": scene_name,
                    "sceneItemId": scene_item_id,
                    "sceneItemEnabled": !visible,
                }),
            )?;
            Ok(())
        }
    });

    result.map_err(|error| {
        if is_obs_connection_error(&error) || error.contains("Nao foi possivel conectar ao OBS") {
            "Nao foi possivel conectar ao OBS. Verifique se o OBS esta aberto e o WebSocket esta ativo."
                .into()
        } else {
            error
        }
    })
}

pub fn validate_stream_action(action: &StreamAction) -> Result<(), String> {
    match (&action.provider, &action.operation) {
        (StreamProvider::Obs, StreamOperation::SetScene) => {
            if action.scene_name.trim().is_empty() {
                return Err("A acao Stream precisa de uma cena do OBS.".into());
            }
            Ok(())
        }
        (
            StreamProvider::Obs,
            StreamOperation::StartRecording
            | StreamOperation::StopRecording
            | StreamOperation::ToggleRecording
            | StreamOperation::StartStreaming
            | StreamOperation::StopStreaming
            | StreamOperation::ToggleStreaming,
        ) => Ok(()),
        (StreamProvider::Obs, StreamOperation::SetInputMute) => {
            if action.input_name.trim().is_empty() {
                return Err("A acao Stream precisa de um input de audio.".into());
            }
            if action.muted.is_none() {
                return Err("A acao Stream precisa informar se o input sera mutado.".into());
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleInputMute) => {
            if action.input_name.trim().is_empty() {
                return Err("A acao Stream precisa de um input de audio.".into());
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::SetSourceVisibility) => {
            if action.scene_name.trim().is_empty() {
                return Err("A acao Stream precisa de uma cena do OBS.".into());
            }
            if action.source_name.trim().is_empty() {
                return Err("A acao Stream precisa de uma fonte do OBS.".into());
            }
            if action.visible.is_none() {
                return Err("A acao Stream precisa informar a visibilidade da fonte.".into());
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleSourceVisibility) => {
            if action.scene_name.trim().is_empty() {
                return Err("A acao Stream precisa de uma cena do OBS.".into());
            }
            if action.source_name.trim().is_empty() {
                return Err("A acao Stream precisa de uma fonte do OBS.".into());
            }
            Ok(())
        }
    }
}

fn invalidate_obs_client(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<ObsClientState>() else {
        return;
    };
    let Ok(mut client) = state.client.lock() else {
        eprintln!("Nao foi possivel bloquear o cliente OBS para invalidacao.");
        return;
    };

    if client.take().is_some() {
        eprintln!("Preferencias Stream alteradas; a conexao OBS sera recriada na proxima chamada.");
    }
}

fn with_obs_connection<T>(
    app: &tauri::AppHandle,
    operation: impl FnOnce(&mut ObsSocket) -> Result<T, String>,
) -> Result<T, String> {
    let preferences = load_stream_preferences_internal(app)?.preferences.obs;
    let state = app.state::<ObsClientState>();
    let mut client = state
        .client
        .lock()
        .map_err(|_| "Nao foi possivel acessar o cliente OBS.".to_string())?;

    let should_reconnect = match client.as_ref() {
        Some(current) if current.preferences != preferences => {
            eprintln!("Configuracao OBS alterada; reconectando ao OBS WebSocket.");
            true
        }
        Some(current) if current.last_used.elapsed() > OBS_IDLE_TIMEOUT => {
            eprintln!("Conexao OBS ociosa; reconectando ao OBS WebSocket.");
            true
        }
        Some(_) => false,
        None => true,
    };

    if should_reconnect {
        *client = None;
        eprintln!(
            "Conectando ao OBS WebSocket em {}:{}.",
            preferences.host, preferences.port
        );
        let socket = connect_identified(&preferences)?;
        *client = Some(ObsClient {
            preferences: preferences.clone(),
            socket,
            last_used: Instant::now(),
        });
    } else {
        eprintln!("Reutilizando conexao OBS WebSocket.");
    }

    let current = client
        .as_mut()
        .ok_or_else(|| "Cliente OBS indisponivel apos conexao.".to_string())?;
    let result = operation(&mut current.socket);

    match result {
        Ok(value) => {
            current.last_used = Instant::now();
            Ok(value)
        }
        Err(error) => {
            if is_obs_connection_error(&error) {
                eprintln!("Conexao OBS perdida; descartando cliente atual.");
                *client = None;
            }
            Err(error)
        }
    }
}

fn is_obs_connection_error(error: &str) -> bool {
    error.contains("A conexao com OBS falhou")
        || error.contains("O OBS fechou a conexao")
        || error.contains("Nao foi possivel enviar comando ao OBS")
        || error.contains("O OBS retornou uma mensagem invalida")
        || error.contains("O OBS retornou dados invalidos")
}

enum ObsStatusAttemptError {
    Busy,
    Failed(String),
}

fn with_obs_status_connection<T>(
    app: &tauri::AppHandle,
    operation: impl FnOnce(&mut ObsSocket) -> Result<T, String>,
) -> Result<T, ObsStatusAttemptError> {
    let preferences = load_stream_preferences_internal(app)
        .map_err(ObsStatusAttemptError::Failed)?
        .preferences
        .obs;
    let state = app.state::<ObsClientState>();
    let mut client = match state.client.try_lock() {
        Ok(client) => client,
        Err(TryLockError::WouldBlock) => return Err(ObsStatusAttemptError::Busy),
        Err(TryLockError::Poisoned(_)) => {
            return Err(ObsStatusAttemptError::Failed(
                "Nao foi possivel acessar o cliente OBS.".into(),
            ));
        }
    };

    let should_reconnect = match client.as_ref() {
        Some(current) if current.preferences != preferences => true,
        Some(current) if current.last_used.elapsed() > OBS_IDLE_TIMEOUT => true,
        Some(_) => false,
        None => true,
    };

    if should_reconnect {
        *client = None;
        eprintln!(
            "Conectando ao OBS WebSocket para status em {}:{}.",
            preferences.host, preferences.port
        );
        let socket =
            connect_identified_with_timeout(&preferences, Some(OBS_STATUS_CONNECT_TIMEOUT))
                .map_err(ObsStatusAttemptError::Failed)?;
        *client = Some(ObsClient {
            preferences: preferences.clone(),
            socket,
            last_used: Instant::now(),
        });
    } else {
        eprintln!("Reutilizando conexao OBS WebSocket para status.");
    }

    let current = client.as_mut().ok_or_else(|| {
        ObsStatusAttemptError::Failed("Cliente OBS indisponivel apos conexao.".into())
    })?;
    let result = operation(&mut current.socket);

    match result {
        Ok(value) => {
            current.last_used = Instant::now();
            Ok(value)
        }
        Err(error) => {
            if is_obs_connection_error(&error) {
                eprintln!("Conexao OBS perdida; descartando cliente atual.");
                *client = None;
            }
            Err(ObsStatusAttemptError::Failed(error))
        }
    }
}

fn status_cooldown_response(app: &tauri::AppHandle) -> Option<ObsStreamStatus> {
    let state = app.state::<ObsClientState>();
    let Ok(status) = state.status.lock() else {
        return Some(empty_obs_status(false, true));
    };
    let cooldown_until = status.cooldown_until?;
    if Instant::now() < cooldown_until {
        eprintln!("Pulando consulta OBS durante cooldown.");
        return Some(status.cached_status_or_empty(true));
    }
    None
}

fn is_status_in_cooldown(app: &tauri::AppHandle) -> bool {
    let state = app.state::<ObsClientState>();
    state
        .status
        .lock()
        .ok()
        .and_then(|status| status.cooldown_until)
        .is_some_and(|cooldown_until| Instant::now() < cooldown_until)
}

fn remember_status_success(app: &tauri::AppHandle, mut status: ObsStreamStatus) {
    let state = app.state::<ObsClientState>();
    let Ok(mut current) = state.status.lock() else {
        return;
    };

    status.available = true;
    status.stale = false;
    current.cached = Some(CachedObsStatus { status });
    current.cooldown_until = None;
    current.consecutive_failures = 0;
}

fn remember_input_mute_success(app: &tauri::AppHandle, statuses: &HashMap<String, bool>) {
    let state = app.state::<ObsClientState>();
    let Ok(mut current) = state.status.lock() else {
        return;
    };

    let mut status = current
        .cached
        .as_ref()
        .map(|cached| cached.status.clone())
        .unwrap_or_else(|| empty_obs_status(true, false));
    status.available = true;
    status.stale = false;
    for (input_name, muted) in statuses {
        status.input_mutes.insert(input_name.clone(), *muted);
    }
    current.cached = Some(CachedObsStatus { status });
    current.cooldown_until = None;
    current.consecutive_failures = 0;
}

fn remember_source_visibility_success(app: &tauri::AppHandle, statuses: &HashMap<String, bool>) {
    let state = app.state::<ObsClientState>();
    let Ok(mut current) = state.status.lock() else {
        return;
    };

    let mut status = current
        .cached
        .as_ref()
        .map(|cached| cached.status.clone())
        .unwrap_or_else(|| empty_obs_status(true, false));
    status.available = true;
    status.stale = false;
    for (source_key, visible) in statuses {
        status
            .source_visibilities
            .insert(source_key.clone(), *visible);
    }
    current.cached = Some(CachedObsStatus { status });
    current.cooldown_until = None;
    current.consecutive_failures = 0;
}

fn remember_status_failure(app: &tauri::AppHandle) {
    let state = app.state::<ObsClientState>();
    let Ok(mut status) = state.status.lock() else {
        return;
    };

    status.consecutive_failures = status.consecutive_failures.saturating_add(1);
    let cooldown = if status.consecutive_failures > 1 {
        OBS_STATUS_MAX_COOLDOWN
    } else {
        OBS_STATUS_INITIAL_COOLDOWN
    };
    status.cooldown_until = Some(Instant::now() + cooldown);
    eprintln!(
        "OBS indisponivel, entrando em cooldown por {}s.",
        cooldown.as_secs()
    );
}

fn cached_status_or_empty(app: &tauri::AppHandle, stale: bool) -> ObsStreamStatus {
    let state = app.state::<ObsClientState>();
    state
        .status
        .lock()
        .ok()
        .map(|status| status.cached_status_or_empty(stale))
        .unwrap_or_else(|| empty_obs_status(false, true))
}

fn cached_input_mutes(app: &tauri::AppHandle) -> HashMap<String, bool> {
    let state = app.state::<ObsClientState>();
    state
        .status
        .lock()
        .ok()
        .and_then(|status| {
            status
                .cached
                .as_ref()
                .map(|cached| cached.status.input_mutes.clone())
        })
        .unwrap_or_default()
}

fn cached_source_visibilities(app: &tauri::AppHandle) -> HashMap<String, bool> {
    let state = app.state::<ObsClientState>();
    state
        .status
        .lock()
        .ok()
        .and_then(|status| {
            status
                .cached
                .as_ref()
                .map(|cached| cached.status.source_visibilities.clone())
        })
        .unwrap_or_default()
}

impl ObsStatusState {
    fn cached_status_or_empty(&self, stale: bool) -> ObsStreamStatus {
        self.cached
            .as_ref()
            .map(|cached| {
                let mut status = cached.status.clone();
                status.available = !stale;
                status.stale = stale;
                status
            })
            .unwrap_or_else(|| empty_obs_status(false, stale))
    }
}

fn empty_obs_status(available: bool, stale: bool) -> ObsStreamStatus {
    ObsStreamStatus {
        recording: ObsOutputStatus { active: false },
        streaming: ObsOutputStatus { active: false },
        input_mutes: HashMap::new(),
        source_visibilities: HashMap::new(),
        available,
        stale,
    }
}

fn source_visibility_key(scene_name: &str, source_name: &str) -> String {
    format!("{}::{}", scene_name.trim(), source_name.trim())
}

fn output_is_active(socket: &mut ObsSocket, request_type: &str) -> Result<bool, String> {
    let response = send_obs_request(socket, request_type, json!({}))?;
    response
        .get("outputActive")
        .and_then(Value::as_bool)
        .ok_or_else(|| "O OBS nao retornou o status da saida.".to_string())
}

fn ensure_output_active(
    socket: &mut ObsSocket,
    request_type: &str,
    message: &str,
) -> Result<(), String> {
    if output_is_active(socket, request_type)? {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn ensure_output_inactive(
    socket: &mut ObsSocket,
    request_type: &str,
    message: &str,
) -> Result<(), String> {
    if output_is_active(socket, request_type)? {
        Err(message.into())
    } else {
        Ok(())
    }
}

fn input_is_muted(socket: &mut ObsSocket, input_name: &str) -> Result<bool, String> {
    let response = send_obs_request(socket, "GetInputMute", json!({ "inputName": input_name }))?;
    response
        .get("inputMuted")
        .and_then(Value::as_bool)
        .ok_or_else(|| "O OBS nao retornou o status de mute do input.".to_string())
}

fn list_sources_for_scene(socket: &mut ObsSocket, scene_name: &str) -> Result<Vec<String>, String> {
    let response = send_obs_request(
        socket,
        "GetSceneItemList",
        json!({ "sceneName": scene_name }),
    )?;
    let scene_items = response
        .get("sceneItems")
        .and_then(Value::as_array)
        .ok_or_else(|| "O OBS nao retornou a lista de fontes da cena.".to_string())?;

    Ok(scene_items
        .iter()
        .filter_map(|item| item.get("sourceName").and_then(Value::as_str))
        .map(str::to_string)
        .collect())
}

fn scene_item_id(
    socket: &mut ObsSocket,
    scene_name: &str,
    source_name: &str,
) -> Result<i64, String> {
    let response = send_obs_request(
        socket,
        "GetSceneItemList",
        json!({ "sceneName": scene_name }),
    )?;
    let scene_items = response
        .get("sceneItems")
        .and_then(Value::as_array)
        .ok_or_else(|| "O OBS nao retornou a lista de fontes da cena.".to_string())?;

    scene_items
        .iter()
        .find(|item| item.get("sourceName").and_then(Value::as_str) == Some(source_name))
        .and_then(|item| item.get("sceneItemId").and_then(Value::as_i64))
        .ok_or_else(|| format!("Fonte nao encontrada na cena '{scene_name}': {source_name}"))
}

fn scene_item_is_enabled(
    socket: &mut ObsSocket,
    scene_name: &str,
    scene_item_id: i64,
) -> Result<bool, String> {
    let response = send_obs_request(
        socket,
        "GetSceneItemEnabled",
        json!({ "sceneName": scene_name, "sceneItemId": scene_item_id }),
    )?;
    response
        .get("sceneItemEnabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| "O OBS nao retornou a visibilidade da fonte.".to_string())
}

fn connect_identified(preferences: &ObsPreferences) -> Result<ObsSocket, String> {
    connect_identified_with_timeout(preferences, None)
}

fn connect_identified_with_timeout(
    preferences: &ObsPreferences,
    timeout: Option<Duration>,
) -> Result<ObsSocket, String> {
    let host = preferences.host.trim();
    if host.is_empty() {
        return Err("Informe o host do OBS Studio.".into());
    }

    let url = format!("ws://{}:{}", host, preferences.port);
    let (mut socket, _) = if let Some(timeout) = timeout {
        let address = (host, preferences.port)
            .to_socket_addrs()
            .map_err(|error| stream_error("Nao foi possivel resolver o host do OBS", error))?
            .next()
            .ok_or_else(|| "Nao foi possivel resolver o host do OBS.".to_string())?;
        let stream = TcpStream::connect_timeout(&address, timeout).map_err(|error| {
            stream_error(
                "Nao foi possivel conectar ao OBS Studio. Verifique se o WebSocket esta ativo",
                error,
            )
        })?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|error| stream_error("Nao foi possivel configurar timeout do OBS", error))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|error| stream_error("Nao foi possivel configurar timeout do OBS", error))?;
        websocket_client(&url, MaybeTlsStream::Plain(stream)).map_err(|error| {
            stream_error(
                "Nao foi possivel conectar ao OBS Studio. Verifique se o WebSocket esta ativo",
                error,
            )
        })?
    } else {
        connect(&url).map_err(|error| {
            stream_error(
                "Nao foi possivel conectar ao OBS Studio. Verifique se o WebSocket esta ativo",
                error,
            )
        })?
    };

    let hello = read_obs_payload(&mut socket)?;
    let authentication = hello.get("authentication");
    let identify = match authentication {
        Some(authentication) => {
            let challenge = authentication
                .get("challenge")
                .and_then(Value::as_str)
                .ok_or_else(|| "O OBS retornou um desafio de autenticacao invalido.".to_string())?;
            let salt = authentication
                .get("salt")
                .and_then(Value::as_str)
                .ok_or_else(|| "O OBS retornou um salt de autenticacao invalido.".to_string())?;
            json!({
                "op": 1,
                "d": {
                    "rpcVersion": 1,
                    "eventSubscriptions": 0,
                    "authentication": obs_authentication(&preferences.password, salt, challenge)
                }
            })
        }
        None => json!({
            "op": 1,
            "d": {
                "rpcVersion": 1,
                "eventSubscriptions": 0
            }
        }),
    };

    write_obs_message(&mut socket, identify)?;
    wait_for_identified(&mut socket)?;
    Ok(socket)
}

fn obs_authentication(password: &str, salt: &str, challenge: &str) -> String {
    let secret = digest_base64(format!("{password}{salt}").as_bytes());
    digest_base64(format!("{secret}{challenge}").as_bytes())
}

fn digest_base64(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    general_purpose::STANDARD.encode(digest)
}

fn wait_for_identified(socket: &mut ObsSocket) -> Result<(), String> {
    loop {
        let message = read_obs_message(socket)?;
        match message.get("op").and_then(Value::as_i64) {
            Some(2) => return Ok(()),
            Some(9) => {
                let code = message
                    .get("d")
                    .and_then(|data| data.get("code"))
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                let comment = message
                    .get("d")
                    .and_then(|data| data.get("comment"))
                    .and_then(Value::as_str)
                    .unwrap_or("identificacao recusada");
                return Err(format!("OBS recusou a conexao ({code}): {comment}"));
            }
            _ => {}
        }
    }
}

fn send_obs_request(
    socket: &mut ObsSocket,
    request_type: &str,
    request_data: Value,
) -> Result<Value, String> {
    let request_id = format!(
        "orbit-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default()
    );
    write_obs_message(
        socket,
        json!({
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": request_id,
                "requestData": request_data
            }
        }),
    )?;

    loop {
        let message = read_obs_message(socket)?;
        if message.get("op").and_then(Value::as_i64) != Some(7) {
            continue;
        }

        let data = message
            .get("d")
            .ok_or_else(|| "O OBS retornou uma resposta vazia.".to_string())?;
        if data.get("requestId").and_then(Value::as_str) != Some(request_id.as_str()) {
            continue;
        }

        let status = data
            .get("requestStatus")
            .ok_or_else(|| "O OBS retornou uma resposta sem status.".to_string())?;
        if status.get("result").and_then(Value::as_bool) == Some(true) {
            return Ok(data
                .get("responseData")
                .cloned()
                .unwrap_or_else(|| json!({})));
        }

        let code = status
            .get("code")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let comment = status
            .get("comment")
            .and_then(Value::as_str)
            .unwrap_or("a solicitacao falhou");
        return Err(format!("OBS nao executou o comando ({code}): {comment}"));
    }
}

fn read_obs_payload(socket: &mut ObsSocket) -> Result<Value, String> {
    let message = read_obs_message(socket)?;
    message
        .get("d")
        .cloned()
        .ok_or_else(|| "O OBS retornou uma mensagem sem dados.".into())
}

fn read_obs_message(socket: &mut ObsSocket) -> Result<Value, String> {
    loop {
        let message = socket
            .read()
            .map_err(|error| stream_error("A conexao com OBS falhou", error))?;
        let text = match message {
            Message::Text(text) => text,
            Message::Binary(bytes) => String::from_utf8(bytes)
                .map_err(|error| stream_error("O OBS retornou dados invalidos", error))?,
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => return Err("O OBS fechou a conexao.".into()),
            Message::Frame(_) => continue,
        };

        return serde_json::from_str(&text)
            .map_err(|error| stream_error("O OBS retornou uma mensagem invalida", error));
    }
}

fn write_obs_message(socket: &mut ObsSocket, value: Value) -> Result<(), String> {
    socket
        .send(Message::Text(value.to_string()))
        .map_err(|error| stream_error("Nao foi possivel enviar comando ao OBS", error))
}
