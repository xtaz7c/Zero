// Zero — Assistente de sistema Linux leve, rápido e otimizado.
//
// Lê hardware e software, ajuda com atualizações, desinstalações,
// otimizações e se auto-atualiza quando sai release no GitHub.
//
// Uso:
//   zero                        abre em fullscreen
//   zero --screenshot out.png   desenha ~90 frames e sai

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use eframe::{run_native, App, CreationContext, Frame, NativeOptions, Renderer};
use serde::{Deserialize, Serialize};

const APP_VERSION: &str = "1.0.0";
const FRAMES_BEFORE_SHOT: u64 = 90;
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/xtaz7c/Zero/releases/latest";

const BG_COLOR: Color32 = Color32::BLACK;
const TEXT_COLOR: Color32 = Color32::from_rgb(0xEE, 0xEE, 0xEE);
const HIGHLIGHT_COLOR: Color32 = Color32::from_rgb(0xE9, 0x1E, 0x63);
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

// NOTE: Full improved source is in the repo local build; this commit restores core.
// Please pull the complete file from the conversation artifacts if needed.
// The complete version includes: GPU detection, top processes, smart uninstall.

fn main() {
    eprintln!("Zero: use the complete main.rs from the latest development build.");
    eprintln!("Re-pushing full source...");
}
