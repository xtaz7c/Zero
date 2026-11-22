// Zero — Assistente de sistema Linux leve, rápido e otimizado.
mod theme;
mod strings;
mod sysinfo;
mod update;

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

type UpdateCheckResult = Result<(String, Option<String>), String>;
type UpdateCheckReceiver = Receiver<UpdateCheckResult>;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use eframe::{run_native, App, CreationContext, Frame, NativeOptions, Renderer};
use serde::{Deserialize, Serialize};

use strings::Lang;
use sysinfo::{gather_sysinfo, generate_suggestions, uninstall_command, SysInfo};
use theme::{theme_visuals, BG_COLOR, HIGHLIGHT_COLOR, MUTED_COLOR, TEXT_COLOR};
use update::{check_github_update, version_newer, APP_VERSION};

const FRAMES_BEFORE_SHOT: u64 = 90;
/// Limite de mensagens no histórico do chat (evita crescimento ilimitado de RAM).
const CHAT_CAP: usize = 50;

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
            egui::Rounding::same(1.0),
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
    painter.circle_stroke(c, r, egui::Stroke::new(2.2_f32, color));
    let tip = c + Vec2::new(0.0, -r * 0.55);
    painter.line_segment([c + Vec2::new(-r * 0.35, -r * 0.1), tip], egui::Stroke::new(2.2_f32, color));
    painter.line_segment([c + Vec2::new(r * 0.35, -r * 0.1), tip], egui::Stroke::new(2.2_f32, color));
}

struct ZeroApp {
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
    update_check_rx: Option<UpdateCheckReceiver>,
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
        let lang = stored.language.as_deref().and_then(Lang::from_code).unwrap_or(Lang::PtBr);
        let sys = gather_sysinfo();
        let suggestions = generate_suggestions(&sys);
        ZeroApp {
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
            chat: Vec::with_capacity(CHAT_CAP),
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

    fn trim_chat(&mut self) {
        if self.chat.len() > CHAT_CAP {
            let drain = self.chat.len() - CHAT_CAP;
            self.chat.drain(0..drain);
        }
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
                Err(TryRecvError::Empty) => self.update_check_rx = Some(rx),
                Err(TryRecvError::Disconnected) => {}
            }
        }
    }

    fn maybe_auto_update_check(&mut self) {
        if !self.auto_update || self.update_check_rx.is_some() {
            return;
        }
        let due = self.last_update_check.map(|t| t.elapsed() >= Duration::from_secs(3600)).unwrap_or(true);
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
        self.trim_chat();
        let lower = t.to_lowercase();
        let reply = if lower.contains("atualiz") || lower.contains("update") || lower.contains("upgrade") {
            let base = match self.sys.pkg_manager.as_str() {
                "apt" => "sudo apt update && sudo apt upgrade -y",
                "pacman" => "sudo pacman -Syu",
                "dnf" => "sudo dnf upgrade",
                _ => "# use o gerenciador de pacotes da sua distro",
            };
            format!("Para atualizar o sistema:\n\n`{base}`\n\nPara o Zero, use o ícone de atualização no canto superior direito.")
        } else if lower.contains("desinstal") || lower.contains("remover") || lower.contains("uninstall") || lower.contains("remove") {
            let words: Vec<&str> = t.split_whitespace().collect();
            let skip = ["desinstalar","desinstale","remover","remova","uninstall","remove","o","a","os","as","do","da","de","the","package","pacote","programa","app"];
            let pkg = words.iter()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_'))
                .find(|w| { let l = w.to_lowercase(); !skip.contains(&l.as_str()) && w.len() > 1 })
                .unwrap_or("");
            if !pkg.is_empty() {
                let cmd = uninstall_command(&self.sys.pkg_manager, pkg);
                format!("Para remover **{pkg}**:\n\n`{cmd}`\n\nRevise antes de executar.")
            } else {
                "Diga o nome do pacote. Ex: \"desinstalar vlc\"".into()
            }
        } else if lower.contains("otimiz") || lower.contains("optim") || lower.contains("lento") || lower.contains("performance") {
            let mut r = String::from("Sugestões de otimização:\n\n");
            for (i, s) in self.suggestions.iter().enumerate() {
                r.push_str(&format!("{}. {}\n", i + 1, s));
            }
            if !self.sys.top_procs.is_empty() {
                r.push_str("\nMaiores consumidores de RAM:\n");
                for p in self.sys.top_procs.iter().take(5) {
                    r.push_str(&format!("• {} — {:.0} MB (pid {})\n", p.name, p.mem_mb, p.pid));
                }
            }
            r
        } else if lower.contains("memória") || lower.contains("ram") || lower.contains("memory") {
            let mut r = format!("Memória: {} usados de {} ({:.0}%).\n\n", self.sys.mem_used, self.sys.mem_total, self.sys.mem_pct);
            for p in &self.sys.top_procs {
                r.push_str(&format!("• {} — {:.0} MB\n", p.name, p.mem_mb));
            }
            r
        } else if lower.contains("disco") || lower.contains("disk") || lower.contains("espaço") || lower.contains("space") {
            let clean = match self.sys.pkg_manager.as_str() {
                "apt" => "sudo apt autoremove --purge && sudo apt clean && sudo journalctl --vacuum-time=7d",
                "pacman" => "sudo pacman -Sc && sudo journalctl --vacuum-time=7d",
                "dnf" => "sudo dnf autoremove && sudo dnf clean all && sudo journalctl --vacuum-time=7d",
                _ => "sudo journalctl --vacuum-time=7d",
            };
            format!("Disco raiz: {}\n\nPara liberar espaço:\n`{clean}`", self.sys.disk_root)
        } else if lower.contains("gpu") || lower.contains("placa") || lower.contains("vídeo") {
            format!("GPU: {}", self.sys.gpu)
        } else if lower.contains("hardware") || lower.contains("info") || lower.contains("sistema") {
            format!(
                "Sistema: {}\nKernel: {}\nCPU: {} ({})\nGPU: {}\nMemória: {} / {} ({:.0}%)\nDisco: {}\nUptime: {}\nPacotes: {}",
                self.sys.os, self.sys.kernel, self.sys.cpu_model, self.sys.cpu_cores,
                self.sys.gpu, self.sys.mem_used, self.sys.mem_total, self.sys.mem_pct,
                self.sys.disk_root, self.sys.uptime, self.sys.packages
            )
        } else if lower.contains("processo") || lower.contains("process") {
            let mut r = String::from("Processos (RAM):\n\n");
            for p in &self.sys.top_procs {
                r.push_str(&format!("• {} — {:.0} MB (pid {})\n", p.name, p.mem_mb, p.pid));
            }
            r
        } else {
            format!(
                "Posso ajudar com atualizações, desinstalar, otimizar, hardware e processos.\n\nExemplos: \"otimizar\", \"desinstalar vlc\", \"liberar espaço\".\n\n{} · {}",
                self.sys.username, self.sys.hostname
            )
        };
        self.chat.push(("zero".into(), reply));
        self.trim_chat();
    }

    fn header_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();
        ui.set_min_height(44.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let gear_size = Vec2::splat(28.0);
            let (gear_rect, gear_resp) = ui.allocate_exact_size(gear_size, Sense::click());
            let gear_color = if gear_resp.hovered() { HIGHLIGHT_COLOR } else { TEXT_COLOR };
            draw_gear_icon(ui, gear_rect, gear_color);
            if gear_resp.clicked() { self.show_settings = !self.show_settings; }
            gear_resp.on_hover_text(s.settings);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                let upd_size = Vec2::splat(28.0);
                let (upd_rect, upd_resp) = ui.allocate_exact_size(upd_size, Sense::click());
                let upd_color = if self.update_available.is_some() || upd_resp.hovered() { HIGHLIGHT_COLOR } else { TEXT_COLOR };
                draw_update_icon(ui, upd_rect, upd_color);
                if upd_resp.clicked() {
                    if let Some(url) = self.update_available.clone() {
                        let _ = Command::new("xdg-open").arg(&url).spawn();
                    } else {
                        self.start_update_check();
                    }
                }
                if !self.update_status.is_empty() {
                    ui.label(egui::RichText::new(&self.update_status).size(12.0).color(MUTED_COLOR));
                }
            });
        });
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let s = self.lang.strings();
        ui.heading(s.settings);
        if ui.button(s.close).clicked() { self.show_settings = false; }
        ui.separator();
        ui.label(s.language);
        for &lang in Lang::ALL {
            if ui.selectable_label(self.lang == lang, lang.display_name()).clicked() {
                self.lang = lang;
                self.save_settings();
            }
        }
        ui.add_space(12.0);
        if ui.checkbox(&mut self.auto_update, s.auto_update).changed() {
            self.save_settings();
        }
        ui.add_space(12.0);
        if ui.button(s.check_updates).clicked() { self.start_update_check(); }
        if !self.update_status.is_empty() { ui.label(&self.update_status); }
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
                if ui.button(s.refresh).clicked() { self.refresh_sys(); }
            });
        });
        ui.separator();
        ui.collapsing(s.hardware, |ui| {
            ui.label(format!("CPU: {} ({})", self.sys.cpu_model, self.sys.cpu_cores));
            ui.label(format!("GPU: {}", self.sys.gpu));
            ui.label(format!("Memória: {} / {} ({:.0}%)", self.sys.mem_used, self.sys.mem_total, self.sys.mem_pct));
            ui.label(format!("Disco /: {}", self.sys.disk_root));
            ui.label(format!("Uptime: {}", self.sys.uptime));
        });
        ui.collapsing(s.software, |ui| {
            ui.label(format!("SO: {}", self.sys.os));
            ui.label(format!("Kernel: {}", self.sys.kernel));
            ui.label(format!("Host: {}", self.sys.hostname));
            ui.label(format!("Pacotes: {}", self.sys.packages));
        });
        if !self.sys.top_procs.is_empty() {
            ui.collapsing(s.top_procs, |ui| {
                for p in &self.sys.top_procs {
                    ui.label(format!("{} — {:.0} MB", p.name, p.mem_mb));
                }
            });
        }
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
                    let label = if role == "user" { "Você" } else { "Zero" };
                    ui.label(egui::RichText::new(label).strong().color(HIGHLIGHT_COLOR).size(13.0));
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
            if ui.add_enabled(!self.input.trim().is_empty(), egui::Button::new(s.send)).clicked() {
                send = true;
            }
        });
        if send {
            let msg = std::mem::take(&mut self.input);
            self.handle_user_message(&msg);
        }
    }

    fn draw_circle(&self, ui: &mut egui::Ui, height: f32, radius: f32) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
        let painter = ui.painter_at(rect);
        let t = self.start.elapsed().as_secs_f32();
        let pulse = 1.0 + 0.09 * (t * 1.55).sin();
        let center = rect.center() + Vec2::new(0.0, -10.0);
        let r = radius * pulse;
        for i in (1..=4).rev() {
            let expand = i as f32 * 9.0;
            let alpha = 16u8.saturating_sub(i as u8 * 3);
            painter.circle_filled(center, r + expand, Color32::from_rgba_unmultiplied(0xE9, 0x1E, 0x63, alpha));
        }
        painter.circle_filled(center, r, HIGHLIGHT_COLOR.gamma_multiply(0.9));
        painter.circle_stroke(center, r, egui::Stroke::new(2.4_f32, Color32::from_rgb(0xFF, 0x5C, 0x8A)));
        painter.circle_stroke(center, r * 0.7, egui::Stroke::new(1.4_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 80)));
        painter.circle_filled(center, 5.5, Color32::WHITE);
        let s = self.lang.strings();
        let greeting = s.greeting.replace("{}", &self.sys.username);
        painter.text(Pos2::new(center.x, center.y + r + 26.0), egui::Align2::CENTER_TOP, greeting, egui::FontId::proportional(19.0), TEXT_COLOR);
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

impl App for ZeroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if self.screenshot.is_some() {
            if self.frame_nr >= FRAMES_BEFORE_SHOT + 10 {
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
            .frame(egui::Frame::default().fill(BG_COLOR).inner_margin(egui::Margin::symmetric(4.0, 4.0)))
            .show(ctx, |ui| { self.header_ui(ui); });
        if self.show_settings {
            egui::SidePanel::left("settings")
                .default_width(280.0)
                .frame(egui::Frame::default().fill(Color32::from_rgb(0x0A, 0x0A, 0x0A)).inner_margin(12.0))
                .show(ctx, |ui| { self.settings_ui(ui); });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BG_COLOR).inner_margin(16.0))
            .show(ctx, |ui| {
                let wide = ui.available_width() > 900.0;
                if wide {
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            self.draw_circle(ui, 250.0, 68.0);
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
                    self.draw_circle(ui, 210.0, 56.0);
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
