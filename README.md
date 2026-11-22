# Zero

Assistente de sistema **Linux** leve e rápido (Rust + egui).

- Hardware e software (CPU, GPU, RAM, disco, kernel, pacotes)
- Sugestões de otimização
- Ajuda com atualizações e desinstalação
- Auto-update via releases do GitHub
- Fullscreen, círculo animado, saudação personalizada

[![Linux](https://github.com/xtaz7c/Zero/actions/workflows/linux.yml/badge.svg)](https://github.com/xtaz7c/Zero/actions/workflows/linux.yml)

## Build

```bash
cd app
cargo build --release
./target/release/zero
```

## Uso

- Abre em fullscreen
- ⚙️ Configurações (esquerda) · 🔄 Verificar atualizações (direita)
- Pergunte no chat: "otimizar", "desinstalar vlc", "liberar espaço", "mostrar hardware"

## CI (Linux)

Em cada push/PR para `main`, o workflow **Linux**:

1. Instala dependências de sistema (GTK, X11, Wayland, OpenGL)
2. `cargo fmt` / `clippy` (informativos)
3. `cargo build --release`
4. Valida o binário (ELF, executável, tamanho mínimo)
5. Smoke test com `xvfb` + `--screenshot`
6. Publica artefato `zero-linux-x86_64`

Rode manualmente em **Actions → Linux → Run workflow**.
