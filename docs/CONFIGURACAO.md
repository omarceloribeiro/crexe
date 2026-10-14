# Configuração local da engine

O arquivo TOML versão 1 seleciona um perfil em `providers`. `--provider` escolhe outro perfil local; `--model` substitui o modelo daquele perfil. `--config` escolhe explicitamente o arquivo. Não há seleção de endpoint, variável de credencial ou fallback pago pela receita.

Use `crexe configure`, o executável sem argumentos ou o lançador `configure.cmd` / `configure.command` / `configure.sh` do pacote. A tela nativa permite escolher o perfil, editar o modelo, testar conexão, consultar modelos, salvar a API key e importar o provider padrão de outro TOML. `configure --import caminho.toml` também carrega a importação para revisão; só **Salvar** aplica a mudança. Importar preserva as regras locais de execução, outros perfis e não copia referências de cofre de outra configuração. Trocar o perfil na tela descarta as edições ainda não salvas do perfil anterior.

Defaults e tipos estão em `config.example.toml`; campos desconhecidos são rejeitados. O exemplo não é a configuração ativa. A primeira instalação cria o arquivo ativo se ausente; reinstalações preservam o existente. A tela e `doctor` mostram o mesmo caminho. A escrita é atômica, com lock e detecção de edições externas: use **Recarregar** em caso de conflito. Fechar sem salvar não altera o arquivo; uma geração em andamento mantém as configurações com que começou.

A tela consulta `GET /api/tags` no Ollama ou `GET /models` em OpenAI/DeepSeek, apenas pelo botão **Testar conexão / atualizar modelos**. Não faz geração, tem timeout de 15 segundos e permite digitar um modelo mesmo sem listagem. Uma consulta bem-sucedida não garante que todo modelo da lista aceite o contrato de geração CREXE. Rede e cofre são acessados fora da thread gráfica.

Os presets vêm de `config.example.toml`; aparecem também ao abrir configurações antigas. Selecionar um preset ainda ausente só o grava ao salvar. A CLI `--provider deepseek` exige que esse perfil exista no TOML ativo, ou usa os defaults quando não há arquivo ativo. Atualizar/reinstalar não altera automaticamente o provider escolhido.

## Tentativas e orçamento de saída

**Tentativas máximas** na tela principal controla `execution.max_provider_requests`: de 1 a 10, padrão 3, incluindo a geração inicial. Só falhas de compilação/teste permitem correções automáticas; autenticação, saldo, conexão e respostas inválidas encerram a execução. `--max-repairs 0..9` é opcional: quando informado, reduz o número de correções dentro do limite configurado. `--max-repairs 0` permite apenas a proposta inicial.

Sem `execution.max_output_tokens_total`, o orçamento de saída é derivado de `max_provider_requests × providers.<perfil>.max_output_tokens`. Um teto já gravado em configurações antigas continua valendo. Em **Avançado**, **Limitar total de tokens de saída** permite editar esse teto ou voltar ao cálculo automático. Por exemplo, 10 tentativas com 6000 tokens por chamada usam orçamento de 60000; um teto explícito de 18000 reduz o limite efetivo a 3. Se o teto não comportar uma chamada, a execução informa o problema antes de consultar o provider.

A janela de geração mostra provider, modelo, tentativa atual e total efetivo, com a fase em campo separado. Esses valores são capturados no início, incluindo substituições da CLI. Alterar tentativas ou teto de saída não invalida um programa aprovado, e não modifica uma execução já iniciada. O salvamento preserva as demais regras de execução, comentários e perfis locais.

## Chaves no cofre do usuário

**Salvar** guarda a chave digitada no Windows Credential Manager (persistência local), macOS Keychain ou Secret Service no Linux. O TOML recebe somente `credential.id` e `credential.binding`, vinculados ao caminho da configuração, perfil e endpoint. Trocar o endpoint exige nova chave ou remoção explícita. A tela não preenche o campo com a chave antiga; mostra o estado da referência. Cofre bloqueado/ausente gera erro, sem salvar em texto puro ou usar outra chave silenciosamente.

Uma troca grava outra entrada, publica a configuração e só então remove a entrada antiga pertencente ao CREXE. Se salvar falhar, mantém a configuração anterior e tenta remover a entrada temporária. Copiar um TOML com referência para outra máquina ou caminho exige configurar a chave novamente. Trocar apenas a chave/referência não altera a identidade do cache. A geração resolve a chave na primeira chamada e a mantém em memória para seus reparos; uma troca/remoção posterior no cofre não interrompe essas chamadas. Revogar a chave no serviço remoto ainda pode impedir seu uso.

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

DeepSeek usa [Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/) em `https://api.deepseek.com`, `max_tokens`, JSON mode e resposta completa (`stream: false`). O preset `deepseek-flash` tem timeout de 300 segundos e saída máxima de 6000 tokens. `thinking = false` (ou ausente) envia raciocínio desativado explicitamente; `true` habilita o modo e omite temperatura. A tela oferece esse controle em **Avançado**. Raciocínio pode aumentar latência e consumo; não é necessário para o contrato CREXE. A API key é salva no cofre ou resolvida por `api_key_env = "crexe_deepseek_api_key"`.

Respostas DeepSeek vazias, truncadas, recusadas ou contendo chamadas de tools não são aceitas. Apenas o conteúdo final vira arquivos; o raciocínio não entra em fontes, logs ou ZIP. Logs de respostas JSON válidas podem incluir o modelo retornado e contagens numéricas de tokens fornecidas pela API. HTTP 401 indica chave inválida; 402 saldo insuficiente; 400/422 parâmetros inválidos; 429 limite de requisições; 500/503 indisponibilidade. Essas falhas não disparam repetição automática. Reparos de compilação/teste continuam sujeitos aos limites locais.

`crexe_deepseek_api_key` e `DEEPSEEK_API_KEY` são removidas do ambiente dos processos filhos mesmo em configurações antigas ou com outro provider ativo, além dos nomes personalizados configurados. Para DeepSeek, trocar a referência/variável da chave mantém a identidade do cache; `thinking` ausente equivale a `false`. Modelo, endpoint e modo efetivo de raciocínio distinguem gerações. As identidades legadas Ollama/OpenAI permanecem preservadas.

HTTPS é obrigatório fora do loopback. URLs com credenciais, query ou fragment são recusadas. Redirecionamentos HTTP são desabilitados. Corpos de erro HTTP não são reproduzidos nos logs. Nenhum adaptador compra créditos nem seleciona outro provider ao falhar.

## Limites aplicados

| Recurso | Implementação atual |
|---|---|
| Documento | Até 1 MiB |
| Chamada HTTP | `timeout_seconds`, 1–1800 s; conexão até 15 s |
| Saída do modelo | `max_output_tokens`, 128–65536; resposta HTTP até 20 MiB |
| Ollama | Contexto 512–131072, maior que a saída e com espaço também para o prompt |
| Propostas | De 1 a 10 tentativas totais (padrão 3); `--max-repairs 0..9` opcional restringe correções |
| Orçamento local | Até `execution.max_provider_requests` chamadas; saída derivada de tentativas × tokens por chamada, ou `max_output_tokens_total` explícito; prompt completo até `max_prompt_bytes` bytes (padrão 131072) |
| Prazo de geração | `execution.generation_timeout_seconds` (padrão 1800 s) desde o início da preparação; reduz o timeout das próximas chamadas ao tempo restante |
| Fontes | 1–128 arquivos; 4 MiB por arquivo; 16 MiB por resposta |
| Build/teste | Timeout por comando; 2 MiB por stream de log; cauda de até 64 KiB por stream nos diagnósticos |
| Revisão | Até 4096 arquivos / 2 GiB |
| App final | Sem timeout padrão; timeout de run do YAML é aplicado se explícito |

Fechar a janela ou Ctrl+C encerra a orquestração e os grupos de subprocessos próprios. Uma requisição já entregue ao servidor pode continuar brevemente no provider; a engine não publica seu resultado após cancelamento. O serviço Ollama não é parado.

A reserva de saída usa o máximo solicitado por chamada, mesmo se o modelo responder menos. Não mede tokens de entrada nem é um teto monetário da API. O prazo de geração impede novas chamadas após o limite; build/teste têm prazos próprios por comando (padrões 300/60 s), que a receita só pode reduzir. Não é um prazo único que interrompe o aplicativo final.

O contexto incremental é explícito: pedido anterior, fontes aprovados e pedido atual, conforme o [gerenciamento de estado por mensagens](https://developers.openai.com/api/docs/guides/conversation-state). Não depende de arquivos ou conversas mantidos pelo provider. O limite de prompt inclui os contratos dos perfis e esse contexto; excedê-lo interrompe a geração e mantém o baseline. Os reparos enviam o snapshot da tentativa mais recente, sem repetir o snapshot aprovado original.

## Política local de execução

`execution.allowed_tools` autoriza executáveis do PATH ou caminhos absolutos definidos pelo usuário. A comparação usa o caminho resolvido da ferramenta, não apenas seu nome. A allowlist do YAML pode restringir essa lista, mas não ampliá-la. Os artefatos construídos dentro do workspace podem ser executados como app/autoteste.

Shells e scripts de shell exigem `allow_shells = true` e inclusão explícita da ferramenta em `allowed_tools`. Receitas legadas que usavam `cmd`, `sh` ou PowerShell precisam dessa configuração local; não altere a política para todo documento recebido. Os scripts do ZIP são uma alternativa externa à CLI e executam diretamente com as permissões do usuário.

Esse controle se aplica aos comandos iniciados pela engine. Compiladores, projetos, scripts e aplicativos continuam executando no host; a política não restringe seus próprios subprocessos ou acesso a arquivos/rede. Ainda não há retenção automática de revisões/diagnósticos. O autoteste gerado pelo modelo precisa de validação independente para servir como evidência funcional forte.

No YAML, `allowNetwork` exige um booleano real; `false` continua sendo recusado por exigir isolamento indisponível. Políticas desconhecidas são rejeitadas. Timeouts exigem segundos inteiros positivos; `generate` pode reduzir o timeout por chamada do provider. Uma allowlist vazia para o OS bloqueia ferramentas, em vez de liberar qualquer comando. Estruturas inválidas de build/test/publish e templates pendentes nessas operações são recusados antes da geração.
