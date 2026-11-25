# Alterações

## 1.0.0 — em desenvolvimento, não publicada

- Prévia CREXP: ZIP de especificações Markdown, consolidação determinística, detecção por conteúdo e histórico incremental preservado ao editar o pacote.
- Prévia CREXB: exportação da última revisão aceita com receita original, fontes, autotestes e payload; abertura compatível sem IA ou compilação, integridade e dados separados do cache.
- Associações Windows para `.crexp` e `.crexb`; guias, roadmap e downloads dos handoffs originais no site em português e inglês, com exemplo CREXP verificável.
- Editor integrado `crexe edit`, menu Editar Windows e diálogo para arquivos vazios; gravação atômica com preservação de UTF-8/BOM/LF/CRLF e detecção de conflitos.
- Geração incremental com fontes/pedido anteriores, histórico por documento, recuperação explícita por `--fresh` e adoção verificável do cache legado.
- Tentativas totais configuráveis de 1 a 10, orçamento de saída derivado ou explícito e acompanhamento de provider/modelo/tentativa.
- Seleção de perfil na resposta de geração e perfis OpenGL por host; OpenTK 4.9.4 fixado e lockfile exportável no Windows.

- Pacote Cargo na raiz, baseline com referência fixa e implementação histórica arquivada.
- Instalação por usuário, associação Windows e janela de espera separada por plataforma.
- YAML legado, Markdown com marcador e prompt livre; perfil C#/.NET no host.
- Configuração local; Ollama, OpenAI e DeepSeek sem fallback pago automático.
- DeepSeek na tela de configuração, chave no cofre/ambiente, catálogo de modelos e raciocínio opcional.
- Workspace temporário, timeout/cancelamento e remoção de credenciais do ambiente dos filhos.
- Cache transacional com hashes e revisões imutáveis.
- Vários fontes, build, correções limitadas, autoteste do perfil e ZIP com scripts.
- Manifesto de projeto e comandos build/test/run/publish/export sem chamadas ao modelo.
- Política local de ferramentas, shells opt-in e orçamento de chamadas/saída/tempo.
- Perfis experimentais C++/Win32, Objective-C++/Cocoa e C/GTK3, com SDKs do host e ensaio nativo separado.
- Testes offline, verificação nativa Windows e CI aprovada em Windows/Linux/macOS.

`docs/IMPLEMENTACAO_V1.md` distingue resultados medidos e pendências. O número do pacote foi preservado do baseline; não indica release estável já distribuída.
