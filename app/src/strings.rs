#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    PtBr,
    En,
}

#[derive(Clone, Copy)]
pub struct Strings {
    pub settings: &'static str,
    pub close: &'static str,
    pub language: &'static str,
    pub auto_update: &'static str,
    pub check_updates: &'static str,
    pub update_checking: &'static str,
    pub update_current: &'static str,
    pub update_new: &'static str,
    pub update_fail: &'static str,
    pub update_open: &'static str,
    pub greeting: &'static str,
    pub system_info: &'static str,
    pub hardware: &'static str,
    pub software: &'static str,
    pub suggestions: &'static str,
    pub top_procs: &'static str,
    pub input_placeholder: &'static str,
    pub send: &'static str,
    pub refresh: &'static str,
    pub no_suggestions: &'static str,
}

impl Lang {
    pub const ALL: &'static [Lang] = &[Lang::PtBr, Lang::En];

    pub fn code(self) -> &'static str {
        match self {
            Lang::PtBr => "pt_BR",
            Lang::En => "en",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Lang::PtBr => "Português (Brasil)",
            Lang::En => "English",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.iter().copied().find(|l| l.code() == code)
    }

    pub fn strings(self) -> Strings {
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
                top_procs: "Processos (RAM)",
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
                top_procs: "Processes (RAM)",
                input_placeholder: "Ask about updates, uninstall, optimize…",
                send: "Send",
                refresh: "Refresh data",
                no_suggestions: "No critical suggestions right now. Your system looks healthy.",
            },
        }
    }
}
