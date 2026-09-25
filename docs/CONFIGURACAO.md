# Configuração local da engine

O arquivo TOML versão 1 seleciona um perfil em `providers`. `--provider` escolhe outro perfil local; `--model` substitui o modelo daquele perfil. `--config` escolhe explicitamente o arquivo. Não há seleção de endpoint, variável de credencial ou fallback pago pela receita.

Defaults e tipos estão em `config.example.toml`; campos desconhecidos são rejeitados. Segredos literais não fazem parte do schema. Precedência da chave: variável do processo, arquivo `--env-file` explícito, variável do usuário Windows. O carregamento de `.env` é restrito à memória da engine, sem exportar seu conteúdo ao processo ou aos filhos. O nome referenciado precisa corresponder ao nome à esquerda de `=`.

| Plataforma | Arquivo padrão |
|---|---|
| Windows | `%APPDATA%\CREXE\config.toml` |
| Linux | `${XDG_CONFIG_HOME:-$HOME/.config}/crexe/config.toml` |
| macOS | `~/Library/Application Support/CREXE/config.toml` |
| `CREXE_HOME` explícito | `<CREXE_HOME>/config.toml` |

## Adaptadores

Ollama usa [`POST /api/chat`](https://docs.ollama.com/api/chat), schema JSON de `files`, `stream: false`, `think`, `keep_alive`, `num_ctx` e `num_predict`. O adaptador recebe uma resposta completa; streaming e geração por arquivo são evoluções possíveis, sem promessa de corrigir sintaxe por si sós. `done: true` e `done_reason: stop` são exigidos.

OpenAI usa [Chat Completions](https://developers.openai.com/api/reference/cli/resources/chat/subresources/completions/methods/create), JSON mode e Bearer quando há variável configurada. Usa `max_completion_tokens` para famílias de raciocínio reconhecidas e `max_tokens` nas demais. Recusa e término diferente de `stop` não viram arquivos. O modelo de exemplo é `gpt-4o-mini`; alterar o modelo é uma escolha local.

HTTPS é obrigatório fora do loopback. URLs com credenciais, query ou fragment são recusadas. Redirecionamentos HTTP são desabilitados. Corpos de erro HTTP não são reproduzidos nos logs. Nenhum adaptador compra créditos nem seleciona outro provider ao falhar.

## Limites aplicados

| Recurso | Implementação atual |
|---|---|
| Documento | Até 1 MiB |
| Chamada HTTP | `timeout_seconds`, 1–1800 s; conexão até 15 s |
| Saída do modelo | `max_output_tokens`, 128–65536; resposta HTTP até 20 MiB |
| Ollama | Contexto 512–131072, maior que a saída e com espaço também para o prompt |
| Propostas | Inicial + até duas correções de build/teste (`--max-repairs 0..2`) |
| Orçamento local | Até `execution.max_provider_requests` chamadas (padrão 3); reserva de até `max_output_tokens_total` tokens de saída (padrão 18000); prompt até `max_prompt_bytes` bytes (padrão 131072) |
| Prazo de geração | `execution.generation_timeout_seconds` (padrão 1800 s) desde o início da preparação; reduz o timeout das próximas chamadas ao tempo restante |
| Fontes | 1–128 arquivos; 4 MiB por arquivo; 16 MiB por resposta |
| Build/teste | Timeout por comando; 2 MiB por stream de log; cauda de até 64 KiB por stream nos diagnósticos |
| Revisão | Até 4096 arquivos / 2 GiB |
| App final | Sem timeout padrão; timeout de run do YAML é aplicado se explícito |

Fechar a janela ou Ctrl+C encerra a orquestração e os grupos de subprocessos próprios. Uma requisição já entregue ao servidor pode continuar brevemente no provider; a engine não publica seu resultado após cancelamento. O serviço Ollama não é parado.

A reserva de saída usa o máximo solicitado por chamada, mesmo se o modelo responder menos. Não mede tokens de entrada nem é um teto monetário da API. O prazo de geração impede novas chamadas após o limite; build/teste têm prazos próprios por comando (padrões 300/60 s), que a receita só pode reduzir. Não é um prazo único que interrompe o aplicativo final.

## Política local de execução

`execution.allowed_tools` autoriza executáveis do PATH ou caminhos absolutos definidos pelo usuário. A comparação usa o caminho resolvido da ferramenta, não apenas seu nome. A allowlist do YAML pode restringir essa lista, mas não ampliá-la. Os artefatos construídos dentro do workspace podem ser executados como app/autoteste.

Shells e scripts de shell exigem `allow_shells = true` e inclusão explícita da ferramenta em `allowed_tools`. Receitas legadas que usavam `cmd`, `sh` ou PowerShell precisam dessa configuração local; não altere a política para todo documento recebido. Os scripts do ZIP são uma alternativa externa à CLI e executam diretamente com as permissões do usuário.

Esse controle se aplica aos comandos iniciados pela engine. Compiladores, projetos, scripts e aplicativos continuam executando no host; a política não restringe seus próprios subprocessos ou acesso a arquivos/rede. Ainda não há retenção automática de revisões/diagnósticos. O autoteste gerado pelo modelo precisa de validação independente para servir como evidência funcional forte.

No YAML, `allowNetwork` exige um booleano real; `false` continua sendo recusado por exigir isolamento indisponível. Políticas desconhecidas são rejeitadas. Timeouts exigem segundos inteiros positivos; `generate` pode reduzir o timeout por chamada do provider. Uma allowlist vazia para o OS bloqueia ferramentas, em vez de liberar qualquer comando. Estruturas inválidas de build/test/publish e templates pendentes nessas operações são recusados antes da geração.
