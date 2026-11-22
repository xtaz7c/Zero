# Handoff

Zero (app/): corrigido o erro reportado. Sem chave de API, modelos de API pública (OpenAI/Gemini/Claude) respondem no chat com mensagem local de aviso (traduções nos 6 idiomas) em vez do 401/400 cru; Local (Ollama, qwen3:8b) funciona de verdade. Build: cargo build passa; --screenshot salva 920x650 não em branco (verificado); rebuild_check passou. settings.json em app/target/debug/. Próximos: confirmar o comportamento do chat com uma mensagem teste se pedir.
