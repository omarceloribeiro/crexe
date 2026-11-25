# CREXE — Creative Executable

Um arquivo `.crexe` descreve a intenção de um aplicativo. A engine gera um projeto, compila, testa quando o perfil oferece testes, guarda uma revisão no cache e abre o programa. Editar a intenção evolui a última revisão aprovada; reabrir sem alterações reutiliza a revisão validada.

Com **Gerar com CREXE**, o ponto de partida também pode ser um documento comum: selecione o arquivo e a engine gera ou reutiliza um leitor, que recebe esse documento ao abrir.

**Em desenvolvimento na branch `feature/v1`; ainda não é uma release pública.** O executor original está preservado na tag `baseline/pre-v1`. Veja o [plano](docs/PLANO_V1.md) e o [registro da implementação](docs/IMPLEMENTACAO_V1.md) para distinguir funcionalidades presentes de critérios pendentes.

## CREXP e CREXB

A prévia inclui **CREXP — Creative Executable Package**, um ZIP de documentos SDD em Markdown que alimenta a geração, e **CREXB — Creative Executable Bundle**, que preserva a receita e o snapshot aceito para abrir em um target compatível sem IA nem recompilação. São subconjuntos mínimos: runtime e bibliotecas continuam necessários, e os aplicativos não têm sandbox.

Consulte os guias de [CREXP](docs/publicacoes/crexp.md), [CREXB](docs/publicacoes/crexb.md) e o [roadmap completo](docs/publicacoes/roadmap-formatos.md). Os [handoffs e JSON originais](docs/formatos/originais/README.md) estão preservados integralmente para download. Site e guias têm edição em português e inglês; os handoffs completos permanecem no original em português, identificados dessa forma.

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

O perfil preferido no Windows usa C#/.NET 8 e Windows Forms. Há também perfis C++/Win32, Objective-C++/Cocoa, C/GTK3 e OpenGL específico para cada host. Em texto/Markdown sem perfil explícito, o modelo pode escolher entre os perfis com pré-requisitos disponíveis, justificando mudanças de tecnologia. A engine fornece projetos e comandos, restaura bibliotecas cadastradas, compila e executa o autoteste antes de publicar o cache. São 3 tentativas totais por padrão, configuráveis de 1 a 10. A qualidade do programa ainda depende do modelo; um build aprovado não comprova todos os comportamentos pedidos. [Perfis e requisitos](docs/PERFIS_V1.md).

`doctor` informa arquitetura da engine/host, perfil, requisitos e caminhos. Com `--check`, consulta o SDK local antes da geração, sem chamar a IA. `inspect` mostra os requisitos do perfil junto da intenção resolvida; não executa SDKs.

## Gerar com CREXE

O [guia completo da feature](docs/GERAR_COM_CREXE.md) detalha chamadas, cache, privacidade, SDKs, testes de aceitação e roteiro de demonstração. Também está disponível [em inglês](docs/en/GENERATE_WITH_CREXE.md).

Para abrir um documento sem escrever uma receita, use **Selecionar arquivo e gerar programa…** na janela da engine. No Windows, após `install` e `associate`, a ação **Gerar com CREXE** também aparece no menu de um arquivo; no Windows 11, pode ficar em **Mostrar mais opções**. No macOS e Linux, a entrada gráfica desta prévia é o seletor na janela, sem integração de menu Finder/GNOME/Dolphin anunciada.

Na CLI, a partir da pasta extraída do pacote:

```powershell
.\crexe.exe open-generating -- "notas da reunião.md"
```

No macOS/Linux, use `./crexe open-generating -- "notas da reunião.md"`. Para a engine compilada do checkout, o executável fica em `target/release`. O caminho é resolvido a partir da pasta de quem chamou; `--` permite nomes iniciados por hífen. O comando aceita um arquivo local regular por vez com extensão válida.

Na primeira preparação de um leitor, **uma única chamada à LLM formula a intenção a partir da extensão do arquivo**. Depois ocorre a geração normal de código, com build, autotestes e correções limitadas. O planejamento tem contagem separada: o padrão permite um plano e até três tentativas de código, respeitando os limites de tokens e tempo. O programa abre com o documento selecionado, sem pedir a mesma seleção novamente. Outro documento do mesmo tipo compatível reutiliza o leitor aprovado: cache válido significa zero chamadas extras ao modelo e nenhuma recompilação. Alterar somente o conteúdo de um documento não muda a identidade do leitor.

Extensões desconhecidas válidas são aceitas. Arquivos sem extensão ou com extensão inválida recebem aviso de falta de suporte, sem chamada ao modelo nem leitura de bytes para descobrir o tipo. Selecionar `.crexe`, `.crexp` ou `.crexb` nessa ação também retorna um aviso: receitas, pacotes e bundles devem ser abertos pelo fluxo normal de execução.

O nome, caminho e corpo do documento não entram nas requisições ao modelo, no histórico de geração ou no ZIP dos fontes. O aplicativo recebe o caminho como argumento separado e lê os dados localmente em runtime. A configuração ativa salva fornece provider, modelo e limites; selecionar um documento na janela não salva nem descarta alterações pendentes nas configurações. Cancelar o seletor não chama a IA.

O contrato desta prévia é **somente leitura**, preservando o original: conteúdo ou erro legível, indicação de arquivo vazio e limite de leitura. Para formatos desconhecidos, o leitor pode mostrar texto ou uma prévia hexadecimal identificada. Não há promessa de interpretar integralmente formatos proprietários. O contrato orienta o código gerado; ele não oferece sandbox nem garantia de segurança. Provider/modelo e SDK do perfil continuam necessários para gerar o primeiro leitor, e provedores remotos podem cobrar pelo planejamento, geração e reparos.

## Provider e credenciais

Abra `crexe configure` (ou o executável sem argumentos) para escolher provider/modelo, **Tentativas máximas** e salvar a API key no cofre do sistema. **Avançado** permite limitar o total de tokens de saída; a tela mostra quando esse teto reduz as tentativas efetivas. A tela mostra o arquivo ativo e permite importar um TOML para revisão. `config.example.toml` é um exemplo: editá-lo não altera a configuração ativa. Instalar/reinstalar preserva suas escolhas; os instaladores interativos abrem a tela ao concluir.

Os presets disponíveis são **Ollama**, **OpenAI** e **DeepSeek**. Para DeepSeek, selecione o perfil `deepseek`, informe a API key e salve. O preset usa `deepseek-flash`, com raciocínio desligado por padrão; a opção pode ser alterada em **Avançado**. Instalações antigas também oferecem o novo perfil na tela, sem trocar o provider ativo até salvar.

O arquivo ativo fica em `%APPDATA%\CREXE\config.toml` no Windows, `~/Library/Application Support/CREXE/config.toml` no macOS ou `${XDG_CONFIG_HOME:-$HOME/.config}/crexe/config.toml` no Linux. Alternativamente, use `--config caminho.toml`. Sem arquivo, os defaults são **Ollama local**, sem chave. A CLI continua disponível em hosts sem desktop.

Provider, endpoint e referência da credencial pertencem à configuração local. Os campos `generator` das receitas legadas não controlam esses valores; a CLI informa essa migração. Uma falha no Ollama nunca muda automaticamente para um serviço pago.

```powershell
# Seleção explícita de OpenAI; pode consumir créditos da API.
.\target\release\crexe.exe calculadora.crexe --provider openai

# Arquivo de segredos explicitamente selecionado; não é procurado ao lado da receita.
.\target\release\crexe.exe calculadora.crexe --provider openai --env-file .env

# DeepSeek: após salvar o perfil na tela, ou usando --config com seu TOML.
.\target\release\crexe.exe calculadora.crexe --provider deepseek
```

A configuração avançada por ambiente continua disponível: o exemplo referencia `crexe_openai_api_key_env`, no ambiente ou em uma linha do `.env` explicitamente selecionado. Uma chave salva pela tela tem prioridade sobre variáveis antigas; `--env-file` explícito pode substituí-la. O TOML guarda apenas uma referência vinculada ao destino, nunca a chave. Consulte [configuração, cofre e limites](docs/CONFIGURACAO.md).

O perfil DeepSeek referencia `crexe_deepseek_api_key`, também compatível com `--env-file .env`. O saldo da API é gerenciado pelo usuário no provider; a engine não compra créditos nem troca de serviço automaticamente.

## Instalar e abrir com dois cliques

```powershell
.\target\release\crexe.exe install
& "$env:LOCALAPPDATA\CREXE\releases\v1\crexe.exe" associate
```

A associação aponta para `%LOCALAPPDATA%\CREXE\releases\v1\crexe.exe` e habilita a janela “Creative Executable / Gerando seu programa…”. Ela mostra provider, modelo e “Tentativa 1 de 3 — até 2 correções”, separadamente da fase atual. A espera acompanha a abertura da janela do aplicativo (até 15 segundos), restaura janelas minimizadas e solicita foco ao Windows. Se o sistema negar foco, a barra de tarefas sinaliza a abertura. Falhas aparecem em uma mensagem; fechar a espera cancela a preparação. A invocação por terminal preserva logs; `--ui` ativa a apresentação explicitamente.

Cliques repetidos durante a preparação avisam a execução existente, sem gerar ou compilar novamente. Se a intenção/opções mudaram, a espera orienta concluir ou cancelar primeiro. Depois que o aplicativo abre, outro clique pode abrir outra instância pelo cache.

O Windows pode exigir escolher CREXE como aplicativo padrão de `.crexe`. A ação genérica **Gerar com CREXE** não altera os aplicativos padrão dos demais documentos. `unassociate` remove a associação e o menu pertencentes a esta instalação e preserva escolhas de outros aplicativos. No Linux/macOS, a CLI tem caminhos de instalação próprios e módulos selecionados por `cfg`; a abertura gráfica de documentos comuns usa o seletor da janela. A associação de `.crexe` e os menus dos gerenciadores dessas plataformas ainda não estão implementados. [Guia de plataformas](docs/platforms/README.md).

`CREXE_HOME` seleciona uma raiz absoluta para instalação, configuração e cache, útil em testes e instalações portáteis.

## Editar a intenção

Use **Editar** no menu de contexto Windows (após registrar a associação atual) ou `crexe edit "calculadora.crexe"`. O editor integrado funciona em Windows, macOS e Linux com sessão gráfica. Cada abertura tem sua própria janela, com caminho, indicação de alterações, **Salvar**, **Recarregar** e **Fechar**. Oferece seleção, copiar/colar, desfazer/refazer e `Ctrl+S` (`Cmd+S` no macOS).

Salvar altera somente o documento. Para gerar ou atualizar o programa, abra o `.crexe` novamente. O editor não consulta providers nem carrega credenciais. Aceita texto vazio ou incompleto, preserva UTF-8, BOM e LF/CRLF e limita o arquivo a 1 MiB. A gravação é atômica; alterações externas bloqueiam o salvamento e mantêm o texto da janela para recuperação. Recarregar e fechar pedem confirmação quando há alterações locais.

Abrir um arquivo vazio, apenas com espaços ou BOM pelo fluxo gráfico Windows apresenta **“O arquivo está vazio. Abrir para edição?”**, com **Sim** e **Cancelar**. Sim abre o mesmo editor, sem iniciar geração após salvar. A CLI orienta usar `crexe edit`; sem sessão gráfica, o comando informa a limitação.

## Formatos, cache e ZIP

- [YAML legado](examples/yaml/calculator_native.crexe): targets e comandos explícitos. O exemplo original usa MinGW/Win32, Clang/Cocoa ou GCC/GTK3; esses compiladores não são instalados automaticamente.
- [Markdown](examples/markdown/regra-de-tres.crexe): front matter com `crexe: 1`, metadados opcionais e corpo livre.
- [Texto puro](examples/plaintext/calculadora.crexe): somente a intenção. O nome padrão vem do arquivo.

```powershell
crexe exec calculadora.crexe --no-run --export calculadora-fontes.zip
crexe calculadora.crexe --rebuild --max-repairs 2
crexe calculadora.crexe --fresh
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

Para receitas `.crexe`, o cache tem revisões imutáveis, hashes e um índice por caminho canônico da receita. Cada alteração envia o pedido anterior, os fontes aprovados e o pedido atual à IA. Ela recebe orientação para preservar comportamentos não afetados, podendo justificar uma recriação completa ou troca de perfil quando necessário. Os reparos usam a tentativa mais recente e seus diagnósticos. Binários, logs e diretórios de build não entram nesse contexto. Em **Gerar com CREXE**, a receita do leitor é interna e independente do documento aberto; o cache identifica o leitor compatível, não o conteúdo de cada arquivo.

O índice só avança após build/testes aprovados. Falha ou cancelamento preserva o baseline. `--rebuild` força nova geração a partir dele; `--fresh` gera sem ele. Fontes aprovados corrompidos ou contexto acima do limite causam diagnóstico, sem descarte silencioso. Cache antigo só é adotado com identidade e inventário verificáveis. Copiar ou mover o `.crexe` começa um histórico independente; `--cache-dir` seleciona outra raiz local. Alterar somente o limite de tentativas não invalida a revisão aprovada.

## Limites atuais

Build e aplicativo usam o **host com as permissões do usuário**. A pasta temporária organiza o trabalho e não isola arquivos, rede ou processos. Docker e WSL não são requisitos. Windows Sandbox é proibido nesta fase. Use receitas e código que você esteja disposto a executar localmente; não há garantia de segurança para programas externos.

Há validação de caminhos, limites de resposta/arquivos/logs, controle de subprocessos, timeout e cancelamento, sem herança das credenciais configuradas. Isso não impede um programa nativo malicioso de acessar o host. `allowNetwork: false` é rejeitado porque a engine não oferece isolamento de rede.

Ferramentas e limites de chamadas, tokens de saída e tempo são definidos na configuração local; a receita não pode ampliá-los. Ainda faltam, entre outros critérios: validação com modelos reais e uso de aplicativos em toda a matriz nativa, triagem de compatibilidade, dependências além do catálogo da engine e preparação da release. Confira o registro de implementação antes de distribuir.

## Documentação e licença

- [Site público de crexe.org](src/site/README.md): landing estática, exemplos e downloads; build e preview locais independentes da engine.
- [Biblioteca CREXE](docs/publicacoes/README.md): white papers, guias, artigos e manifesto; implementação atual e visão de futuro identificadas em cada texto.
- [Contrato implementado dos formatos](docs/FORMATOS_V1.md)
- [Gerar com CREXE: uso, arquitetura e validação](docs/GERAR_COM_CREXE.md)
- [Configuração](docs/CONFIGURACAO.md), [contribuição](CONTRIBUTING.md) e [limites de segurança](SECURITY.md)
- [RFCs e visão histórica](docs/crexe_handoff_docs/README.md), em reconciliação com a implementação
- [Baseline e arquivos históricos](docs/archive/README.md)
- [Análise do Ollama local](docs/ANALISE_OLLAMA_LOCAL_2026-09-23.md)

A engine usa a [licença MIT](LICENSE), identificada em `Cargo.toml`. Você pode usar, modificar e redistribuir a engine, inclusive em produtos proprietários, preservando os avisos de copyright e licença exigidos pela MIT. **Você escolhe a licença do aplicativo gerado, inclusive proprietária, e pode vendê-lo.** O CREXE não exige abrir seus fontes, pagar royalties ou atribuir crédito. Os materiais que a engine incorpora à saída e as receitas oficiais são [MIT-0](LICENSE-GENERATED); o ZIP inclui esse aviso sem substituir o `LICENSE` do aplicativo. Condições de dependências, assets, receitas de terceiros e do modelo/provider continuam aplicáveis. Consulte o [licenciamento e a organização do ecossistema](docs/LICENCIAMENTO_E_ECOSSISTEMA.md).
