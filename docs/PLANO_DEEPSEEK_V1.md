# Plano: provider DeepSeek no CREXE v1

Data do plano: 25/09/2026. Base: `b81c641`, branch `feature/v1`. Atualização de 26/09/2026: **implementado**, com testes reais autorizados sobre os US$ 5 informados pelo autor. O texto abaixo preserva as decisões do plano; resultados e limitações estão em [IMPLEMENTACAO_V1.md](IMPLEMENTACAO_V1.md) e no relatório de validação DeepSeek.

O objetivo é escolher DeepSeek na tela de configuração ou por `--provider deepseek`, usando o mesmo fluxo de projeto completo, build, correção, cache, ZIP e abertura já implementado. A integração continua usando o workspace temporário no host, independente do fornecedor da IA.

## 1. Premissas e diagnóstico

`crexe_deepseek_api_key` será tratado como o nome da variável de ambiente que contém a credencial, não como o segredo literal. Sua presença/localização ainda não foi verificada; nenhum valor foi lido para preparar este plano. A chave também poderá ser informada pela tela e salva no cofre do sistema, sem exigir variável de ambiente.

A engine atual já usa HTTP com `reqwest`, autenticação Bearer, JSON de múltiplos arquivos e adaptadores Ollama/OpenAI. O DeepSeek documenta compatibilidade com Chat Completions; isso permite compartilhar transporte e validação da resposta. Contudo, precisa de tratamento explícito de parâmetros, identificação na interface e erros. A compatibilidade do protocolo não comprova a qualidade dos programas gerados. [API oficial](https://api-docs.deepseek.com/api/create-chat-completion/).

Dois pontos locais exigem atenção: o editor ainda oferece uma lista fixa com `local` e `openai`; o filtro de credenciais dos subprocessos precisa reconhecer o nome DeepSeek mesmo quando uma configuração antiga ainda não contém esse perfil.

## 2. Configuração e credenciais

Adicionar `Kind::Deepseek`, serializado como `kind = "deepseek"`, e um preset próprio. O trecho abaixo é a configuração proposta, ainda não aceito pelo binário atual:

```toml
[providers.deepseek]
kind = "deepseek"
base_url = "https://api.deepseek.com"
model = "deepseek-flash"
api_key_env = "crexe_deepseek_api_key"
timeout_seconds = 300
max_output_tokens = 6000
thinking = false
```

O guia oficial de JSON usa esse endpoint/modelo. A seleção inicial deverá ser conferida contra o catálogo da conta durante a implementação; a tela continuará aceitando modelo manual. Não presumir que aliases de exemplos antigos permanecem disponíveis. [Guia JSON](https://api-docs.deepseek.com/guides/json_mode/), [catálogo oficial](https://api-docs.deepseek.com/api/list-models/).

Reaproveitar as regras existentes:

- Seleção explícita pela tela, `default_provider` ou CLI. A atualização não altera o provider ativo de instalações existentes; novas instalações mantêm o default local atual.
- Cofre do usuário com referência vinculada à configuração, perfil e endpoint; não reutilizar a credencial OpenAI no destino DeepSeek.
- Com referência de cofre: override correspondente em `--env-file` explícito, depois cofre. Sem referência: resolução legada por ambiente/arquivo explícito/variável do usuário Windows.
- Retirar `crexe_deepseek_api_key` e `DEEPSEEK_API_KEY` do ambiente de todos os filhos, inclusive quando o provider selecionado for outro. Os nomes personalizados configurados continuam filtrados. Isso não transforma o workspace no host em isolamento de segurança.
- Preservar snapshot da credencial durante uma geração, exclusão da chave/referência da identidade do cache e escrita atômica da configuração.

## 3. Adaptador de geração

Implementar uma ramificação DeepSeek pequena, compartilhando a parte compatível de HTTP/decodificação, sem introduzir SDK de fornecedor nem reformular toda a arquitetura de providers.

Contrato proposto:

- Enviar `POST /chat/completions`, Bearer, `model`, mensagens, `stream: false`, `max_tokens` e `response_format = {"type":"json_object"}`. Não enviar automaticamente `max_completion_tokens`, opções Ollama ou parâmetros inferidos de nomes de modelos OpenAI.
- Traduzir o campo local `thinking` para `thinking.type` explícito. Nesta integração, ausente equivale a desativado; `true` habilita o modo. O contrato oficial permite ambos e ativa raciocínio quando omitido na requisição. O default desativado é uma decisão inicial do CREXE para validar latência e orçamento; não é afirmação de qualidade superior.
- Usar `temperature = 0.2` somente com raciocínio desativado. Não adicionar um seletor de esforço de raciocínio neste primeiro incremento.
- Aceitar somente conteúdo final não vazio, `finish_reason = stop` e estrutura de arquivos válida. Conteúdo de raciocínio não vira código, log ou ZIP. Recusa, truncamento, retorno de tools e interrupção do servidor falham de forma explícita.

Essas opções estão descritas no [contrato de Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/).

O prompt deve pedir JSON e incluir um exemplo mínimo de `files` com `path` e `content`. O guia alerta para conteúdo vazio; tratar vazio/whitespace/JSON incompleto com erro claro, sem publicar cache parcial ou iniciar tentativas pagas extras. Manter os limites de tamanho de resposta, tempo, chamadas e tokens definidos localmente. [Orientações de JSON](https://api-docs.deepseek.com/guides/json_mode/).

O raciocínio será uma opção avançada, inicialmente desligada. Ao habilitá-lo, orientar que pode aumentar tempo/uso de tokens e exigir mais espaço de saída. Normalizar seu valor efetivo para a identidade do novo provider; não mudar a identidade dos perfis legados Ollama/OpenAI. Alterar chave mantém cache; alterar provider/modelo/modo de geração distingue revisões.

## 4. Tela, catálogo e mensagens

Derivar os presets disponíveis da configuração de exemplo, em vez de acrescentar outra lista fixa. Assim DeepSeek aparece também em instalações com TOML antigo, mas seu perfil só é persistido quando o usuário salva. Preservar perfis personalizados e regras de execução durante seleção/importação.

Mostrar o serviço como **DeepSeek**, com modelo, campo de API key, estado da chave salva e opção avançada de raciocínio. Reaproveitar os botões de salvar, recarregar e testar conexão, com operações de rede/cofre fora da thread gráfica.

O teste de conexão consulta `GET /models`, lê `data[].id` e não gera programas. Campos adicionais desconhecidos no catálogo não devem quebrar a listagem; uma indisponibilidade não impede digitar o modelo. Preservar timeout e limites de resposta atuais. [Listagem de modelos](https://api-docs.deepseek.com/api/list-models/).

Mensagens devem distinguir credencial inválida (401), saldo insuficiente (402), parâmetros inválidos (400/422), limite de requisições (429) e erro/sobrecarga (500/503), sem imprimir corpos HTTP arbitrários. Também cobrir timeout e falha de conexão. Não comprar créditos, trocar de fornecedor/modelo ou repetir automaticamente uma chamada paga. [Códigos oficiais](https://api-docs.deepseek.com/quick_start/error_codes/).

## 5. Sequência de implementação

| Etapa | Arquivos principais | Entrega verificável |
|---|---|---|
| Configuração | `src/config.rs`, `config.example.toml`, `src/executor.rs` | Novo tipo/preset, resolução da chave e filtro de herança, sem alteração de preferências existentes |
| Protocolo | `src/generator.rs` | Payload específico, resposta validada e erros do provider, preservando o ciclo atual de build/reparo |
| Interface | `src/config_editor.rs`, `src/configure.rs` | DeepSeek disponível em TOML novo/antigo, catálogo, chave no cofre e opção de raciocínio |
| Regressão | `tests/cli.rs`, testes unitários e de UI | Servidores locais controlados comprovando comportamento sem consumo real |
| Aceitação e distribuição | `docs/CONFIGURACAO.md`, `README.md`, relatórios e pacotes | Testes reais delimitados, CI nos três OS e novo snapshot para o autor |

Nenhuma etapa modifica associação de arquivos, política do workspace, formato `.crexe` ou instalação dos SDKs. Streaming, tools de agente, Responses API e consulta de saldo ficam fora deste incremento.

## 6. Testes e aceitação

Primeiro, usar credenciais sintéticas e servidores HTTP locais:

1. Conferir endpoint, autorização, payload JSON, raciocínio ligado/desligado e ausência de parâmetros de outros providers.
2. Testar resposta válida, vazia, apenas raciocínio, JSON inválido, whitespace antes do JSON, truncamento e motivos de término não aceitos. Nenhuma resposta inválida publica uma revisão.
3. Simular os erros HTTP e timeout. Contar chamadas para garantir ausência de fallback e repetição escondida.
4. Gerar vários fontes, provocar um erro de compilação controlado, aplicar reparo, compilar e executar; validar cache, edição da intenção e exportação recompilável.
5. Abrir uma configuração antiga, selecionar DeepSeek, salvar/reabrir e observar que a próxima chamada vai apenas ao destino selecionado. Verificar importação, preservação dos demais perfis e conflito de edição.
6. Exercitar cofre com chaves sintéticas, precedência, troca de endpoint e cache após troca de chave. Conferir que subprocessos não herdam a variável DeepSeek, inclusive sem perfil DeepSeek salvo.
7. Repetir regressões de Ollama/OpenAI, clique duplicado, cancelamento e configuração. Executar fmt, clippy, testes e empacotamento em Windows/Linux/macOS; distinguir build de aceitação visual em cada desktop.

Depois de iniciada a implementação, a validação real proposta é: consulta do catálogo; geração de uma calculadora pequena; alteração pedindo fundo verde; reabertura de cada revisão sem novas chamadas; build do ZIP exportado. Limitar esse ensaio a **duas intenções, até um reparo por intenção, no máximo quatro chamadas de geração e 6000 tokens de saída reservados por chamada**. Interromper ao alcançar o limite; não ampliar orçamento silenciosamente. Esses limites não representam teto monetário, pois há tokens de entrada e tarifação do serviço.

Registrar modelo solicitado/retornado, tempo, quantidade de chamadas, uso de tokens quando informado pela API, resultado do build, testes funcionais e cache. Nenhum segredo ou conteúdo bruto de raciocínio entra no relatório. Não executar chamadas reais durante a preparação deste plano.

Critério de conclusão: DeepSeek selecionável e persistente na UI/CLI, chave funcionando por cofre e pela referência indicada, geração/reparo/ZIP/cache comprovados, erros compreensíveis e nenhuma regressão nos providers existentes. A integração só será declarada validada após esses ensaios; atualizar modelo por disponibilidade exige registrar a escolha, sem substituição silenciosa.
