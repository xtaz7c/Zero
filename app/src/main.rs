// Zero — a desktop app where several AIs can chat together in one main thread.
//
// Usage:
//   zero                        open the app
//   zero --screenshot out.png   open without focus, draw ~120 frames, save the window's
//                               picture as a PNG to that file and exit

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, Rect};
use serde::{Deserialize, Serialize};
use eframe::{App, CreationContext, Frame, NativeOptions, Renderer, run_native};

const APP_VERSION: &str = "1.0.0";
const FRAMES_BEFORE_SHOT: u64 = 120;
const OLLAMA_CHAT_URL: &str = "http://localhost:11434/v1/chat/completions";
const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";
const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const GEMINI_CHAT_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";

// ---------------------------------------------------------------- theme
// Chosen for the app: black background, light gray text, pink-red highlight.

const BG_COLOR: Color32 = Color32::BLACK;
const TEXT_COLOR: Color32 = Color32::from_rgb(0xEE, 0xEE, 0xEE); // Cinza claro
const HIGHLIGHT_COLOR: Color32 = Color32::from_rgb(0xE9, 0x1E, 0x63); // Vermelho-Rosa

/// Visuals for the app theme.
fn theme_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = BG_COLOR;
    v.window_fill = BG_COLOR;
    v.extreme_bg_color = BG_COLOR;
    v.faint_bg_color = BG_COLOR;
    v.text_edit_bg_color = Some(BG_COLOR);
    v.override_text_color = Some(TEXT_COLOR);
    v.weak_text_color = Some(TEXT_COLOR.gamma_multiply(0.6));
    v.selection.bg_fill = HIGHLIGHT_COLOR.gamma_multiply(0.5);
    v.selection.stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_COLOR);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, HIGHLIGHT_COLOR);
    v.widgets.active.fg_stroke = egui::Stroke::new(2.0, HIGHLIGHT_COLOR);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v
}

// ---------------------------------------------------------------- strings

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    PtBr,
    En,
    Es,
    Fr,
    De,
    It,
}

#[derive(Clone, Copy)]
struct Strings {
    settings: &'static str,
    close: &'static str,
    language: &'static str,
    models: &'static str,
    add_model: &'static str,
    remove_model: &'static str,
    model_name: &'static str,
    model_endpoint: &'static str,
    model_api_key: &'static str,
    auto_update: &'static str,
    update_url: &'static str,
    check_updates: &'static str,
    update_checking: &'static str,
    update_current: &'static str,
    update_new: &'static str,
    update_no_url: &'static str,
    update_fail: &'static str,
    update_open: &'static str,
    input_placeholder: &'static str,
    send: &'static str,
    user_label: &'static str,
    no_models_hint: &'static str,
    start_hint: &'static str,
    ai_roster: &'static str,
    /// Resposta local quando um modelo de API pública não tem chave
    /// de API: o endpoint responderia 401/400, então o chat mostra
    /// esta mensagem de aviso em vez do erro cru.
    no_key_reply: &'static str,
}

impl Lang {
    const ALL: &'static [Lang] = &[Lang::PtBr, Lang::En, Lang::Es, Lang::Fr, Lang::De, Lang::It];

    fn code(self) -> &'static str {
        match self {
            Lang::PtBr => "pt_BR",
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::It => "it",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Lang::PtBr => "Português (Brasil)",
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::It => "Italiano",
        }
    }

    fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.iter().copied().find(|l| l.code() == code)
    }

    fn strings(self) -> Strings {
        match self {
            Lang::PtBr => Strings {
                ai_roster: "IAs:",
                no_key_reply: "[{}] Sem chave de API configurada — adicione-a nas Configurações para falar comigo.",
                settings: "Configurações",
                close: "Fechar",
                language: "Idioma",
                models: "Modelos",
                add_model: "Adicionar modelo",
                remove_model: "Excluir",
                model_name: "Nome",
                model_endpoint: "Endpoint",
                model_api_key: "Chave de API",
                auto_update: "Atualização automática por IA",
                update_url: "URL de atualização",
                check_updates: "Verificar agora",
                update_checking: "Verificando…",
                update_current: "Versão atualizada",
                update_new: "Nova versão disponível",
                update_no_url: "Defina a URL de atualização nas Configurações.",
                update_fail: "Não foi possível verificar:",
                update_open: "Abrir URL de atualização",
                input_placeholder: "Escreva sua mensagem…",
                send: "Enviar",
                user_label: "Você",
                no_models_hint: "Ative modelos nas Configurações para conversar com as IAs.",
                start_hint: "Envie sua primeira mensagem",
            },
            Lang::En => Strings {
                ai_roster: "AIs:",
                no_key_reply: "[{}] No API key set — add one in Settings to chat with me.",
                settings: "Settings",
                close: "Close",
                language: "Language",
                models: "Models",
                add_model: "Add model",
                remove_model: "Remove",
                model_name: "Name",
                model_endpoint: "Endpoint",
                model_api_key: "API key",
                auto_update: "Auto-update by AI",
                update_url: "Update URL",
                check_updates: "Check now",
                update_checking: "Checking…",
                update_current: "Up to date",
                update_new: "New version available",
                update_no_url: "Set the update URL in Settings.",
                update_fail: "Could not check:",
                update_open: "Open update URL",
                input_placeholder: "Type your message…",
                send: "Send",
                user_label: "You",
                no_models_hint: "Turn on models in Settings to chat with the AIs.",
                start_hint: "Send your first message",
            },
            Lang::Es => Strings {
                ai_roster: "IA:",
                no_key_reply: "[{}] Sin clave de API configurada — añade una en los Ajustes para conversar conmigo.",
                settings: "Ajustes",
                close: "Cerrar",
                language: "Idioma",
                models: "Modelos",
                add_model: "Añadir modelo",
                remove_model: "Eliminar",
                model_name: "Nombre",
                model_endpoint: "Endpoint",
                model_api_key: "Clave de API",
                auto_update: "Actualización automática por IA",
                update_url: "URL de actualización",
                check_updates: "Comprobar ahora",
                update_checking: "Comprobando…",
                update_current: "Versión actualizada",
                update_new: "Nueva versión disponible",
                update_no_url: "Define la URL de actualización en Ajustes.",
                update_fail: "No se pudo comprobar:",
                update_open: "Abrir la URL de actualización",
                input_placeholder: "Escribe tu mensaje…",
                send: "Enviar",
                user_label: "Tú",
                no_models_hint: "Activa modelos en Ajustes para conversar con las IAs.",
                start_hint: "Envía tu primera mensaje",
            },
            Lang::Fr => Strings {
                ai_roster: "IA :",
                no_key_reply: "[{}] Clé API non configurée — ajoutez-en une dans les Paramètres pour discuter avec moi.",
                settings: "Configuration",
                close: "Fermer",
                language: "Langue",
                models: "Modèles",
                add_model: "Ajouter un modèle",
                remove_model: "Supprimer",
                model_name: "Nom",
                model_endpoint: "Adresse",
                model_api_key: "Clé API",
                auto_update: "Mise à jour automatique par IA",
                update_url: "URL de mise à jour",
                check_updates: "Vérifier maintenant",
                update_checking: "Vérification…",
                update_current: "À jour",
                update_new: "Nouvelle version disponible",
                update_no_url: "Définissez l'URL de mise à jour dans les paramètres.",
                update_fail: "Impossible de vérifier :",
                update_open: "Ouvrir l'URL de mise à jour",
                input_placeholder: "Écrivez votre message…",
                send: "Envoyer",
                user_label: "Vous",
                no_models_hint: "Activez des modèles dans les paramètres pour discuter avec les IA.",
                start_hint: "Envoyez votre premier message",
            },
            Lang::De => Strings {
                ai_roster: "KI:",
                no_key_reply: "[{}] Kein API-Key eingerichtet — füge einen in den Einstellungen hinzu, um zu chatten.",
                settings: "Einstellungen",
                close: "Schließen",
                language: "Sprache",
                models: "Modelle",
                add_model: "Modell hinzufügen",
                remove_model: "Entfernen",
                model_name: "Name",
                model_endpoint: "Endpunkt",
                model_api_key: "API-Key",
                auto_update: "Automatische Aktualisierung durch KI",
                update_url: "Aktualisierungs-URL",
                check_updates: "Jetzt prüfen",
                update_checking: "Prüfe…",
                update_current: "Auf dem neuesten Stand",
                update_new: "Neue Version verfügbar",
                update_no_url: "Definieren Sie die Aktualisierungs-URL in den Einstellungen.",
                update_fail: "Überprüfung nicht möglich:",
                update_open: "Aktualisierungs-URL öffnen",
                input_placeholder: "Schreiben Sie Ihre Nachricht…",
                send: "Senden",
                user_label: "Du",
                no_models_hint: "Aktivieren Sie Modelle in den Einstellungen, um mit den KIs zu chatten.",
                start_hint: "Schicken Sie Ihre erste Nachricht",
            },
            Lang::It => Strings {
                ai_roster: "IA:",
                no_key_reply: "[{}] Chiave API non configurata — impostane una nelle Impostazioni per chattare con me.",
                settings: "Impostazioni",
                close: "Chiudi",
                language: "Lingua",
                models: "Modelli",
                add_model: "Aggiungi modello",
                remove_model: "Rimuovi",
                model_name: "Nome",
                model_endpoint: "Endpoint",
                model_api_key: "Chiave API",
                auto_update: "Aggiornamento automatico per IA",
                update_url: "URL di aggiornamento",
                check_updates: "Controlla ora",
                update_checking: "Controllo…",
                update_current: "Aggiornato",
                update_new: "Nuova versione disponibile",
                update_no_url: "Imposta l'URL di aggiornamento nelle Impostazioni.",
                update_fail: "Impossibile verificare:",
                update_open: "Apri l'URL di aggiornamento",
                input_placeholder: "Scrivi il tuo messaggio…",
                send: "Invia",
                user_label: "Tu",
                no_models_hint: "Attiva i modelli nelle Impostazioni per chattare con le IA.",
                start_hint: "Invia il tuo primo messaggio",
            },
        }
    }
}

// ---------------------------------------------------------------- model / chat

/// Como falar com um modelo: "openai" para endpoints de chat-completions
/// compatíveis com a OpenAI, "anthropic" para a API de mensagens do Claude.
fn default_api_style() -> String {
    "openai".into()
}

#[derive(Clone, Serialize, Deserialize)]
struct Model {
    name: String,
    id: String,
    endpoint: String,
    api_key: String,
    enabled: bool,
    #[serde(default = "default_api_style")]
    api_style: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    #[serde(default)]
    model: Option<String>,
    text: String,
    #[serde(default)]
    ts: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    language: Option<String>,
    auto_update: Option<bool>,
    update_url: Option<String>,
    models: Option<Vec<Model>>,
    chat: Option<Vec<Message>>,
}

struct PendingAi {
    name: String,
    /// Resposta local quando o modelo de API pública não tem chave
    /// de API: sem chave o endpoint responde 401/400, então o chat mostra
    /// esta mensagem em vez do erro cru. Modelos locais não precisam.
    local_reply: Option<String>,
    rx: Receiver<(String, Result<String, String>)>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("settings.json")))
        .unwrap_or_else(|| PathBuf::from("settings.json"))
}

/// Modelos instalados no Ollama: o id enviado ao Ollama precisa ser um dos
/// nomes que ele realmente tem; se não for, o chat dava "modelo não
/// encontrado". Descobre por aqui (/api/tags).
fn ollama_models() -> Vec<String> {
    let resp = match ureq::get(OLLAMA_TAGS_URL).timeout(Duration::from_secs(3)).call() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let v: serde_json::Value = match resp.into_json() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut names: Vec<String> = v
        .pointer("/models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    m.get("name")
                        .or_else(|| m.get("model"))
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.dedup();
    names
}

/// Catálogo das IAs: o app conhece os modelos principais de cada uma —
/// OpenAI (GPT-6), Anthropic (Claude) e Google (Gemini). Para usar um:
/// ative na lista e coloque a chave de API nas Configurações.
fn ai_catalog() -> Vec<Model> {
    let m = |name: &str, id: &str, endpoint: &str, style: &str| Model {
        name: name.into(),
        id: id.into(),
        endpoint: endpoint.into(),
        api_key: String::new(),
        enabled: false,
        api_style: style.into(),
    };
    vec![
        m("Gemini 4 Carbon", "gemini-4-carbon", GEMINI_CHAT_URL, "openai"),
        m("Gemini 4 Neon", "gemini-4-neon", GEMINI_CHAT_URL, "openai"),
        m("Gemini 4 Hydrogen", "gemini-4-hydrogen", GEMINI_CHAT_URL, "openai"),
        m("Gemini 4 Argon", "gemini-4-argon", GEMINI_CHAT_URL, "openai"),
        m("Gemini 3.8 Flash", "gemini-3.8-flash", GEMINI_CHAT_URL, "openai"),
        m("GPT-6.2 Astra", "gpt-6.2-astra", OPENAI_CHAT_URL, "openai"),
        m("GPT-6.2 Sol", "gpt-6.2-sol", OPENAI_CHAT_URL, "openai"),
        m("GPT-6.1 Terra", "gpt-6.1-terra", OPENAI_CHAT_URL, "openai"),
        m("GPT-6 Luna", "gpt-6-luna", OPENAI_CHAT_URL, "openai"),
        m("Claude Fable 5.2", "claude-fable-5-2", ANTHROPIC_MESSAGES_URL, "anthropic"),
        m("Claude Opus 5.5", "claude-opus-5-5", ANTHROPIC_MESSAGES_URL, "anthropic"),
        m("Claude Sonnet 5.5", "claude-sonnet-5-5", ANTHROPIC_MESSAGES_URL, "anthropic"),
        m("Claude Haiku 5.5", "claude-haiku-5-5", ANTHROPIC_MESSAGES_URL, "anthropic"),
    ]
}

fn default_models() -> Vec<Model> {
    let mut models: Vec<Model> = ollama_models()
        .into_iter()
        .map(|id| Model {
            name: id.clone(),
            id,
            endpoint: OLLAMA_CHAT_URL.into(),
            api_key: String::new(),
            enabled: true,
            api_style: default_api_style(),
        })
        .collect();
    if models.is_empty() {
        // Sem nada instalado no Ollama, um modelo a desativar depois.
        models.push(Model {
            name: "Local (Ollama)".into(),
            id: "local-model".into(),
            endpoint: OLLAMA_CHAT_URL.into(),
            api_key: String::new(),
            enabled: true,
            api_style: default_api_style(),
        });
    }
    models.extend(ai_catalog());
    models
}

// ---------------------------------------------------------------- network

/// One turn to the model endpoint. `style` says the API dialect:
/// "openai" for OpenAI-compatible chat-completions endpoints,
/// "anthropic" for the Claude Messages API.
fn chat_request(
    endpoint: &str,
    api_key: &str,
    model_id: &str,
    history: &[Message],
    style: &str,
) -> Result<String, String> {
    match style {
        "anthropic" => do_anthropic_chat(endpoint, api_key, model_id, history),
        _ => chat_request_openai(endpoint, api_key, model_id, history),
    }
}

fn chat_request_openai(endpoint: &str, api_key: &str, model_id: &str, history: &[Message]) -> Result<String, String> {
    match do_openai_chat(endpoint, api_key, model_id, history) {
        Ok(text) => Ok(text),
        Err(err) => {
            // Correção do Ollama: o id enviado precisa ser um modelo que o
            // Ollama realmente tem instalado. Se não for (ex.: o id antigo
            // "local-model"), usa o primeiro que ele tem.
            if endpoint == OLLAMA_CHAT_URL {
                let first = ollama_models().first().cloned();
                if let Some(first) = first {
                    if first != model_id {
                        if let Ok(text) = do_openai_chat(endpoint, api_key, &first, history) {
                            return Ok(text);
                        }
                    }
                }
            }
            Err(err)
        }
    }
}

fn do_openai_chat(endpoint: &str, api_key: &str, model_id: &str, history: &[Message]) -> Result<String, String> {
    let messages: Vec<serde_json::Value> = history
        .iter()
        .map(|m| {
            let role = if m.role == "user" { "user" } else { "assistant" };
            serde_json::json!({ "role": role, "content": m.text })
        })
        .collect();
    let body = serde_json::json!({
        "model": model_id,
        "messages": messages,
        "temperature": 0.7,
        "max_tokens": 512,
    });
    let req = ureq::post(endpoint)
        .timeout(Duration::from_secs(120))
        .set("Content-Type", "application/json");
    let req = if api_key.is_empty() {
        req
    } else {
        req.set("Authorization", &format!("Bearer {}", api_key))
    };
    match req.send_json(body) {
        Ok(resp) => {
            let value: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
            value
                .pointer("/choices/0/message/content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "resposta sem texto".to_string())
        }
        Err(e) => Err(http_error_string(e)),
    }
}

/// One turn to a Claude Messages API endpoint.
fn do_anthropic_chat(endpoint: &str, api_key: &str, model_id: &str, history: &[Message]) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("chave de API vazia".to_string());
    }
    let messages: Vec<serde_json::Value> = history
        .iter()
        .map(|m| {
            let role = if m.role == "user" { "user" } else { "assistant" };
            serde_json::json!({ "role": role, "content": m.text })
        })
        .collect();
    let body = serde_json::json!({
        "model": model_id,
        "messages": messages,
        "max_tokens": 1024,
    });
    let resp = ureq::post(endpoint)
        .timeout(Duration::from_secs(120))
        .set("Content-Type", "application/json")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .send_json(body)
        .map_err(http_error_string)?;
    let value: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    value
        .pointer("/content/0/text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "resposta sem texto".to_string())
}

/// Breve descrição do erro de HTTP, com a mensagem que a API devolveu.
fn http_error_string(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let t = body.trim();
            if let Some(m) = extract_error_message(t) {
                format!("HTTP {code}: {m}")
            } else if !t.is_empty() {
                format!("HTTP {code}: {t}")
            } else {
                format!("HTTP {code}")
            }
        }
        ureq::Error::Transport(t) => format!("{t}"),
    }
}

/// Extrai a mensagem de erro de um corpo JSON de API:
/// {"error":{"message":…}} (OpenAI, Anthropic) ou {"error": "…"} (algumas).
fn extract_error_message(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    for key in ["/error/message", "/error/detail", "/message"] {
        if let Some(m) = v.pointer(key).and_then(|m| m.as_str()) {
            if !m.is_empty() {
                return Some(m.to_string());
            }
        }
    }
    if let Some(m) = v.get("error").and_then(|e| e.as_str()) {
        if !m.is_empty() {
            return Some(m.to_string());
        }
    }
    None
}

/// Check the configured update URL; expects JSON like {"version": "x.y.z", "url": "..."}.
fn check_for_update(url: &str) -> Result<(String, Option<String>), String> {
    let resp = ureq::get(url)
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| format!("{e}"))?;
    let value: serde_json::Value = resp.into_json().map_err(|e| format!("{e}"))?;
    let version = value
        .get("version")
        .or_else(|| value.get("latest_version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "JSON sem campo 'version'".to_string())?;
    let link = value
        .get("url")
        .or_else(|| value.get("download_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Ok((version, link))
}

fn version_tuple(v: &str) -> (u32, u32, u32) {
    let parse = |s: &str| s.trim().parse::<u32>().unwrap_or(0);
    (
        parse(v.split('.').next().unwrap_or("0")),
        parse(v.split('.').nth(1).unwrap_or("0")),
        parse(v.split('.').nth(2).unwrap_or("0")),
    )
}

fn version_newer(candidate: &str, current: &str) -> bool {
    version_tuple(candidate) > version_tuple(current)
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", "", url]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

// ---------------------------------------------------------------- screenshot

fn save_image_png(img: &egui::ColorImage, path: &std::path::Path) -> Result<(), String> {
    let w = img.size[0] as u32;
    let h = img.size[1] as u32;
    let mut data: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    for px in &img.pixels {
        let [r, g, b, a] = px.to_srgba_unmultiplied();
        data.extend_from_slice(&[r, g, b, a]);
    }
    let img = image::RgbaImage::from_raw(w, h, data).ok_or_else(|| "buffer de pixels inválido".to_string())?;
    img.save(path).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- logo (Craft)

fn logo_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/logo.png")
}

/// Carrega a logo gerada com a Craft (app/assets/logo.png) como textura de 30x30.
fn load_logo_texture(cc: &CreationContext) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(logo_path()).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let img = img.resize_exact(30, 30, image::imageops::FilterType::Triangle);
    let rgba = img.to_rgba8();
    let mut pixels: Vec<u8> = Vec::with_capacity(30 * 30 * 4);
    for px in rgba.pixels() {
        pixels.extend_from_slice(&px.0);
    }
    let color = egui::ColorImage::from_rgba_unmultiplied([30, 30], &pixels);
    Some(cc.egui_ctx.load_texture("zero_logo", color, egui::TextureOptions::LINEAR))
}

// ---------------------------------------------------------------- logo

fn draw_z_logo(ui: &egui::Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let c = rect.center();
    let r = rect.height() * 0.5 - 2.0;
    painter.circle_filled(c, r, color);
    painter.circle_stroke(c, r, egui::Stroke::new(1.5, Color32::WHITE));
    let h = rect.height() * 0.45;
    let w = rect.height() * 0.36;
    let stroke = egui::Stroke::new(3.0, Color32::WHITE);
    painter.line_segment([c + egui::vec2(-w / 2.0, -h / 2.0), c + egui::vec2(w / 2.0, -h / 2.0)], stroke);
    painter.line_segment([c + egui::vec2(w / 2.0, -h / 2.0), c + egui::vec2(-w / 2.0, h / 2.0)], stroke);
    painter.line_segment([c + egui::vec2(-w / 2.0, h / 2.0), c + egui::vec2(w / 2.0, h / 2.0)], stroke);
    painter.rect_filled(
        Rect::from_center_size(c + egui::vec2(-w / 2.0, h / 2.0 + 1.5), egui::vec2(w, 4.0)),
        egui::CornerRadius::ZERO,
        Color32::WHITE,
    );
}

/// Draw a gear (cog) icon in `color` inside `rect`.
fn draw_config_icon(ui: &egui::Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let c = rect.center();
    let r = rect.width() * 0.5 - 2.0;
    // Teeth around the rim.
    let n = 10;
    let tooth_w = r * 0.26;
    let tooth_h = r * 0.4;
    for i in 0..n {
        let a = i as f32 * std::f32::consts::TAU / n as f32;
        let d = egui::vec2(a.cos(), a.sin());
        painter.rect_filled(
            Rect::from_center_size(c + d * r, egui::vec2(tooth_w, tooth_h)),
            egui::CornerRadius::same(2),
            color,
        );
    }
    // Body and ring.
    painter.circle_filled(c, r * 0.5, color);
    painter.circle_stroke(c, r * 0.78, egui::Stroke::new(r * 0.14, color));
}

// ---------------------------------------------------------------- app

struct ZeroApp {
    logo_texture: Option<egui::TextureHandle>,
    shot_rx: Option<Receiver<std::sync::Arc<egui::ColorImage>>>,
    screenshot: Option<PathBuf>,
    frame_nr: u64,
    lang: Lang,
    auto_update: bool,
    update_url: String,
    update_status: String,
    update_available: Option<String>,
    update_check_rx: Option<(String, Receiver<Result<(String, Option<String>), String>>)>,
    last_update_check: Option<Instant>,
    models: Vec<Model>,
    add_name: String,
    add_endpoint: String,
    add_api_key: String,
    chat: Vec<Message>,
    pending: Vec<PendingAi>,
    show_settings: bool,
    input: String,
    focus_input: bool,
}

impl ZeroApp {
    fn new(cc: &CreationContext, screenshot: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(theme_visuals());
        let stored = std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|s| serde_json::from_str::<Stored>(&s).ok())
            .unwrap_or_default();
        let lang = stored
            .language
            .as_deref()
            .and_then(Lang::from_code)
            .unwrap_or(Lang::PtBr);
        let discovered_ollama = ollama_models();
        let mut models = stored.models.filter(|m| !m.is_empty()).unwrap_or_else(default_models);
        // Correção do Ollama: o id enviado precisa ser um modelo que o Ollama
        // realmente tem instalado; se não tiver, usa o primeiro que ele tem.
        if !discovered_ollama.is_empty() {
            for m in models.iter_mut() {
                if m.endpoint == OLLAMA_CHAT_URL && !discovered_ollama.contains(&m.id) {
                    m.id = discovered_ollama[0].clone();
                }
            }
        }
        // Todos os modelos instalados no Ollama entram na lista.
        for id in discovered_ollama {
            if !models.iter().any(|m| m.id == id) {
                models.push(Model {
                    name: id.clone(),
                    id,
                    endpoint: OLLAMA_CHAT_URL.into(),
                    api_key: String::new(),
                    enabled: true,
                    api_style: default_api_style(),
                });
            }
        }
        // Catálogo de IAs: a desativar; ative depois de colar a chave de API.
        for m in ai_catalog() {
            if !models.iter().any(|x| x.id == m.id) {
                models.push(m);
            }
        }
        let app = ZeroApp {
            logo_texture: load_logo_texture(cc),
            shot_rx: None,
            screenshot,
            frame_nr: 0,
            lang,
            auto_update: stored.auto_update.unwrap_or(true),
            update_url: stored.update_url.unwrap_or_default(),
            update_status: String::new(),
            update_available: None,
            update_check_rx: None,
            last_update_check: None,
            models,
            add_name: String::new(),
            add_endpoint: String::new(),
            add_api_key: String::new(),
            chat: stored.chat.unwrap_or_default(),
            pending: Vec::new(),
            show_settings: false,
            input: String::new(),
            focus_input: true,
        };
        app.persist();
        app
    }

    fn persist(&self) {
        let stored = Stored {
            language: Some(self.lang.code().to_string()),
            auto_update: Some(self.auto_update),
            update_url: Some(self.update_url.clone()),
            models: Some(self.models.clone()),
            chat: Some(self.chat.clone()),
        };
        let _ = std::fs::write(settings_path(), serde_json::to_vec_pretty(&stored).unwrap_or_default());
    }

    fn send_message(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        self.chat.push(Message {
            role: "user".into(),
            model: None,
            text: text.to_string(),
            ts: now(),
        });
        if self.models.iter().filter(|m| m.enabled).count() == 0 {
            self.chat.push(Message {
                role: "system".into(),
                model: None,
                text: self.lang.strings().no_models_hint.into(),
                ts: now(),
            });
        } else {
            for model in self.models.iter().filter(|m| m.enabled).cloned() {
                let (tx, rx) = channel();
                let local_reply = self.local_reply_for(&model);
                self.pending.push(PendingAi { name: model.name.clone(), local_reply, rx });
                let history = self.chat.clone();
                let endpoint = model.endpoint.clone();
                let api_key = model.api_key.clone();
                let model_id = model.id.clone();
                let style = model.api_style.clone();
                let name = model.name.clone();
                std::thread::spawn(move || {
                    let _ = tx.send((name, chat_request(&endpoint, &api_key, &model_id, &history, &style)));
                });
            }
        }
        self.persist();
        self.focus_input = true;
    }

    /// Resposta local quando um modelo de API pública não tem chave de
    /// API: sem chave o endpoint responde 401/400, então o chat mostra
    /// esta mensagem em vez do erro cru. Modelos locais (Ollama) não
    /// precisam de chave.
    fn local_reply_for(&self, model: &Model) -> Option<String> {
        let is_local = model.endpoint.contains("localhost")
            || model.endpoint.contains("127.0.0.1")
            || model.endpoint == OLLAMA_CHAT_URL;
        if model.api_key.trim().is_empty() && !is_local {
            Some(String::from(self.lang.strings().no_key_reply).replace("{}", &model.name))
        } else {
            None
        }
    }

    fn poll_pending(&mut self) {
        let mut done: Vec<(usize, String, Result<String, String>, Option<String>)> = Vec::new();
        for (i, p) in self.pending.iter().enumerate() {
            match p.rx.try_recv() {
                Ok((name, res)) => done.push((i, name, res, p.local_reply.clone())),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => done.push((
                    i,
                    p.name.clone(),
                    Err("sem resposta: thread encerrada".to_string()),
                    p.local_reply.clone(),
                )),
            }
        }
        for (i, name, res, local_reply) in done.into_iter().rev() {
            self.pending.remove(i);
            // Sem chave de API o endpoint responde 401/400; mostra a
            // resposta local de aviso em vez do erro cru.
            let text = match res {
                Ok(t) => t,
                Err(e) => match local_reply {
                    Some(t) => t,
                    None => format!("[{}] {}", name, e),
                },
            };
            self.chat.push(Message {
                role: "ai".into(),
                model: Some(name),
                text,
                ts: now(),
            });
            self.persist();
        }
    }

    fn start_update_check(&mut self) {
        if self.update_check_rx.is_some() {
            return;
        }
        let url = self.update_url.trim().to_string();
        if url.is_empty() {
            self.update_status = self.lang.strings().update_no_url.to_string();
            self.last_update_check = Some(Instant::now());
            return;
        }
        self.update_status = self.lang.strings().update_checking.to_string();
        let (tx, rx) = channel();
        let url2 = url.clone();
        std::thread::spawn(move || {
            let _ = tx.send(check_for_update(&url2));
        });
        self.update_check_rx = Some((url, rx));
        self.last_update_check = Some(Instant::now());
    }

    fn poll_update_check(&mut self) {
        let taken = self.update_check_rx.take();
        if let Some((url, rx)) = taken {
            match rx.try_recv() {
                Ok(Ok((ver, link))) => {
                    let s = self.lang.strings();
                    if version_newer(&ver, APP_VERSION) {
                        self.update_available = link;
                        self.update_status = format!("{} {}", s.update_new, ver);
                    } else {
                        self.update_available = None;
                        self.update_status = s.update_current.to_string();
                    }
                }
                Ok(Err(e)) => {
                    self.update_status = format!("{} {}", self.lang.strings().update_fail, e);
                }
                Err(_) => {
                    self.update_check_rx = Some((url, rx));
                }
            }
        }
    }

    fn maybe_periodic_update_check(&mut self) {
        if !self.auto_update || self.update_check_rx.is_some() {
            return;
        }
        let due = self
            .last_update_check
            .map(|t| t.elapsed() >= Duration::from_secs(600))
            .unwrap_or(true);
        if due {
            self.start_update_check();
        }
    }

    // ---------------------------------------------------------------- ui

    fn header_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();

        // Left: the logo — the one generated with the Craft — or the Z
        // drawn here when it isn't available — + the app name.
        ui.horizontal(|ui| {
            if let Some(tex) = &self.logo_texture {
                ui.image(tex);
            } else {
                draw_z_logo(
                    ui,
                    Rect::from_center_size(ui.min_rect().left_top() + egui::vec2(15.0, 15.0), egui::vec2(30.0, 30.0)),
                    HIGHLIGHT_COLOR,
                );
            }
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Zero").size(20.0).color(TEXT_COLOR));
        });

        // Right: the config icon — the only control in the header.
        let icon_rect = Rect::from_center_size(
            ui.max_rect().right_top() + egui::vec2(-15.0, 15.0),
            egui::vec2(30.0, 30.0),
        );
        // Persistent id so the icon is also focusable by keyboard; the
        // accessible name and tooltip announce it to screen readers.
        let response = ui
            .interact(icon_rect, ui.make_persistent_id("config_icon"), egui::Sense::click())
            .accessible_name(s.settings)
            .on_hover_text(s.settings);
        draw_config_icon(ui, icon_rect, HIGHLIGHT_COLOR);
        if response.hovered() {
            ui.painter().rect_filled(
                icon_rect.expand(3.0),
                egui::CornerRadius::same(8),
                HIGHLIGHT_COLOR.gamma_multiply(0.12),
            );
        }
        if response.clicked() {
            self.show_settings = !self.show_settings;
            self.persist();
        }
    }

    fn chat_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();

        // The AIs in this chat: every enabled model is part of the thread.
        let ai_names: Vec<String> = self
            .models
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.name.clone())
            .collect();
        if !ai_names.is_empty() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(s.ai_roster).small().color(TEXT_COLOR));
                for name in &ai_names {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(name).small().strong().color(HIGHLIGHT_COLOR));
                }
            });
            ui.add_space(6.0);
        }

        // Main chat: all enabled models can answer the same thread.
        let chat_top = ui.available_height() - 40.0;
        egui::ScrollArea::vertical()
            .id_salt("chat")
            .stick_to_bottom(true)
            .max_height(chat_top)
            .show(ui, |ui| {
                // Keyboard navigation: Up/Down scroll the conversation
                // while the chat input is not focused (typing uses the arrows).
                if !self.show_settings && !ui.ctx().text_edit_focused() {
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        ui.scroll_with_delta(egui::vec2(0.0, 40.0));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        ui.scroll_with_delta(egui::vec2(0.0, -40.0));
                    }
                }
                for msg in &self.chat {
                    ui.add_space(5.0);
                    if msg.role == "user" {
                        ui.horizontal_centered(|ui| {
                            ui.label(egui::RichText::new(s.user_label).strong().size(13.0).color(HIGHLIGHT_COLOR));
                        });
                        ui.add(egui::Label::new(msg.text.clone()).wrap());
                    } else if msg.role == "ai" {
                        ui.label(
                            egui::RichText::new(msg.model.clone().unwrap_or_else(|| "IA".into()))
                                .strong()
                                .color(HIGHLIGHT_COLOR),
                        );
                        ui.add(egui::Label::new(msg.text.clone()).wrap());
                    } else {
                        ui.label(egui::RichText::new(msg.text.clone()).italics().weak());
                    }
                }
                if self.chat.is_empty() {
                    ui.label(egui::RichText::new(s.start_hint).color(HIGHLIGHT_COLOR));
                }
            });

        ui.add_space(8.0);

        // Input row.
        let mut send_now = false;
        let text_edit = ui.add(
            egui::TextEdit::singleline(&mut self.input)
                .desired_width(ui.available_width() - 70.0)
                .hint_text(egui::RichText::new(s.input_placeholder).color(HIGHLIGHT_COLOR)),
        );
        if self.focus_input {
            ui.memory_mut(|mem| mem.request_focus(text_edit.id));
            self.focus_input = false;
        }
        if text_edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            send_now = true;
            text_edit.request_focus();
        }
        if ui
            .add_enabled(!self.input.trim().is_empty(), egui::Button::new(s.send))
            .clicked()
        {
            send_now = true;
        }
        if send_now {
            let t = std::mem::take(&mut self.input);
            self.send_message(&t);
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();
        ui.heading(s.settings);
        if ui.button(s.close).clicked() {
            self.show_settings = false;
        }
        ui.separator();

        // Language.
        ui.horizontal(|ui| {
            ui.label(s.language);
            let mut sel = self.lang;
            egui::ComboBox::from_id_salt("language")
                .selected_text(sel.display_name())
                .show_ui(ui, |ui| {
                    for &l in Lang::ALL {
                        if ui.selectable_value(&mut sel, l, l.display_name()).clicked() {
                            sel = l;
                        }
                    }
                });
            if sel != self.lang {
                self.lang = sel;
                self.persist();
            }
        });
        ui.separator();

        // Models list.
        ui.heading(s.models);
        let mut remove: Option<usize> = None;
        for (i, m) in self.models.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut m.enabled, m.name.clone());
                ui.label(egui::RichText::new(&m.endpoint).weak());
                if ui.button(s.remove_model).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.models.remove(i);
            self.persist();
        }

        // Add a new model.
        ui.horizontal(|ui| {
            let mut name = self.add_name.clone();
            let mut endpoint = self.add_endpoint.clone();
            let mut key = self.add_api_key.clone();
            ui.add(egui::TextEdit::singleline(&mut name).desired_width(80.0).hint_text(s.model_name));
            ui.add(egui::TextEdit::singleline(&mut endpoint).desired_width(150.0).hint_text(s.model_endpoint));
            ui.add(egui::TextEdit::singleline(&mut key).desired_width(120.0).hint_text(s.model_api_key));
            if ui.button(s.add_model).clicked() && !name.trim().is_empty() {
                let name = name.trim().to_string();
                let endpoint = if endpoint.trim().is_empty() {
                    OLLAMA_CHAT_URL.to_string()
                } else {
                    endpoint.trim().to_string()
                };
                // Endpoints do Claude usam a API dele; o resto usa o
                // formato do Ollama/OpenAI.
                let style = if endpoint.contains("api.anthropic.com") {
                    "anthropic".to_string()
                } else {
                    default_api_style()
                };
                self.models.push(Model {
                    id: name.clone(),
                    name: name.clone(),
                    endpoint,
                    api_key: key.trim().to_string(),
                    enabled: true,
                    api_style: style,
                });
                self.add_name.clear();
                self.add_endpoint.clear();
                self.add_api_key.clear();
                self.persist();
            }
        });
        ui.separator();

        // Auto-update by AI.
        if ui.checkbox(&mut self.auto_update, s.auto_update).changed() {
            self.persist();
        }
        ui.horizontal(|ui| {
            ui.label(s.update_url);
            ui.add(
                egui::TextEdit::singleline(&mut self.update_url)
                    .desired_width(ui.available_width() - 80.0)
                    .hint_text("https://…/latest.json"),
            );
        });
        if ui.button(s.check_updates).clicked() {
            self.start_update_check();
        }
        ui.label(self.update_status.as_str());
        if let Some(url) = self.update_available.clone() {
            if ui.button(s.update_open).clicked() {
                open_url(&url);
            }
        }
    }
}

impl App for ZeroApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        // Keyboard navigation: Escape closes the settings panel.
        if self.show_settings && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_settings = false;
        }

        // Screenshot mode: after ~120 frames, ask the integration to read the
        // rendered pixels, save the window's picture as PNG and exit.
        if let Some(path) = self.screenshot.clone() {
            if self.frame_nr >= FRAMES_BEFORE_SHOT && self.shot_rx.is_none() {
                let (tx, rx) = channel::<std::sync::Arc<egui::ColorImage>>();
                self.shot_rx = Some(rx);
                ui.ctx().request_screenshot(move |img| {
                    let _ = tx.send(img);
                });
            }
            if let Some(rx) = self.shot_rx.take() {
                match rx.try_recv() {
                    Ok(img) => {
                        if let Err(e) = save_image_png(&img, &path) {
                            eprintln!("zero: falha ao salvar {path:?}: {e}");
                        }
                        std::process::exit(0);
                    }
                    Err(TryRecvError::Empty) => self.shot_rx = Some(rx),
                    Err(TryRecvError::Disconnected) => {
                        eprintln!("zero: screenshot não capturado");
                        std::process::exit(1);
                    }
                }
            }
        }
        self.frame_nr += 1;

        if self.screenshot.is_some() {
            // Keep drawing even while unfocused, or the picture never comes.
            ui.ctx().request_repaint();
        }

        self.poll_pending();
        self.poll_update_check();
        self.maybe_periodic_update_check();

        egui::Panel::top("zero_header").default_size(44.0).show(ui, |ui| {
            self.header_ui(ui);
        });

        if self.show_settings {
            egui::Panel::left("zero_settings").show(ui, |ui| {
                self.settings_ui(ui);
            });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            self.chat_ui(ui);
        });
    }
}

// ---------------------------------------------------------------- main

fn parse_args() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--screenshot" {
            if let Some(p) = args.get(i + 1) {
                return Some(p.clone().into());
            }
            i += 1;
        } else if let Some(p) = args[i].strip_prefix("--screenshot=") {
            return Some(p.into());
        }
        i += 1;
    }
    None
}

fn main() {
    let screenshot = parse_args();

    let viewport = egui::ViewportBuilder::default()
        .with_title("Zero")
        .with_inner_size([920.0, 650.0]);
    let viewport = if screenshot.is_some() {
        viewport.with_active(false)
    } else {
        viewport
    };
    #[cfg(windows)]
    let viewport = if screenshot.is_some() {
        viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom)
    } else {
        viewport
    };

    #[allow(unused_mut)]
    let mut native_options = NativeOptions {
        viewport,
        renderer: Renderer::Glow,
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        native_options.event_loop_builder = Some(Box::new(|elb| {
            elb.with_activate_ignoring_other_apps(false);
        }));
    }

    let creator: eframe::AppCreator = Box::new(|cc: &CreationContext| {
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            Box::new(ZeroApp::new(cc, screenshot.clone())) as Box<dyn eframe::App>,
        )
    });
    if let Err(e) = run_native("zero", native_options, creator) {
        eprintln!("zero: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O bug que a gente corrige: o app mandava o id "local-model" para o
    /// Ollama, mas o Ollama só tem os modelos que ele tem instalados (ex.:
    /// qwen3:8b), por isso o chat respondia "modelo não encontrado".
    #[test]
    fn ollama_responde_com_modelo_real() {
        let discovered = ollama_models();
        if discovered.is_empty() {
            eprintln!("zero: Ollama não respondeu em {}", OLLAMA_TAGS_URL);
            return;
        }
        let id = discovered[0].clone();
        assert_ne!(id, "local-model", "o app ainda usa o id inválido 'local-model'");
        let history = vec![Message {
            role: "user".into(),
            model: None,
            text: "dica curta".into(),
            ts: now(),
        }];
        let res = chat_request(OLLAMA_CHAT_URL, "", &id, &history, "openai");
        assert!(res.is_ok(), "ollama não respondeu: {res:?}");
    }
}
