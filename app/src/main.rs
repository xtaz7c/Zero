// Zero — Assistente de sistema Linux leve, rápido e otimizado.
//
// Lê hardware e software, ajuda com atualizações, desinstalações,
// otimizações e se auto-atualiza quando sai release no GitHub.
//
// Uso:
//   zero                        abre em fullscreen
//   zero --screenshot out.png   desenha ~90 frames, salva PNG e sai

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use eframe::{run_native, App, CreationContext, Frame, NativeOptions, Renderer};
use serde::{Deserialize, Serialize};

const APP_VERSION: &str = "1.0.0";
const FRAMES_BEFORE_SHOT: u64 = 90;
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/xtaz7c/Zero/releases/latest";

// ---------------------------------------------------------------- tema

const BG_COLOR: Color32 = Color32::BLACK;
const TEXT_COLOR: Color32 = Color32::from_rgb(0xEE, 0xEE, 0xEE);
const HIGHLIGHT_COLOR: Color32 = Color32::from_rgb(0xE9, 0x1E, 0x63); // rosa-vermelho
const MUTED_COLOR: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);

fn theme_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = BG_COLOR;
    v.window_fill = BG_COLOR;
    v.extreme_bg_color = BG_COLOR;
    v.faint_bg_color = BG_COLOR;
    v.text_edit_bg_color = Some(Color32::from_rgb(0x12, 0x12, 0x12));
    v.override_text_color = Some(TEXT_COLOR);
    v.weak_text_color = Some(MUTED_COLOR);
    v.selection.bg_fill = HIGHLIGHT_COLOR.gamma_multiply(0.45);
    v.selection.stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_COLOR);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, HIGHLIGHT_COLOR);
    v.widgets.active.fg_stroke = egui::Stroke::new(2.0, HIGHLIGHT_COLOR);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v
}

// ---------------------------------------------------------------- strings (pt-BR focado + en)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    PtBr,
    En,
}

#[derive(Clone, Copy)]
struct Strings {
    settings: &'static str,
    close: &'static str,
    language: &'static str,
    auto_update: &'static str,
    check_updates: &'static str,
    update_checking: &'static str,
    update_current: &'static str,
    update_new: &'static str,
    update_fail: &'static str,
    update_open: &'static str,
    greeting: &'static str,
    system_info: &'static str,
    hardware: &'static str,
    software: &'static str,
    suggestions: &'static str,
    input_placeholder: &'static str,
    send: &'static str,
    refresh: &'static str,
    no_suggestions: &'static str,
}

impl Lang {
    const ALL: &'static [Lang] = &[Lang::PtBr, Lang::En];

    fn code(self) -> &'static str {
        match self {
            Lang::PtBr => "pt_BR",
            Lang::En => "en",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Lang::PtBr => "Português (Brasil)",
            Lang::En => "English",
        }
    }

    fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.iter().copied().find(|l| l.code() == code)
    }

    fn strings(self) -> Strings {
        match self {
            Lang::PtBr => Strings {
                settings: "Configurações",
                close: "Fechar",
                language: "Idioma",
                auto_update: "Atualização automática",
                check_updates: "Verificar atualizações",
                update_checking: "Verificando…",
                update_current: "Você está na versão mais recente",
                update_new: "Nova versão disponível:",
                update_fail: "Falha ao verificar:",
                update_open: "Abrir release",
                greeting: "Olá {}, como posso te ajudar?",
                system_info: "Informações do sistema",
                hardware: "Hardware",
                software: "Software",
                suggestions: "Sugestões de otimização",
                input_placeholder: "Pergunte sobre atualizações, desinstalar, otimizar…",
                send: "Enviar",
                refresh: "Atualizar dados",
                no_suggestions: "Nenhuma sugestão crítica no momento. Seu sistema parece saudável.",
            },
            Lang::En => Strings {
                settings: "Settings",
                close: "Close",
                language: "Language",
                auto_update: "Automatic updates",
                check_updates: "Check for updates",
                update_checking: "Checking…",
                update_current: "You are on the latest version",
                update_new: "New version available:",
                update_fail: "Failed to check:",
                update_open: "Open release",
                greeting: "Hello {}, how can I help you?",
                system_info: "System information",
                hardware: "Hardware",
                software: "Software",
                suggestions: "Optimization suggestions",
                input_placeholder: "Ask about updates, uninstall, optimize…",
                send: "Send",
                refresh: "Refresh data",
                no_suggestions: "No critical suggestions right now. Your system looks healthy.",
            },
        }
    }
}

// ---------------------------------------------------------------- dados do sistema

#[derive(Clone, Default)]
struct SysInfo {
    username: String,
    hostname: String,
    kernel: String,
    os: String,
    cpu_model: String,
    cpu_cores: String,
    mem_total: String,
    mem_used: String,
    mem_pct: f32,
    disk_root: String,
    uptime: String,
    packages: String,
}

fn read_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else {
        format!("{:.0} KB", b / KB)
    }
}

fn gather_sysinfo() -> SysInfo {
    let mut info = SysInfo::default();

    info.username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "usuário".into());

    info.hostname = read_file("/etc/hostname")
        .map(|s| s.trim().to_string())
        .or_else(|| run_cmd("hostname", &[]))
        .unwrap_or_else(|| "localhost".into());

    info.kernel = read_file("/proc/version")
        .and_then(|v| v.split_whitespace().nth(2).map(|s| s.to_string()))
        .or_else(|| run_cmd("uname", &["-r"]))
        .unwrap_or_else(|| "?".into());

    if let Some(os_release) = read_file("/etc/os-release") {
        for line in os_release.lines() {
            if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                info.os = val.trim_matches('"').to_string();
                break;
            }
        }
    }
    if info.os.is_empty() {
        info.os = "Linux".into();
    }

    if let Some(cpuinfo) = read_file("/proc/cpuinfo") {
        let mut model = None;
        let mut cores = 0u32;
        for line in cpuinfo.lines() {
            if line.starts_with("model name") || line.starts_with("Hardware") {
                if model.is_none() {
                    model = line.split(':').nth(1).map(|s| s.trim().to_string());
                }
            }
            if line.starts_with("processor") {
                cores += 1;
            }
        }
        info.cpu_model = model.unwrap_or_else(|| "CPU".into());
        info.cpu_cores = if cores > 0 {
            format!("{cores} núcleos")
        } else {
            "?".into()
        };
    }

    if let Some(meminfo) = read_file("/proc/meminfo") {
        let mut total_kb = 0u64;
        let mut avail_kb = 0u64;
        for line in meminfo.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                total_kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
            if let Some(rest) = line.strip_prefix("MemAvailable:") {
                avail_kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }
        let used_kb = total_kb.saturating_sub(avail_kb);
        info.mem_total = format_bytes(total_kb * 1024);
        info.mem_used = format_bytes(used_kb * 1024);
        info.mem_pct = if total_kb > 0 {
            (used_kb as f32 / total_kb as f32) * 100.0
        } else {
            0.0
        };
    }

    if let Some(df) = run_cmd("df", &["-h", "--output=size,used,avail,pcent", "/"]) {
        if let Some(line) = df.lines().nth(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                info.disk_root = format!(
                    "{} total · {} usados · {} livres ({})",
                    parts[0], parts[1], parts[2], parts[3]
                );
            }
        }
    }

    if let Some(uptime) = read_file("/proc/uptime") {
        if let Some(secs_s) = uptime.split_whitespace().next() {
            if let Ok(secs) = secs_s.parse::<f64>() {
                let days = (secs / 86400.0) as u64;
                let hours = ((secs % 86400.0) / 3600.0) as u64;
                let mins = ((secs % 3600.0) / 60.0) as u64;
                info.uptime = if days > 0 {
                    format!("{days}d {hours}h {mins}m")
                } else if hours > 0 {
                    format!("{hours}h {mins}m")
                } else {
                    format!("{mins}m")
                };
            }
        }
    }

    if run_cmd("which", &["apt"]).is_some() {
        if let Some(n) = run_cmd("dpkg", &["--get-selections"]) {
            let count = n.lines().filter(|l| l.contains("install")).count();
            info.packages = format!("apt · {count} pacotes");
        } else {
            info.packages = "apt".into();
        }
    } else if run_cmd("which", &["pacman"]).is_some() {
        if let Some(n) = run_cmd("pacman", &["-Q"]) {
            let count = n.lines().count();
            info.packages = format!("pacman · {count} pacotes");
        } else {
            info.packages = "pacman".into();
        }
    } else if run_cmd("which", &["dnf"]).is_some() || run_cmd("which", &["rpm"]).is_some() {
        if let Some(n) = run_cmd("rpm", &["-qa"]) {
            let count = n.lines().count();
            info.packages = format!("rpm · {count} pacotes");
        } else {
            info.packages = "rpm/dnf".into();
        }
    } else {
        info.packages = "desconhecido".into();
    }

    info
}

fn generate_suggestions(info: &SysInfo) -> Vec<String> {
    let mut s = Vec::new();

    if info.mem_pct > 85.0 {
        s.push(format!(
            "Memória em uso alto ({:.0}%). Feche apps pesados ou considere aumentar o swap.",
            info.mem_pct
        ));
    } else if info.mem_pct > 70.0 {
        s.push(format!(
            "Memória em {:.0}%. Monitore processos com `htop` ou `btop`.",
            info.mem_pct
        ));
    }

    if info.disk_root.contains("9") && info.disk_root.contains("%") {
        s.push("Disco raiz com pouco espaço. Rode `sudo apt autoremove` e limpe caches.".into());
    }

    s.push("Mantenha o sistema atualizado: `sudo apt update && sudo apt upgrade` (ou equivalente).".into());
    s.push("Para remover pacotes órfãos: `sudo apt autoremove --purge`.".into());
    s.push("Verifique serviços desnecessários na inicialização com `systemctl list-unit-files --state=enabled`.".into());

    s
}

// ---------------------------------------------------------------- update check

fn check_github_update() -> Result<(String, Option<String>), String> {
    let resp = ureq::get(GITHUB_RELEASES_URL)
        .set("User-Agent", "Zero-App/1.0")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|e| e.to_string())?;

    let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let html_url = v
        .get("html_url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    if tag.is_empty() {
        return Err("resposta sem tag_name".into());
    }
    Ok((tag, html_url))
}

fn version_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let r = parse(remote);
    let l = parse(local);
    for i in 0..r.len().max(l.len()) {
        let rv = r.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if rv > lv {
            return true;
        }
        if rv < lv {
            return false;
        }
    }
    false
}

// ---------------------------------------------------------------- settings

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    language: Option<String>,
    auto_update: Option<bool>,
}

fn settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("settings.json")))
        .unwrap_or_else(|| PathBuf::from("settings.json"))
}

// ---------------------------------------------------------------- ícones

fn draw_gear_icon(ui: &egui::Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let c = rect.center();
    let r = rect.width() * 0.38;
    let n = 8;
    let tooth_w = r * 0.32;
    let tooth_h = r * 0.42;
    for i in 0..n {
        let a = i as f32 * std::f32::consts::TAU / n as f32;
        let d = Vec2::new(a.cos(), a.sin());
        painter.rect_filled(
            Rect::from_center_size(c + d * r, Vec2::new(tooth_w, tooth_h)),
            egui::CornerRadius::same(1),
            color,
        );
    }
    painter.circle_filled(c, r * 0.48, color);
    painter.circle_filled(c, r * 0.22, BG_COLOR);
}

fn draw_update_icon(ui: &egui::Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let c = rect.center();
    let r = rect.width() * 0.36;
    painter.circle_stroke(c, r, egui::Stroke::new(2.2, color));
    let tip = c + Vec2::new(0.0, -r * 0.55);
    painter.line_segment([c + Vec2::new(-r * 0.35, -r * 0.1), tip], egui::Stroke::new(2.2, color));
    painter.line_segment([c + Vec2::new(r * 0.35, -r * 0.1), tip], egui::Stroke::new(2.2, color));
}

// ---------------------------------------------------------------- app

struct ZeroApp {
    shot_rx: Option<Receiver<std::sync::Arc<egui::ColorImage>>>,
    screenshot: Option<PathBuf>,
    frame_nr: u64,
    start: Instant,
    lang: Lang,
    auto_update: bool,
    show_settings: bool,
    sys: SysInfo,
    suggestions: Vec<String>,
    update_status: String,
    update_available: Option<String>,
    update_check_rx: Option<Receiver<Result<(String, Option<String>), String>>>,
    last_update_check: Option<Instant>,
    input: String,
    chat: Vec<(String, String)>,
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

        let sys = gather_sysinfo();
        let suggestions = generate_suggestions(&sys);

        ZeroApp {
            shot_rx: None,
            screenshot,
            frame_nr: 0,
            start: Instant::now(),
            lang,
            auto_update: stored.auto_update.unwrap_or(true),
            show_settings: false,
            sys,
            suggestions,
            update_status: String::new(),
            update_available: None,
            update_check_rx: None,
            last_update_check: None,
            input: String::new(),
            chat: Vec::new(),
            focus_input: true,
        }
    }

    fn save_settings(&self) {
        let stored = Stored {
            language: Some(self.lang.code().to_string()),
            auto_update: Some(self.auto_update),
        };
        if let Ok(json) = serde_json::to_string_pretty(&stored) {
            let _ = std::fs::write(settings_path(), json);
        }
    }

    fn refresh_sys(&mut self) {
        self.sys = gather_sysinfo();
        self.suggestions = generate_suggestions(&self.sys);
    }

    fn start_update_check(&mut self) {
        if self.update_check_rx.is_some() {
            return;
        }
        self.update_status = self.lang.strings().update_checking.to_string();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(check_github_update());
        });
        self.update_check_rx = Some(rx);
        self.last_update_check = Some(Instant::now());
    }

    fn poll_update_check(&mut self) {
        let taken = self.update_check_rx.take();
        if let Some(rx) = taken {
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
                Err(TryRecvError::Empty) => {
                    self.update_check_rx = Some(rx);
                }
                Err(TryRecvError::Disconnected) => {}
            }
        }
    }

    fn maybe_auto_update_check(&mut self) {
        if !self.auto_update || self.update_check_rx.is_some() {
            return;
        }
        let due = self
            .last_update_check
            .map(|t| t.elapsed() >= Duration::from_secs(3600))
            .unwrap_or(true);
        if due {
            self.start_update_check();
        }
    }

    fn handle_user_message(&mut self, text: &str) {
        let t = text.trim();
        if t.is_empty() {
            return;
        }
        self.chat.push(("user".into(), t.to_string()));

        let lower = t.to_lowercase();
        let reply = if lower.contains("atualiz") || lower.contains("update") || lower.contains("upgrade") {
            "Para atualizar o sistema:\n\n• Debian/Ubuntu: `sudo apt update && sudo apt upgrade -y`\n• Arch: `sudo pacman -Syu`\n• Fedora: `sudo dnf upgrade`\n\nPosso verificar se há nova versão do próprio Zero também — use o ícone de atualização no canto superior direito.".into()
        } else if lower.contains("desinstal") || lower.contains("remover") || lower.contains("uninstall") || lower.contains("remove") {
            "Para desinstalar pacotes:\n\n• apt: `sudo apt remove <pacote>` ou `sudo apt purge <pacote>`\n• Depois limpe: `sudo apt autoremove`\n• Arch: `sudo pacman -Rns <pacote>`\n\nDiga o nome do programa que quer remover que eu monto o comando.".into()
        } else if lower.contains("otimiz") || lower.contains("optim") || lower.contains("lento") || lower.contains("performance") {
            let mut r = String::from("Sugestões rápidas de otimização:\n\n");
            for (i, s) in self.suggestions.iter().enumerate() {
                r.push_str(&format!("{}. {}\n", i + 1, s));
            }
            r.push_str("\nTambém recomendo instalar `btop` ou `htop` para monitorar em tempo real.");
            r
        } else if lower.contains("memória") || lower.contains("ram") || lower.contains("memory") {
            format!(
                "Memória: {} usados de {} ({:.0}%).\n\nPara ver o que mais consome: `ps aux --sort=-%mem | head -15`",
                self.sys.mem_used, self.sys.mem_total, self.sys.mem_pct
            )
        } else if lower.contains("disco") || lower.contains("disk") || lower.contains("espaço") || lower.contains("space") {
            format!(
                "Disco raiz: {}\n\nPara liberar espaço:\n• `sudo apt autoremove --purge`\n• Limpar journal: `sudo journalctl --vacuum-time=7d`\n• Cache de pacotes: `sudo apt clean`",
                self.sys.disk_root
            )
        } else if lower.contains("hardware") || lower.contains("info") || lower.contains("sistema") {
            format!(
                "Sistema: {}\nKernel: {}\nCPU: {} ({})\nMemória: {} / {} ({:.0}%)\nDisco: {}\nUptime: {}\nPacotes: {}",
                self.sys.os,
                self.sys.kernel,
                self.sys.cpu_model,
                self.sys.cpu_cores,
                self.sys.mem_used,
                self.sys.mem_total,
                self.sys.mem_pct,
                self.sys.disk_root,
                self.sys.uptime,
                self.sys.packages
            )
        } else {
            format!(
                "Posso ajudar com:\n• Atualizações do sistema e do Zero\n• Desinstalar programas\n• Otimizações e limpeza\n• Informações de hardware e software\n\nExemplos: \"como atualizar?\", \"liberar espaço\", \"mostrar hardware\", \"otimizar\".\n\nSeu usuário: {} · Host: {}",
                self.sys.username, self.sys.hostname
            )
        };

        self.chat.push(("zero".into(), reply));
    }

    fn header_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();
        let h = 44.0;
        ui.set_min_height(h);

        ui.horizontal(|ui| {
            ui.add_space(8.0);

            let gear_size = Vec2::splat(28.0);
            let (gear_rect, gear_resp) = ui.allocate_exact_size(gear_size, Sense::click());
            let gear_color = if gear_resp.hovered() {
                HIGHLIGHT_COLOR
            } else {
                TEXT_COLOR
            };
            draw_gear_icon(ui, gear_rect, gear_color);
            if gear_resp.clicked() {
                self.show_settings = !self.show_settings;
            }
            gear_resp.on_hover_text(s.settings);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);

                let upd_size = Vec2::splat(28.0);
                let (upd_rect, upd_resp) = ui.allocate_exact_size(upd_size, Sense::click());
                let upd_color = if self.update_available.is_some() {
                    HIGHLIGHT_COLOR
                } else if upd_resp.hovered() {
                    HIGHLIGHT_COLOR
                } else {
                    TEXT_COLOR
                };
                draw_update_icon(ui, upd_rect, upd_color);
                if upd_resp.clicked() {
                    self.start_update_check();
                }
                upd_resp.on_hover_text(s.check_updates);

                if !self.update_status.is_empty() {
                    ui.label(
                        egui::RichText::new(&self.update_status)
                            .size(12.0)
                            .color(MUTED_COLOR),
                    );
                }
            });
        });
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();
        ui.heading(s.settings);
        if ui.button(s.close).clicked() {
            self.show_settings = false;
        }
        ui.separator();

        ui.label(s.language);
        for &lang in Lang::ALL {
            if ui
                .selectable_label(self.lang == lang, lang.display_name())
                .clicked()
            {
                self.lang = lang;
                self.save_settings();
            }
        }

        ui.add_space(12.0);
        if ui
            .checkbox(&mut self.auto_update, s.auto_update)
            .changed()
        {
            self.save_settings();
        }

        ui.add_space(12.0);
        if ui.button(s.check_updates).clicked() {
            self.start_update_check();
        }
        if !self.update_status.is_empty() {
            ui.label(&self.update_status);
        }
        if let Some(url) = &self.update_available {
            if ui.button(s.update_open).clicked() {
                let _ = Command::new("xdg-open").arg(url).spawn();
            }
        }

        ui.add_space(16.0);
        ui.label(egui::RichText::new(format!("Zero v{APP_VERSION}")).weak());
    }

    fn system_panel(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();

        ui.horizontal(|ui| {
            ui.heading(s.system_info);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(s.refresh).clicked() {
                    self.refresh_sys();
                }
            });
        });
        ui.separator();

        ui.collapsing(s.hardware, |ui| {
            ui.label(format!("CPU: {} ({})", self.sys.cpu_model, self.sys.cpu_cores));
            ui.label(format!(
                "Memória: {} / {} ({:.0}%)",
                self.sys.mem_used, self.sys.mem_total, self.sys.mem_pct
            ));
            ui.label(format!("Disco /: {}", self.sys.disk_root));
            ui.label(format!("Uptime: {}", self.sys.uptime));
        });

        ui.collapsing(s.software, |ui| {
            ui.label(format!("SO: {}", self.sys.os));
            ui.label(format!("Kernel: {}", self.sys.kernel));
            ui.label(format!("Host: {}", self.sys.hostname));
            ui.label(format!("Pacotes: {}", self.sys.packages));
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new(s.suggestions).strong().color(HIGHLIGHT_COLOR));
        if self.suggestions.is_empty() {
            ui.label(s.no_suggestions);
        } else {
            for sug in &self.suggestions {
                ui.label(format!("• {sug}"));
            }
        }
    }

    fn chat_panel(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .max_height(ui.available_height() - 48.0)
            .show(ui, |ui| {
                for (role, text) in &self.chat {
                    ui.add_space(6.0);
                    if role == "user" {
                        ui.label(
                            egui::RichText::new("Você")
                                .strong()
                                .color(HIGHLIGHT_COLOR)
                                .size(13.0),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Zero")
                                .strong()
                                .color(HIGHLIGHT_COLOR)
                                .size(13.0),
                        );
                    }
                    ui.label(text);
                }
            });

        ui.add_space(6.0);

        let mut send = false;
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .desired_width(ui.available_width() - 80.0)
                    .hint_text(s.input_placeholder),
            );
            if self.focus_input {
                edit.request_focus();
                self.focus_input = false;
            }
            if edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send = true;
            }
            if ui
                .add_enabled(!self.input.trim().is_empty(), egui::Button::new(s.send))
                .clicked()
            {
                send = true;
            }
        });

        if send {
            let msg = std::mem::take(&mut self.input);
            self.handle_user_message(&msg);
        }
    }
}

impl App for ZeroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if let Some(path) = self.screenshot.clone() {
            if self.frame_nr >= FRAMES_BEFORE_SHOT + 10 {
                let _ = path;
                std::process::exit(0);
            }
            ctx.request_repaint();
        }
        self.frame_nr += 1;

        self.poll_update_check();
        self.maybe_auto_update_check();

        if self.show_settings && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_settings = false;
        }

        egui::TopBottomPanel::top("header")
            .exact_height(48.0)
            .frame(egui::Frame::NONE.fill(BG_COLOR).inner_margin(egui::Margin::symmetric(4, 4)))
            .show(ctx, |ui| {
                self.header_ui(ui);
            });

        if self.show_settings {
            egui::SidePanel::left("settings")
                .default_width(280.0)
                .frame(egui::Frame::NONE.fill(Color32::from_rgb(0x0A, 0x0A, 0x0A)).inner_margin(12.0))
                .show(ctx, |ui| {
                    self.settings_ui(ui);
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_COLOR).inner_margin(16.0))
            .show(ctx, |ui| {
                let wide = ui.available_width() > 900.0;

                if wide {
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            let circle_h = 260.0;
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), circle_h),
                                Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            let t = self.start.elapsed().as_secs_f32();
                            let pulse = 1.0 + 0.09 * (t * 1.55).sin();
                            let center = rect.center() + Vec2::new(0.0, -12.0);
                            let r = 68.0 * pulse;

                            for i in (1..=4).rev() {
                                let expand = i as f32 * 9.0;
                                let alpha = 16u8.saturating_sub(i as u8 * 3);
                                painter.circle_filled(
                                    center,
                                    r + expand,
                                    Color32::from_rgba_unmultiplied(0xE9, 0x1E, 0x63, alpha),
                                );
                            }
                            painter.circle_filled(center, r, HIGHLIGHT_COLOR.gamma_multiply(0.9));
                            painter.circle_stroke(
                                center,
                                r,
                                egui::Stroke::new(2.4, Color32::from_rgb(0xFF, 0x5C, 0x8A)),
                            );
                            painter.circle_stroke(
                                center,
                                r * 0.7,
                                egui::Stroke::new(1.4, Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
                            );
                            painter.circle_filled(center, 5.5, Color32::WHITE);

                            let s = self.lang.strings();
                            let greeting = s.greeting.replace("{}", &self.sys.username);
                            painter.text(
                                Pos2::new(center.x, center.y + r + 28.0),
                                egui::Align2::CENTER_TOP,
                                greeting,
                                egui::FontId::proportional(19.0),
                                TEXT_COLOR,
                            );

                            ui.ctx().request_repaint();

                            ui.add_space(8.0);
                            self.system_panel(ui);
                        });

                        cols[1].vertical(|ui| {
                            ui.heading("Assistente");
                            ui.separator();
                            self.chat_panel(ui);
                        });
                    });
                } else {
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 220.0),
                        Sense::hover(),
                    );
                    let painter = ui.painter_at(rect);
                    let t = self.start.elapsed().as_secs_f32();
                    let pulse = 1.0 + 0.09 * (t * 1.55).sin();
                    let center = rect.center();
                    let r = 58.0 * pulse;

                    for i in (1..=3).rev() {
                        let expand = i as f32 * 8.0;
                        let alpha = 14u8.saturating_sub(i as u8 * 3);
                        painter.circle_filled(
                            center,
                            r + expand,
                            Color32::from_rgba_unmultiplied(0xE9, 0x1E, 0x63, alpha),
                        );
                    }
                    painter.circle_filled(center, r, HIGHLIGHT_COLOR.gamma_multiply(0.9));
                    painter.circle_stroke(
                        center,
                        r,
                        egui::Stroke::new(2.2, Color32::from_rgb(0xFF, 0x5C, 0x8A)),
                    );
                    painter.circle_filled(center, 5.0, Color32::WHITE);

                    let s = self.lang.strings();
                    let greeting = s.greeting.replace("{}", &self.sys.username);
                    painter.text(
                        Pos2::new(center.x, center.y + r + 22.0),
                        egui::Align2::CENTER_TOP,
                        greeting,
                        egui::FontId::proportional(17.0),
                        TEXT_COLOR,
                    );
                    ui.ctx().request_repaint();

                    ui.add_space(6.0);
                    self.system_panel(ui);
                    ui.add_space(10.0);
                    ui.heading("Assistente");
                    ui.separator();
                    self.chat_panel(ui);
                }
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
        } else if let Some(p) = args[i].strip_prefix("--screenshot=") {
            return Some(p.into());
        }
        i += 1;
    }
    None
}

fn main() {
    let screenshot = parse_args();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Zero")
        .with_fullscreen(screenshot.is_none())
        .with_inner_size([1100.0, 720.0]);

    if screenshot.is_some() {
        viewport = viewport.with_active(false).with_fullscreen(false);
    }

    let native_options = NativeOptions {
        viewport,
        renderer: Renderer::Glow,
        ..Default::default()
    };

    let creator: eframe::AppCreator<'_> = Box::new(move |cc: &CreationContext| {
        Ok(Box::new(ZeroApp::new(cc, screenshot.clone())) as Box<dyn App>)
    });

    if let Err(e) = run_native("Zero", native_options, creator) {
        eprintln!("zero: {e}");
    }
}
