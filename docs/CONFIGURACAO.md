# Configuração local da engine

O arquivo TOML versão 1 seleciona um perfil em `providers`. `--provider` escolhe outro perfil local; `--model` substitui o modelo daquele perfil. `--config` escolhe explicitamente o arquivo. Não há seleção de endpoint, variável de credencial ou fallback pago pela receita.

Use `crexe configure`, o executável sem argumentos ou o lançador `configure.cmd` / `configure.command` / `configure.sh` do pacote. A tela nativa permite escolher o perfil, editar o modelo, testar conexão, consultar modelos, salvar a API key e importar o provider padrão de outro TOML. `configure --import caminho.toml` também carrega a importação para revisão; só **Salvar** aplica a mudança. Importar preserva as regras locais de execução, outros perfis e não copia referências de cofre de outra configuração. Trocar o perfil na tela descarta as edições ainda não salvas do perfil anterior.

Defaults e tipos estão em `config.example.toml`; campos desconhecidos são rejeitados. O exemplo não é a configuração ativa. A primeira instalação cria o arquivo ativo se ausente; reinstalações preservam o existente. A tela e `doctor` mostram o mesmo caminho. A escrita é atômica, com lock e detecção de edições externas: use **Recarregar** em caso de conflito. Fechar sem salvar não altera o arquivo; uma geração em andamento mantém as configurações com que começou.

A tela consulta `GET /api/tags` no Ollama ou `GET /models` no endpoint OpenAI compatível, apenas pelo botão **Testar conexão / atualizar modelos**. Não faz geração, tem timeout de 15 segundos e permite digitar um modelo mesmo sem listagem. Uma consulta bem-sucedida não garante que todo modelo da lista aceite o contrato de geração CREXE. Rede e cofre são acessados fora da thread gráfica.

## Chaves no cofre do usuário

**Salvar** guarda a chave digitada no Windows Credential Manager (persistência local), macOS Keychain ou Secret Service no Linux. O TOML recebe somente `credential.id` e `credential.binding`, vinculados ao caminho da configuração, perfil e endpoint. Trocar o endpoint exige nova chave ou remoção explícita. A tela não preenche o campo com a chave antiga; mostra o estado da referência. Cofre bloqueado/ausente gera erro, sem salvar em texto puro ou usar outra chave silenciosamente.

Uma troca grava outra entrada, publica a configuração e só então remove a entrada antiga pertencente ao CREXE. Se salvar falhar, mantém a configuração anterior e tenta remover a entrada temporária. Copiar um TOML com referência para outra máquina ou caminho exige configurar a chave novamente. Trocar apenas a chave/referência não altera a identidade do cache.

Precedência quando há referência de cofre: variável correspondente em `--env-file` explícito, depois cofre. Variáveis antigas do processo/Windows não substituem a chave salva. Sem referência, mantém a compatibilidade anterior: variável do processo, arquivo `--env-file` explícito, variável do usuário Windows. Remover uma chave salva reativa essa resolução legada, quando `api_key_env` estiver configurado.

Segredos literais não fazem parte do schema TOML. O carregamento de `.env` é restrito à memória da engine, sem exportar seu conteúdo ao processo ou aos filhos. O nome referenciado precisa corresponder ao nome à esquerda de `=`. CLI/doctor/inspect não requerem display; hosts sem desktop/cofre podem continuar usando ambiente ou `--env-file`. O Linux requer uma sessão Secret Service desbloqueada para salvar chaves pela tela.

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
