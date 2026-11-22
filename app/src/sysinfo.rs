use std::process::Command;

#[derive(Clone, Default)]
pub struct ProcInfo {
    pub mem_mb: f32,
    pub pid: u32,
    pub name: String,
}

#[derive(Clone, Default)]
pub struct SysInfo {
    pub username: String,
    pub hostname: String,
    pub kernel: String,
    pub os: String,
    pub cpu_model: String,
    pub cpu_cores: String,
    pub gpu: String,
    pub mem_total: String,
    pub mem_used: String,
    pub mem_pct: f32,
    pub disk_root: String,
    pub disk_pct: f32,
    pub uptime: String,
    pub packages: String,
    pub pkg_manager: String,
    pub top_procs: Vec<ProcInfo>,
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

fn detect_gpu() -> String {
    if let Some(out) = run_cmd("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]) {
        let name = out.lines().next().unwrap_or("").trim();
        if !name.is_empty() {
            return format!("NVIDIA {name}");
        }
    }
    if let Some(out) = run_cmd("lspci", &[]) {
        for line in out.lines() {
            let lower = line.to_lowercase();
            if lower.contains("vga") || lower.contains("3d") || lower.contains("display") {
                if let Some(rest) = line.split(": ").nth(1) {
                    return rest.trim().to_string();
                }
                return line.trim().to_string();
            }
        }
    }
    "GPU não detectada".into()
}

fn top_memory_procs(limit: usize) -> Vec<ProcInfo> {
    let out = match run_cmd("ps", &["-eo", "pid,rss,comm", "--sort=-rss"]) {
        Some(o) => o,
        None => return Vec::new(),
    };
    let mut list = Vec::with_capacity(limit);
    for line in out.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let pid: u32 = parts[0].parse().unwrap_or(0);
        let rss_kb: f32 = parts[1].parse().unwrap_or(0.0);
        let name = parts[2..].join(" ");
        if name.starts_with('[') || name.is_empty() || pid == 0 {
            continue;
        }
        list.push(ProcInfo {
            mem_mb: rss_kb / 1024.0,
            pid,
            name,
        });
        if list.len() >= limit {
            break;
        }
    }
    list
}

pub fn gather_sysinfo() -> SysInfo {
    let mut info = SysInfo {
        username: std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "usuário".into()),
        hostname: read_file("/etc/hostname")
            .map(|s| s.trim().to_string())
            .or_else(|| run_cmd("hostname", &[]))
            .unwrap_or_else(|| "localhost".into()),
        kernel: read_file("/proc/version")
            .and_then(|v| v.split_whitespace().nth(2).map(|s| s.to_string()))
            .or_else(|| run_cmd("uname", &["-r"]))
            .unwrap_or_else(|| "?".into()),
        ..SysInfo::default()
    };

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
            if (line.starts_with("model name") || line.starts_with("Hardware")) && model.is_none() {
                model = line.split(':').nth(1).map(|s| s.trim().to_string());
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

    info.gpu = detect_gpu();

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
                info.disk_pct = parts[3].trim_end_matches('%').parse().unwrap_or(0.0);
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
        info.pkg_manager = "apt".into();
        if let Some(n) = run_cmd("dpkg", &["--get-selections"]) {
            let count = n.lines().filter(|l| l.contains("install")).count();
            info.packages = format!("apt · {count} pacotes");
        } else {
            info.packages = "apt".into();
        }
    } else if run_cmd("which", &["pacman"]).is_some() {
        info.pkg_manager = "pacman".into();
        if let Some(n) = run_cmd("pacman", &["-Q"]) {
            let count = n.lines().count();
            info.packages = format!("pacman · {count} pacotes");
        } else {
            info.packages = "pacman".into();
        }
    } else if run_cmd("which", &["dnf"]).is_some() || run_cmd("which", &["rpm"]).is_some() {
        info.pkg_manager = "dnf".into();
        if let Some(n) = run_cmd("rpm", &["-qa"]) {
            let count = n.lines().count();
            info.packages = format!("rpm · {count} pacotes");
        } else {
            info.packages = "rpm/dnf".into();
        }
    } else {
        info.packages = "desconhecido".into();
    }

    info.top_procs = top_memory_procs(6);
    info
}

pub fn generate_suggestions(info: &SysInfo) -> Vec<String> {
    let mut s = Vec::new();

    if info.mem_pct > 85.0 {
        s.push(format!(
            "Memória em uso alto ({:.0}%). Feche apps pesados ou aumente o swap.",
            info.mem_pct
        ));
    } else if info.mem_pct > 70.0 {
        s.push(format!(
            "Memória em {:.0}%. Monitore com `htop` ou `btop`.",
            info.mem_pct
        ));
    }

    if info.disk_pct >= 90.0 {
        s.push(format!(
            "Disco raiz quase cheio ({:.0}%). Rode limpeza de pacotes e journal.",
            info.disk_pct
        ));
    } else if info.disk_pct >= 80.0 {
        s.push(format!(
            "Disco em {:.0}%. Considere limpeza de caches e órfãos.",
            info.disk_pct
        ));
    }

    match info.pkg_manager.as_str() {
        "apt" => {
            s.push("Atualize: `sudo apt update && sudo apt upgrade -y`".into());
            s.push("Limpe órfãos: `sudo apt autoremove --purge`".into());
        }
        "pacman" => {
            s.push("Atualize: `sudo pacman -Syu`".into());
            s.push("Limpe cache: `sudo pacman -Sc`".into());
        }
        "dnf" => {
            s.push("Atualize: `sudo dnf upgrade`".into());
            s.push("Limpe: `sudo dnf autoremove`".into());
        }
        _ => s.push("Mantenha o sistema atualizado pelo gerenciador da sua distro.".into()),
    }

    s.push("Revise serviços na boot: `systemctl list-unit-files --state=enabled`".into());
    s
}

pub fn uninstall_command(pkg_manager: &str, package: &str) -> String {
    let p = package.trim();
    match pkg_manager {
        "apt" => format!("sudo apt remove {p} && sudo apt autoremove"),
        "pacman" => format!("sudo pacman -Rns {p}"),
        "dnf" => format!("sudo dnf remove {p}"),
        _ => format!("# tente remover o pacote `{p}` manualmente"),
    }
}
