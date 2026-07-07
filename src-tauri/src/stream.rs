use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, net::TcpStream, path::PathBuf, time::SystemTime};
use tauri::Manager;
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

const STREAM_PREFERENCES_FILE: &str = "stream-preferences.json";

type ObsSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPreferences {
    pub obs: ObsPreferences,
}

#[derive(Clone, Deserialize, Serialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsOutputStatus {
    pub active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsStreamStatus {
    pub recording: ObsOutputStatus,
    pub streaming: ObsOutputStatus,
    pub input_mutes: HashMap<String, bool>,
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
    write_preferences(&app, &normalize_preferences(preferences))
}

#[tauri::command]
pub fn test_obs_connection(app: tauri::AppHandle) -> Result<ObsConnectionStatus, String> {
    let preferences = load_stream_preferences_internal(&app)?.preferences;
    connect_identified(&preferences.obs)?;
    Ok(ObsConnectionStatus {
        ok: true,
        message: "Conexao com OBS Studio realizada.".into(),
    })
}

#[tauri::command]
pub fn list_obs_scenes(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let preferences = load_stream_preferences_internal(&app)?.preferences;
    let mut socket = connect_identified(&preferences.obs)?;
    let response = send_obs_request(&mut socket, "GetSceneList", json!({}))?;
    let scenes = response
        .get("scenes")
        .and_then(Value::as_array)
        .ok_or_else(|| "O OBS nao retornou a lista de cenas.".to_string())?;

    Ok(scenes
        .iter()
        .filter_map(|scene| scene.get("sceneName").and_then(Value::as_str))
        .map(str::to_string)
        .collect())
}

#[tauri::command]
pub fn list_obs_inputs(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let preferences = load_stream_preferences_internal(&app)?.preferences;
    let mut socket = connect_identified(&preferences.obs)?;
    let response = send_obs_request(&mut socket, "GetInputList", json!({}))?;
    let inputs = response
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "O OBS nao retornou a lista de inputs.".to_string())?;

    Ok(inputs
        .iter()
        .filter_map(|input| input.get("inputName").and_then(Value::as_str))
        .map(str::to_string)
        .collect())
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

    let preferences = load_stream_preferences_internal(&app)?.preferences;
    let mut socket = connect_identified(&preferences.obs)?;
    list_sources_for_scene(&mut socket, scene_name)
}

#[tauri::command]
pub fn get_obs_stream_status(app: tauri::AppHandle) -> Result<ObsStreamStatus, String> {
    let preferences = load_stream_preferences_internal(&app)?.preferences;
    let mut socket = connect_identified(&preferences.obs)?;
    Ok(ObsStreamStatus {
        recording: ObsOutputStatus {
            active: output_is_active(&mut socket, "GetRecordStatus")?,
        },
        streaming: ObsOutputStatus {
            active: output_is_active(&mut socket, "GetStreamStatus")?,
        },
        input_mutes: HashMap::new(),
    })
}

#[tauri::command]
pub fn get_obs_input_mute_statuses(
    app: tauri::AppHandle,
    input_names: Vec<String>,
) -> Result<HashMap<String, bool>, String> {
    let preferences = load_stream_preferences_internal(&app)?.preferences;
    let mut socket = connect_identified(&preferences.obs)?;
    let mut statuses = HashMap::new();

    for input_name in input_names {
        let input_name = input_name.trim();
        if input_name.is_empty() || statuses.contains_key(input_name) {
            continue;
        }
        if let Ok(muted) = input_is_muted(&mut socket, input_name) {
            statuses.insert(input_name.to_string(), muted);
        }
    }

    Ok(statuses)
}

#[tauri::command]
pub fn execute_stream_action(app: tauri::AppHandle, action: StreamAction) -> Result<(), String> {
    validate_stream_action(&action)?;
    let preferences = load_stream_preferences_internal(&app)?.preferences;
    let mut socket = connect_identified(&preferences.obs)?;

    match (&action.provider, &action.operation) {
        (StreamProvider::Obs, StreamOperation::SetScene) => {
            let scene_name = action.scene_name.trim();
            send_obs_request(
                &mut socket,
                "SetCurrentProgramScene",
                json!({ "sceneName": scene_name }),
            )?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StartRecording) => {
            ensure_output_inactive(&mut socket, "GetRecordStatus", "A gravacao ja esta ativa.")?;
            send_obs_request(&mut socket, "StartRecord", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StopRecording) => {
            ensure_output_active(
                &mut socket,
                "GetRecordStatus",
                "A gravacao ja esta inativa.",
            )?;
            send_obs_request(&mut socket, "StopRecord", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleRecording) => {
            if output_is_active(&mut socket, "GetRecordStatus")? {
                send_obs_request(&mut socket, "StopRecord", json!({}))?;
            } else {
                send_obs_request(&mut socket, "StartRecord", json!({}))?;
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StartStreaming) => {
            ensure_output_inactive(
                &mut socket,
                "GetStreamStatus",
                "A transmissao ja esta ativa.",
            )?;
            send_obs_request(&mut socket, "StartStream", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::StopStreaming) => {
            ensure_output_active(
                &mut socket,
                "GetStreamStatus",
                "A transmissao ja esta inativa.",
            )?;
            send_obs_request(&mut socket, "StopStream", json!({}))?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::ToggleStreaming) => {
            if output_is_active(&mut socket, "GetStreamStatus")? {
                send_obs_request(&mut socket, "StopStream", json!({}))?;
            } else {
                send_obs_request(&mut socket, "StartStream", json!({}))?;
            }
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::SetInputMute) => {
            send_obs_request(
                &mut socket,
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
            let muted = input_is_muted(&mut socket, input_name)?;
            send_obs_request(
                &mut socket,
                "SetInputMute",
                json!({ "inputName": input_name, "inputMuted": !muted }),
            )?;
            Ok(())
        }
        (StreamProvider::Obs, StreamOperation::SetSourceVisibility) => {
            let scene_name = action.scene_name.trim();
            let source_name = action.source_name.trim();
            let scene_item_id = scene_item_id(&mut socket, scene_name, source_name)?;
            send_obs_request(
                &mut socket,
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
            let scene_item_id = scene_item_id(&mut socket, scene_name, source_name)?;
            let visible = scene_item_is_enabled(&mut socket, scene_name, scene_item_id)?;
            send_obs_request(
                &mut socket,
                "SetSceneItemEnabled",
                json!({
                    "sceneName": scene_name,
                    "sceneItemId": scene_item_id,
                    "sceneItemEnabled": !visible,
                }),
            )?;
            Ok(())
        }
    }
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
    let host = preferences.host.trim();
    if host.is_empty() {
        return Err("Informe o host do OBS Studio.".into());
    }

    let url = format!("ws://{}:{}", host, preferences.port);
    let (mut socket, _) = connect(&url).map_err(|error| {
        stream_error(
            "Nao foi possivel conectar ao OBS Studio. Verifique se o WebSocket esta ativo",
            error,
        )
    })?;

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
