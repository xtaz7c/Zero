# Brief

**Question:** O que devo construir para o app Zero: quais funcionalidades, interface e comportamento para atender à solicitação: app desktop com eframe/egui chamado Zero com interface simples, configurações (idioma, adição de modelos, atualização automática via IA), chat principal onde todas as IAs podem conversar para trabalhar juntas, e aplicar uma logo simples criada com a Craft.

**Purpose:** own_or_authorized

**Targets:**
- /home/hamber/Documentos/Craft Anything/crie-um-app-chamado-zero/app

**Constraints:**
- Rust com eframe/egui; Cargo.toml no app/; cargo build até passar; mudanças só dentro de app/; logo via Craft (experimental_image_generate) salva em app/assets/logo.png; rodar com --screenshot para verificar; usar core_person_ask só se uma escolha mudar o resultado.

**Done when:** App Zero compila (cargo build em app/), tem: tela inicial com logo, configurações (idioma, adição de modelos, atualização automática por IA), chat principal onde os modelos adicionados conversam com o usuário; foto salva quando rodado com --screenshot; logo em app/assets/logo.png.
