# Gerar com CREXE

[English](en/GENERATE_WITH_CREXE.md) · [Guia principal](../README.md)

**Estado: implementação v1 em desenvolvimento.** Um documento comum pode iniciar a geração de um leitor local. A engine prepara o programa a partir do tipo do arquivo e entrega o documento ao programa somente quando ele abre. Este guia descreve o comportamento implementado, os limites da prévia e os ensaios necessários antes da publicação.

## Usar pela CLI ou pela janela

Na pasta extraída do pacote Windows:

```powershell
.\crexe.exe configure
.\crexe.exe doctor --check
.\crexe.exe open-generating -- "notas da reunião.md"
```

No macOS/Linux, substitua `.\crexe.exe` por `./crexe`. Em um checkout compilado, use `target/release/crexe` ou `target\release\crexe.exe`. Os exemplos abaixo usam `crexe` supondo que o executável esteja no PATH.

```sh
# Um arquivo por invocação; -- também permite nomes iniciados por hífen.
crexe open-generating -- "-notas da reunião.md"

# Perfil explicitamente escolhido; não depende da seleção do modelo.
crexe open-generating --profile dotnet-console -- "amostra.tipo-desconhecido"

# Gera, compila, testa e exporta fontes; não abre o documento no programa final.
crexe open-generating --no-run --export leitor-fontes.zip -- "notas.md"

# Limita reparos de compilação/testes; o planejamento continua sendo uma chamada.
crexe open-generating --max-repairs 0 -- "notas.md"
```

O destino da exportação deve ser um arquivo novo. O ZIP contém o projeto do leitor, não uma cópia do documento selecionado. `--no-run` ainda pode consultar o modelo, compilar e testar quando não existe leitor aprovado.

O comando aceita arquivos locais regulares com qualquer extensão, inclusive sem extensão. Caminhos relativos partem da pasta de quem chamou. Nomes com espaços, acentos ou hífen são argumentos separados; não viram comandos de shell. Um arquivo `.crexe` segue o fluxo de execução de receita existente.

Abra a engine sem argumentos ou use `crexe configure`. Em **Gerar com CREXE**, clique em **Selecionar arquivo e gerar programa…**. O seletor permite um arquivo por vez e não filtra extensões. Cancelá-lo encerra a seleção sem geração nem chamada ao modelo. A abertura usa a configuração salva; mudanças ainda não salvas na tela continuam preservadas e não são aplicadas ao leitor.

No Windows, `install` e `associate` registram **Gerar com CREXE** no menu de contexto dos arquivos. No Windows 11, procure também em **Mostrar mais opções**. Essa ação preserva o aplicativo padrão de cada documento. Para macOS/Linux, o seletor da janela é a alternativa gráfica desta prévia; integração com menus de Finder, GNOME ou Dolphin não está implementada. A CLI também funciona sem sessão gráfica quando o perfil escolhido é de console.

## SDKs e escolha do perfil

O provider, modelo e credenciais vêm da [configuração local](CONFIGURACAO.md). Ollama, OpenAI e DeepSeek usam o mesmo transporte já disponível na engine. A engine não troca automaticamente de provider após uma falha.

| Host ou escolha | Perfil do leitor | Pré-requisitos |
|---|---|---|
| Windows, preferência padrão | `dotnet-winforms` | .NET 8 **SDK** no PATH; Windows Forms |
| Windows, alternativa nativa | `cpp-win32` | MinGW-w64 `g++`, com arquitetura compatível com a engine |
| macOS, padrão | `objc-cocoa` | Apple Clang, Command Line Tools e SDK Cocoa |
| Linux, padrão | `c-gtk` | GCC, GNU Make, `pkg-config` e bibliotecas/headers GTK3 |
| Console, escolha explícita | `dotnet-console` | .NET 8 SDK; disponível nos três hosts |

No Windows, a preferência inicial muda para `cpp-win32` somente quando `dotnet` não é encontrado e `g++` está disponível. Encontrar `dotnet` não prova que o SDK 8 está instalado: o preflight confere esse requisito. Um runtime .NET sem o SDK não basta para gerar. Use `--profile` para escolher outra opção deliberadamente; o leitor mantém esse perfil durante planejamento, geração e reparos. Não há uma tentativa automática em outro perfil após uma falha.

```sh
crexe doctor --check --profile dotnet-winforms
crexe doctor --check --profile cpp-win32
```

Execute o comando correspondente ao host e perfil que pretende usar. O preflight consulta ferramentas locais antes do primeiro planejamento, sem chamar o modelo; a engine não instala SDKs. Os [perfis completos](PERFIS_V1.md) descrevem requisitos adicionais e limitações de distribuição. Um leitor em cache ainda precisa das bibliotecas/runtime usados pelo executável.

## Plano implementado e arquitetura

1. **Validar a entrada e o ambiente.** Resolver o caminho local, rejeitar pasta/dispositivo, identificar o tipo e fixar o perfil. Verificar orçamento e SDK antes da primeira chamada.
2. **Coordenar a preparação por leitor.** Adquirir um lock local para a combinação de tipo e configuração. Outra abertura compatível aguarda essa preparação.
3. **Planejar uma vez.** Enviar apenas o identificador de tipo, o perfil e o contrato do leitor. A resposta deve ser um JSON com um único campo `prompt`; não é uma resposta `files[]`. Há uma única tentativa de planejamento nessa preparação, sem repetição automática ou fallback.
4. **Persistir a intenção interna.** Validar o prompt e salvar o registro de planejamento e a receita interna de forma atômica, com identidade e checksum. A receita tem nome e perfil controlados pela engine.
5. **Gerar e verificar o programa.** Usar o pipeline existente de `files[]`, projeto da engine, build e autoteste. Acrescentar os checks de leitura descritos abaixo. Falhas de build/teste podem pedir correções dentro do orçamento restante.
6. **Publicar e abrir.** Atualizar o histórico somente após aprovação. Liberar a preparação compartilhada e iniciar o executável com o caminho do documento como um argumento nativo. Cada abertura recebe seu próprio arquivo.

Planejamento, geração e reparos usam a **mesma sessão**, com os mesmos limites de solicitações, tokens de saída e tempo e a mesma credencial resolvida. A primeira preparação exige ao menos duas solicitações disponíveis e tokens suficientes para o plano e uma geração completa. O plano reserva até 2.048 tokens de saída, limitado também pelo valor configurado por chamada.

| Situação | Chamadas ao modelo |
|---|---|
| Primeiro leitor, código aprovado sem reparo | 1 planejamento + 1 geração |
| Primeiro leitor, um reparo necessário | 1 planejamento + 1 geração + 1 reparo, se houver orçamento |
| Outro documento do mesmo tipo, leitor compatível aprovado | 0 |
| Somente o conteúdo do documento mudou | 0, enquanto o leitor continua válido |
| Falha de planejamento | Sem segunda tentativa automática de planejamento |
| Falha após salvar um plano válido, seguida de nova abertura explícita | Reutiliza o plano e tenta gerar com o orçamento da nova invocação |

O padrão de três solicitações permite plano, geração e um reparo. `--max-repairs` pode reduzir correções; não amplia o orçamento local. Uma falha HTTP ou de resposta do provider encerra a operação, sem tentativas ocultas. Providers remotos podem cobrar por cada solicitação, inclusive pelas que falham no restante do fluxo.

O cache do leitor considera a extensão normalizada ou a ausência de extensão, os parâmetros de geração do provider/modelo, o perfil, OS/arquitetura da engine e a versão do contrato. Extensões opacas recebem um identificador derivado de seus bytes; não entram como texto livre no prompt. Nome, diretório e conteúdo do documento não fazem parte dessa identidade. Alterar apenas os tetos de solicitações/tokens totais não invalida um leitor aprovado.

O estado interno fica em `readers/<identidade>/`, na raiz gerenciada da instalação; as revisões aprovadas usam o cache/histórico da engine. `CREXE_HOME` permite uma raiz absoluta isolada para testes. Não é necessário criar, editar ou distribuir essas receitas internas para abrir documentos.

Duas aberturas simultâneas do mesmo leitor compartilham uma preparação e, após sucesso, abrem cada documento. Se a preparação falhar ou for cancelada, quem aguardava recebe erro sem iniciar novas chamadas. Uma nova tentativa exige outra invocação explícita. O lock é liberado antes de aguardar o fechamento do aplicativo, permitindo abrir outros documentos enquanto ele permanece aberto.

## Privacidade e limites do leitor v1

O caminho, nome e corpo do documento selecionado não são incluídos no planejamento, na geração, nos reparos, no histórico da receita ou no ZIP dos fontes. O caminho permanece local e só chega ao executável na abertura. Nome e caminho podem aparecer na interface local para identificar o arquivo. Os checks anteriores à publicação usam arquivos de teste sintéticos, sem ler o documento escolhido.

O contrato exige um leitor **somente leitura**: abrir o arquivo selecionado imediatamente, mostrar seu conteúdo real ou um erro legível e preservar o original. Edição mínima, salvar como e interpretação completa de formatos proprietários não fazem parte desta entrega.

- A prévia lê até **65.536 bytes (64 KiB)**, mais um byte para detectar truncamento.
- UTF-8 válido sem bytes NUL é apresentado como texto; outros dados recebem uma prévia hexadecimal identificada. Um corte no meio de uma sequência UTF-8 também pode produzir a prévia hexadecimal.
- Arquivo vazio tem indicação própria; conteúdo maior que o limite deve mostrar aviso de truncamento.
- Formatos conhecidos podem receber apresentação simples, como títulos Markdown ou assunto/corpo EML. Formatos opacos podem oferecer somente texto ou bytes, sem afirmar que foram completamente interpretados.
- O contrato proíbe executar scripts do documento, buscar recursos externos e escrever no original.

Antes de aprovar uma revisão, a engine executa `--crexe-reader-check PATH` em dois textos distintos, arquivo vazio, bytes binários e arquivo acima do limite. A resposta JSON deve conter `kind`, `content` e `truncated`, com os bytes esperados; a engine também confere que os arquivos de teste foram preservados. A implementação gerada deve usar a mesma leitura limitada na interface e nesse modo sem janela.

Esses checks verificam o contrato observável; não são uma sandbox nem comprovam toda a interface ou a segurança do código gerado. O executável roda no host com as permissões do usuário. A proteção contra envio do documento pelo fluxo da engine não equivale a isolamento de rede/arquivos de um programa nativo. Consulte os [limites de segurança](../SECURITY.md).

## Verificação automatizada e evidência local

Execute os comandos abaixo na revisão candidata à publicação. Registre separadamente os resultados automatizados, a geração com modelo real e os percursos gráficos em cada sistema.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib
cargo test --locked --test cli
cargo test --locked --test open_generating -- --ignored --nocapture
```

O último comando requer .NET 8 SDK e usa um servidor local controlado, sem LLM real, API paga ou interação com janelas. Cobre cache, extensão arbitrária/ausente, texto/vazio/binário/truncamento, reparo com orçamento compartilhado, privacidade/exportação, concorrência com sucesso/falha, pré-requisitos inválidos, delegação de `.crexe` e rejeição de leitor com conteúdo fixo. Há um caso adicional em Linux para caminho com bytes não UTF-8; o sistema de arquivos do runner macOS rejeita esse nome antes da execução da engine. Os testes unitários do transporte cobrem os três providers e o contrato de planejamento.

A [configuração da CI](../.github/workflows/ci.yml) inclui a suíte de leitores nos runners; a existência dessa etapa não substitui conferir o resultado de uma execução no commit candidato.

## Aceitação manual: CLI, menu e seletor

Use arquivos sintéticos e uma pasta nova. Registre commit/pacote, host, perfil, SDK, provider/modelo, resultado e tempos. Para uma primeira geração verificável, use um tipo ainda não preparado ou uma instalação de teste isolada, sem remover o cache pessoal.

| Percurso | Ação e resultado a conferir |
|---|---|
| CLI | Abrir um arquivo com espaços/acentos e outro iniciado por hífen usando `--`; o conteúdo e o título correspondem ao arquivo selecionado. |
| Reutilização | Abrir outro arquivo com a mesma extensão e conteúdo diferente; o leitor mostra o segundo conteúdo, sem planejamento, geração ou recompilação. |
| Dados alterados | Modificar apenas o conteúdo do segundo documento e reabrir; a prévia muda e o leitor continua vindo do cache. |
| Limites | Abrir vazio, binário e arquivo maior que 64 KiB; conferir indicações de vazio, hexadecimal e truncamento. Comparar os hashes dos originais antes/depois. |
| Windows Explorer | Após instalação/associação, clicar com o botão direito e selecionar **Gerar com CREXE**; no Windows 11, conferir **Mostrar mais opções**. O documento chega ao leitor sem nova seleção e seu aplicativo padrão permanece igual. |
| Seletor da engine | Abrir `configure`, selecionar o documento e acompanhar até o leitor abrir. A janela da engine continua disponível; depois da preparação, selecionar outro documento. |
| Configuração pendente | Editar uma preferência sem salvar e usar o seletor; a abertura usa a configuração salva e a edição pendente continua na tela. |
| Cancelamento do seletor | Abrir e cancelar a seleção; nenhuma geração é iniciada. |
| Falha antes da geração | Em ambiente de teste sem o SDK ou com orçamento insuficiente, abrir um tipo novo; conferir erro e ausência de solicitações ao provider. |
| Concorrência/falha | Abrir dois documentos do mesmo tipo durante a preparação; conferir um preparo compartilhado. Em falha, ambos recebem diagnóstico, sem uma nova geração automática de quem aguardava. |

No Windows, complemente com o [roteiro de instalação fora do ambiente de desenvolvimento](VALIDACAO_INSTALACAO_WINDOWS.md). Para cada plataforma, uma execução com provider real e interação com o leitor continua sendo um ensaio próprio; os mocks não substituem essa evidência.

## Roteiro de vídeo

1. **Mostrar a intenção.** Criar `calculadora.crexe` com “Gerar uma calculadora simples com as quatro operações e fundo verde.” Mostrar o arquivo de texto e abri-lo com dois cliques após a associação.
2. **Mostrar geração e funcionamento.** Registrar o provider/modelo, as fases e o tempo até o aplicativo abrir. Fazer **6 × 7 = 42**. Fechar e reabrir para mostrar a reutilização do cache.
3. **Evoluir a intenção.** Usar **Editar** ou `crexe edit calculadora.crexe`, mudar o pedido para fundo **azul**, salvar e reabrir. Mostrar o fundo azul e repetir **6 × 7 = 42**, verificando que a função continua disponível.
4. **Partir de um documento comum.** Criar um texto sintético `amostra.tipo-demo-inedito`, com duas linhas reconhecíveis. Mostrar o conteúdo e tentar dois cliques. Se o Windows oferecer escolher um aplicativo por não reconhecer a extensão, cancelar essa janela; esse comportamento depende das associações do host.
5. **Usar Gerar com CREXE.** Clicar com o botão direito no mesmo arquivo; no Windows 11, abrir **Mostrar mais opções**, quando necessário. Escolher **Gerar com CREXE**, mostrar o planejamento único e a geração, e conferir o conteúdo real no leitor.
6. **Mostrar a segunda abertura.** Abrir outro documento da mesma extensão com conteúdo diferente pela mesma ação. Conferir conteúdo novo e cache, sem confundir esse tempo com o da primeira geração.
7. **Mostrar a alternativa gráfica.** Abrir a janela da engine, usar **Selecionar arquivo e gerar programa…** e selecionar um documento. Mostrar também um cancelamento sem geração.

Grave a latência real de planejamento, geração, build e abertura. Se cortar esperas ou acelerar o vídeo, indique isso na tela e informe o tempo total medido. Identifique claramente primeiro uso e cache. Não apresente uma resposta de provider simulado como geração por LLM, nem uma prévia hexadecimal como suporte completo ao formato.
