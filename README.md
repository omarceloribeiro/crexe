# CREXE — Creative Executable

Um arquivo `.crexe` descreve a intenção de um aplicativo. A engine gera um projeto, compila, testa quando o perfil oferece testes, guarda uma revisão no cache e abre o programa. Editar a intenção gera uma nova revisão; reabrir sem alterações reutiliza a revisão validada.

**Em desenvolvimento na branch `feature/v1`; ainda não é uma release pública.** O executor original está preservado na tag `baseline/pre-v1`. Veja o [plano](docs/PLANO_V1.md) e o [registro da implementação](docs/IMPLEMENTACAO_V1.md) para distinguir funcionalidades presentes de critérios pendentes.

## Começar

Com Rust/Cargo e as ferramentas nativas de compilação, na raiz:

```sh
cargo build --release --locked
cargo test --locked
```

No Windows, `scripts\windows\build-executor.cmd` também funciona a partir de outra pasta. Compilar a engine usa o linker do Visual C++. O binário distribuído não exige Rust do usuário; os aplicativos gerados precisam do SDK do seu perfil.

O build MSVC inclui o runtime C estaticamente; a CI inspeciona o executável para evitar dependência das DLLs redistribuíveis do Visual C++. DLLs do próprio Windows continuam necessárias. `scripts/package-engine.py` prepara snapshots de desenvolvimento com binário, exemplos, configuração de exemplo, licença, origem do código e SHA-256. Os pacotes da CI servem para validação e ainda não são releases estáveis. [Guia de distribuição](docs/DISTRIBUICAO_V1.md).

Crie `calculadora.crexe` com apenas:

```text
Gerar uma calculadora simples com as quatro operações e fundo verde.
```

Com o Ollama e `gemma4:12b` disponíveis localmente, e o .NET SDK instalado:

```powershell
.\target\release\crexe.exe doctor
.\target\release\crexe.exe doctor --check
.\target\release\crexe.exe inspect calculadora.crexe
.\target\release\crexe.exe calculadora.crexe
```

O perfil preferido no Windows usa C#/.NET 8 e Windows Forms. Há também perfis experimentais C++/Win32, Objective-C++/Cocoa e C/GTK3, conforme o host e seus SDKs. A engine fornece o plano de build, pede os fontes, compila e executa o autoteste gerado antes de publicar o cache. Até duas propostas de correção podem ser solicitadas após erros de build/teste. A qualidade do programa ainda depende do modelo; um build aprovado não comprova todos os comportamentos pedidos. [Perfis e requisitos](docs/PERFIS_V1.md).

`doctor` informa arquitetura da engine/host, perfil, requisitos e caminhos. Com `--check`, consulta o SDK local antes da geração, sem chamar a IA. `inspect` mostra os requisitos do perfil junto da intenção resolvida; não executa SDKs.

## Provider e credenciais

Abra `crexe configure` (ou o executável sem argumentos) para escolher provider/modelo e salvar a API key no cofre do sistema. A tela mostra o arquivo ativo e permite importar um TOML para revisão. `config.example.toml` é um exemplo: editá-lo não altera a configuração ativa. Instalar/reinstalar preserva suas escolhas; os instaladores interativos abrem a tela ao concluir.

O arquivo ativo fica em `%APPDATA%\CREXE\config.toml` no Windows, `~/Library/Application Support/CREXE/config.toml` no macOS ou `${XDG_CONFIG_HOME:-$HOME/.config}/crexe/config.toml` no Linux. Alternativamente, use `--config caminho.toml`. Sem arquivo, os defaults são **Ollama local**, sem chave. A CLI continua disponível em hosts sem desktop.

Provider, endpoint e referência da credencial pertencem à configuração local. Os campos `generator` das receitas legadas não controlam esses valores; a CLI informa essa migração. Uma falha no Ollama nunca muda automaticamente para um serviço pago.

```powershell
# Seleção explícita de OpenAI; pode consumir créditos da API.
.\target\release\crexe.exe calculadora.crexe --provider openai

# Arquivo de segredos explicitamente selecionado; não é procurado ao lado da receita.
.\target\release\crexe.exe calculadora.crexe --provider openai --env-file .env
```

A configuração avançada por ambiente continua disponível: o exemplo referencia `crexe_openai_api_key_env`, no ambiente ou em uma linha do `.env` explicitamente selecionado. Uma chave salva pela tela tem prioridade sobre variáveis antigas; `--env-file` explícito pode substituí-la. O TOML guarda apenas uma referência vinculada ao destino, nunca a chave. Consulte [configuração, cofre e limites](docs/CONFIGURACAO.md).

## Instalar e abrir com dois cliques

```powershell
.\target\release\crexe.exe install
& "$env:LOCALAPPDATA\CREXE\releases\v1\crexe.exe" associate
```

A associação aponta para `%LOCALAPPDATA%\CREXE\releases\v1\crexe.exe` e habilita a janela “Creative Executable / Gerando seu programa…”. A espera acompanha a abertura da janela do aplicativo (até 15 segundos), restaura janelas minimizadas e solicita foco ao Windows. Se o sistema negar foco, a barra de tarefas sinaliza a abertura. Falhas aparecem em uma mensagem; fechar a espera cancela a preparação. A invocação por terminal preserva logs; `--ui` ativa a apresentação explicitamente.

Cliques repetidos durante a preparação avisam a execução existente, sem gerar ou compilar novamente. Se a intenção/opções mudaram, a espera orienta concluir ou cancelar primeiro. Depois que o aplicativo abre, outro clique pode abrir outra instância pelo cache.

O Windows pode exigir escolher CREXE como aplicativo padrão. `unassociate` remove apenas os registros desta instalação e preserva escolhas de outros aplicativos. No Linux/macOS, a CLI tem caminhos de instalação próprios e módulos selecionados por `cfg`; associação gráfica e janela de espera dessas plataformas ainda não estão implementadas. [Guia de plataformas](docs/platforms/README.md).

`CREXE_HOME` seleciona uma raiz absoluta para instalação, configuração e cache, útil em testes e instalações portáteis.

## Formatos, cache e ZIP

- [YAML legado](examples/yaml/calculator_native.crexe): targets e comandos explícitos. O exemplo original usa MinGW/Win32, Clang/Cocoa ou GCC/GTK3; esses compiladores não são instalados automaticamente.
- [Markdown](examples/markdown/regra-de-tres.crexe): front matter com `crexe: 1`, metadados opcionais e corpo livre.
- [Texto puro](examples/plaintext/calculadora.crexe): somente a intenção. O nome padrão vem do arquivo.

```powershell
crexe exec calculadora.crexe --no-run --export calculadora-fontes.zip
crexe calculadora.crexe --rebuild --max-repairs 2
```

Cada revisão aprovada inclui `source.zip`. No perfil .NET, ele contém os fontes, `.csproj`, instruções e scripts de build, run, test e publish. O ZIP pode ser recompilado sem CREXE, com os pré-requisitos indicados. O publish .NET é dependente do runtime; não instala nem publica na internet. A exportação exige um destino novo. YAML legado inclui scripts de build/run; não inventa uma operação publish ausente.

O manifesto `CREXE-PROJECT.json` guarda fontes e operações usados pela CLI e pelos scripts. Com uma revisão ou ZIP extraído, estas operações não chamam a IA nem precisam de uma chave:

```powershell
crexe build ./projeto-extraido --output ./projeto-recompilado
crexe test ./projeto-extraido
crexe run ./projeto-recompilado
crexe publish ./projeto-extraido --output ./projeto-publicado
crexe export ./projeto-extraido --output ./fontes.zip
```

Build/test/publish usam uma nova pasta temporária; build/publish entregam uma nova pasta de saída e preservam o projeto de origem. Export empacota os fontes atuais sem recompilá-los. Um ZIP de versões anteriores sem manifesto precisa ser regenerado para usar esses comandos; seus scripts continuam utilizáveis. [Contrato das operações](docs/PROJETOS_V1.md).

O cache tem revisões imutáveis, manifesto de hashes e troca atômica do ponteiro de revisão. Builds com falha preservam a revisão anterior e o workspace de diagnóstico. Arquivo ausente/modificado no cache exige regeneração. Copiar apenas o `.crexe` compartilha a intenção, sem transportar o cache. `--cache-dir` muda a raiz por decisão local.

## Limites atuais

Build e aplicativo usam o **host com as permissões do usuário**. A pasta temporária organiza o trabalho e não isola arquivos, rede ou processos. Docker e WSL não são requisitos. Windows Sandbox é proibido nesta fase. Use receitas e código que você esteja disposto a executar localmente; não há garantia de segurança para programas externos.

Há validação de caminhos, limites de resposta/arquivos/logs, controle de subprocessos, timeout e cancelamento, sem herança das credenciais configuradas. Isso não impede um programa nativo malicioso de acessar o host. `allowNetwork: false` é rejeitado porque a engine não oferece isolamento de rede.

Ferramentas e limites de chamadas, tokens de saída e tempo são definidos na configuração local; a receita não pode ampliá-los. Ainda faltam, entre outros critérios: validação com modelos reais e uso de aplicativos em toda a matriz nativa, triagem de compatibilidade, gestão de dependências além do SDK e preparação da release. Confira o registro de implementação antes de distribuir.

## Documentação e licença

- [Contrato implementado dos formatos](docs/FORMATOS_V1.md)
- [Configuração](docs/CONFIGURACAO.md), [contribuição](CONTRIBUTING.md) e [limites de segurança](SECURITY.md)
- [RFCs e visão histórica](docs/crexe_handoff_docs/README.md), em reconciliação com a implementação
- [Baseline e arquivos históricos](docs/archive/README.md)
- [Análise do Ollama local](docs/ANALISE_OLLAMA_LOCAL_2026-09-23.md)

A engine usa a [GNU GPL v3](LICENSE) escolhida para este repositório. O manifest referencia esse arquivo sem acrescentar permissão para versões futuras. A licença de artefatos gerados depende do projeto e das dependências; ela não é definida automaticamente pela licença da engine.
