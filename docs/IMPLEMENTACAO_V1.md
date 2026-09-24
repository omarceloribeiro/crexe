# Implementação da v1

Iniciada em 24/09/2026, na branch `feature/v1`. Referência fixa: `baseline/pre-v1`. Este registro não declara a v1 concluída ou pronta para distribuição pública.

## Implementado no primeiro conjunto de incrementos

- Pacote Cargo/lockfile na raiz, CLI fina e módulos internos. Scripts ativos em `scripts`, exemplos separados por formato, fixture original preservada e tentativa GPT Pro arquivada sem virar baseline.
- Instalação estável por usuário em `releases/v1`; associação Windows preserva registros de outros aplicativos. Módulos Windows usam `cfg`, sem remoção manual de fontes nas outras plataformas.
- Janela de espera nativa Windows com fases, cancelamento e mensagem de falha. Terminal mantém logs quando `--ui` não é solicitado.
- YAML legado validado, Markdown com front matter e plaintext livre. Nome padrão derivado do arquivo; `inspect` e `doctor` sem geração. Perfil automático Windows .NET 8/WinForms e opção explícita .NET console.
- Configuração local TOML, adaptadores Ollama/OpenAI, `.env` apenas explícito, chave consultada sem log e sem fallback pago automático. Receitas não escolhem o destino da credencial.
- Workspace temporário com uma pasta nova por tentativa, validação prévia de arquivos e caminhos, execução com timeout/cancelamento e remoção das referências conhecidas de credenciais dos filhos.
- Cache transacional, revisões imutáveis, hashes, trava por entrada, rejeição de artefatos alterados e preservação da revisão anterior na falha.
- Geração com vários fontes, build, autoteste de perfil, até duas correções e ZIP com fontes/scripts/instruções. Exportação funciona também em cache hit. O perfil .NET inclui scripts de publish dependente do runtime.

A licença do pacote referencia `LICENSE`, sem presumir `-or-later`. Não se atribui automaticamente a licença da engine aos aplicativos gerados.

## Segundo incremento: operações e limites locais

- `CREXE-PROJECT.json` registra os fontes e comandos do projeto. CLI e scripts do ZIP usam esse mesmo contrato. `build`, `test`, `run`, `publish` e `export` não chamam o provider; build/publish produzem outra pasta, preservando a origem.
- Política TOML de ferramentas por caminho resolvido, shells mediante configuração explícita e limites locais por comando. A receita não concede permissões acima da configuração local. Isso não isola código executado no host.
- Limites de quantidade de chamadas, reserva de tokens de saída, tamanho do prompt e prazo para chamadas durante geração/reparo. Não constituem teto monetário nem contagem de tokens de entrada.
- Suíte ampliada para 28 testes (21 unitários, 7 de integração). Recompilar/testar/publicar/exportar com o servidor controlado desligado passou; ferramentas negadas e orçamento esgotado são recusados sem chamadas extras.
- CI do checkpoint `3f6208e` aprovada em Windows, Ubuntu e macOS: [execução 36063638828](https://github.com/omarceloribeiro/crexe/actions/runs/36063638828). O macOS exigiu reconhecer o destino relativo dos aliases de sistema `/var` e `/tmp`; a exceção continua restrita a esses aliases. Isso comprova o build da engine e a suíte, não os perfis GUI nativos.

## Validações realizadas neste host

O ambiente de desenvolvimento usa Rust 1.98.1 em `%LOCALAPPDATA%/CREXE/devtools`, sem alteração permanente do PATH, e ferramentas C++ já instaladas do Visual Studio. O SDK .NET 8 está instalado. Preparar o build da engine não torna Rust obrigatório para seus usuários.

- Suíte Rust offline com compilação real e providers locais controlados: cache, atualização de intenção, artefato ausente, rebuild com falha, timeout, correção de erro de compilador e ZIP recompilável. Casos de paths, schema, endpoints e credenciais também cobertos. Checkpoint: 25 testes aprovados (20 unitários e 5 de integração), fmt/clippy sem erros e build release nativo aprovado.
- Janela nativa real capturada e inspecionada. Fechamento após build e cancelamento preservando cache verificados por `tests/windows_progress.py`. A captura usa pixels físicos para não cortar a imagem em displays com escala. O ensaio de associação detectou e confirmou a correção de identificadores de terminal inválidos após FreeConsole; o app filho recebe handles válidos no modo desktop.
- Associação testada via Windows ShellExecute, com compilação Rust e marcadores WHITE/WHITE/GREEN/GREEN. Isso não é clique manual no Explorer nem teste da calculadora GUI.
- **Ollama real:** Gemma 12B gerou dois fontes de console, compilou com zero erros/avisos, passou no autoteste e imprimiu `42`; ZIP criado. A calculadora local primeiro excedeu 300 s; com 900 s entregou três fontes. O reparo corrigiu a sintaxe inicial, mas restou `MessageBoxButtons.OK1` após a única correção permitida naquele ensaio. Não foi publicado cache dessa falha. [Relatório](validation/2026-09-24-ollama-engine.json).
- **OpenAI como último recurso autorizado:** uma chamada `gpt-4o-mini` gerou três fontes da calculadora. Build sem erros, com 14 avisos de nulidade; autoteste aprovado. A engine abriu a GUI verde e operações `2+3`, `8-3`, `2*3`, `8/2` foram verificadas pelos controles reais. Cache hit sem geração confirmado. ZIP extraído em outra pasta recompilou e executou test/publish. Nenhuma compra de créditos. [Relatório](validation/2026-09-24-calculator-openai.json).

Os autotestes são produzidos pelo modelo, portanto não substituem revisão ou testes independentes. O teste de ZIP foi neste mesmo host, não em uma máquina limpa. O perfil C++/MinGW original não foi reproduzido aqui. Não houve envio de emails ou teste de dispositivos físicos.

## Critérios que continuam abertos

1. Robustez e qualidade da geração local de apps desktop, contratos de testes independentes, contabilização de consumo efetivo e opção de correção granular; streaming não é tratado como solução comprovada de sintaxe.
2. Perfis automáticos C++/Win32, Cocoa/macOS e GTK/Linux, seleção por inventário mais completo, triagem neutra de requisitos e diagnóstico de dependências/arquitetura física do host. Hoje `ARCH` informa a arquitetura da engine.
3. Validação de campos restantes do YAML, dependências além da biblioteca padrão/.NET, retenção/limpeza de cache e relatórios completos de execução.
4. Expansão do contrato de projetos para outros perfis/dependências e distribuição, sem prometer portabilidade implícita de binários.
5. Matriz de apps/arquiteturas, instalação a partir de sessão normal do usuário, clique manual no Explorer, máquina limpa e recuperação após interrupção abrupta. Há virtualização de caminhos AppData no ambiente do Codex; testes feitos por ele não substituem a verificação fora do app.
6. Reconciliação normativa de todas as RFCs, matriz pública de suporte, release com checksums e conteúdo final para `crexe.org`. Sem publicação do site, alteração do domínio ou integração em `main` nesta etapa.

O .env informado pelo autor não estava visível no checkout durante a consulta; a chamada autorizada usou a variável `crexe_openai_api_key_env` já existente no ambiente de usuário Windows. Seu valor não foi registrado em arquivos de configuração, documentação ou logs da engine.
