# Gerar com CREXE

[English](en/GENERATE_WITH_CREXE.md) · [Guia principal](../README-pt-BR.md)

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

O comando aceita arquivos locais regulares com extensão válida, inclusive extensões desconhecidas. A extensão normalizada deve ter até 64 bytes UTF-8 e conter somente letras, números, hífen ou sublinhado. Arquivos sem extensão, com extensão vazia, inválida, muito longa ou não UTF-8 recebem um aviso de falta de suporte antes de qualquer chamada ao modelo. A engine não lê bytes do documento para descobrir o tipo. Caminhos relativos partem da pasta de quem chamou. Nomes com espaços, acentos ou hífen são argumentos separados; não viram comandos de shell.

Selecionar `.crexe`, `.crexp` ou `.crexb` nessa ação também retorna um aviso, sem enviar a receita ao modelo. Para executar uma receita, pacote ou bundle, use sua abertura normal ou `crexe exec receita.crexe`; **Gerar com CREXE** não delega silenciosamente para esse fluxo. No fluxo explícito de um [CREXP](publicacoes/crexp.md), os documentos SDD são a receita enviada ao provider; essa ação tem um contrato de privacidade diferente do leitor de documentos.

Abra a engine sem argumentos ou use `crexe configure`. Em **Gerar com CREXE**, clique em **Selecionar arquivo e gerar programa…**. O seletor permite um arquivo por vez e não filtra extensões. Cancelá-lo encerra a seleção sem geração nem chamada ao modelo. A abertura usa a configuração salva; mudanças ainda não salvas na tela continuam preservadas e não são aplicadas ao leitor.

No Windows, `install` e `associate` registram **Gerar com CREXE** no menu de contexto dos arquivos. No Windows 11, procure também em **Mostrar mais opções**. Essa ação preserva o aplicativo padrão de cada documento. Para macOS/Linux, o seletor da janela é a alternativa gráfica desta prévia; integração com menus de Finder, GNOME ou Dolphin não está implementada. A CLI também funciona sem sessão gráfica quando o perfil escolhido é de console.

## Manter o leitor como programa padrão no Windows

Depois de executar `crexe install` e `crexe associate`, o Explorer também oferece **Gerar com CREXE e Manter como Programa Padrão**. No Windows 11, a opção pode estar em **Mostrar mais opções**. Ela prepara o leitor, preserva a versão aceita fora do cache de geração e oferece a escolha do programa padrão na interface do Windows. A ação original **Gerar com CREXE** continua sem alterar padrões.

```powershell
.\crexe.exe open-generating --keep-as-default --ui -- "primeiro documento.xyz"
.\crexe.exe reader-defaults list
.\crexe.exe reader-defaults choose --extension xyz
```

Depois de observar uma janela utilizável do leitor, a ação com `--ui` mostra o convite **Leitor mantido — CREXE**, com **Sim/Não**, para abrir Configurações. Esse convite também aparece ao reutilizar um leitor pela nova ação explícita; a abertura comum pelo leitor mantido não o exibe. Sem `--ui`, ou se uma janela utilizável não for confirmada, a engine informa o comando `reader-defaults choose`.

O Windows exige confirmação do usuário para tornar o leitor padrão. O CREXE registra um candidato para a extensão, mas não escreve `UserChoice` nem troca silenciosamente a preferência. Depois de conferir o leitor, confirme a escolha em **Configurações → Aplicativos → Aplicativos padrão**. Em sistemas que aceitam o deep link, a engine abre a página do leitor; nos demais, selecione a extensão na página geral. Fechar essa tela não significa que a escolha foi aplicada. [Regras do Windows](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform).

Após a confirmação, dois cliques em **outro arquivo da mesma extensão** abrem esse documento com o leitor mantido, sem chamar a LLM, recompilar ou carregar credenciais. O programa permanece em `reader-apps/`, na raiz da instalação, separado de `readers/` e do cache de geração. Trocar provider/modelo ou limpar somente o cache não deve apagar esse leitor. O runtime/bibliotecas do perfil continuam necessários.

O comando usado pela associação é `crexe open-kept-reader --extension xyz --ui -- "outro documento.xyz"`. Ele é exclusivamente de execução e confere a extensão esperada e a integridade do programa. Leitor ausente, alterado ou incompatível produz erro local, sem regeneração automática. A preparação explícita pode continuar dependendo de modelo e SDK. `--keep-as-default` não aceita `--no-run` nem `--export`, pois o fluxo precisa abrir o leitor preparado.

Para deixar de oferecer um leitor, use `crexe reader-defaults forget --extension xyz`. O comando remove o candidato e o índice da extensão, mas preserva os objetos do programa, fontes, receita e estado no disco; ainda não há coleta automática desses arquivos. A remoção preserva outros aplicativos e não restaura silenciosamente um padrão anterior. Se necessário, escolha outro programa nas Configurações do Windows. `crexe unassociate` remove também os registros de leitores e os dois menus pertencentes à instalação; dados e payloads mantidos não são apagados por essa ação.

Esse MVP é exclusivo do Windows e aceita extensões de documentos. Tipos executáveis, scripts, instaladores, atalhos e formatos CREXE são recusados nessa nova opção. A prévia continua somente leitura e com os mesmos limites de 64 KiB. Somente a extensão segue para a LLM durante a preparação; nome, caminho, conteúdo e outros metadados do documento permanecem fora das requisições.

A implementação e os testes automatizados não substituem a aceitação real do menu, da confirmação de padrão e do duplo clique no Windows. O [plano e critérios de aceite](PLANO_PROGRAMA_PADRAO_V1.md) e o [registro de validação](validation/2026-11-25-kept-reader.md) distinguem essas evidências; a execução gráfica no Windows alvo deve ser registrada antes de anunciar o fluxo como validado para publicação.

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

1. **Validar a entrada e o ambiente.** Resolver o caminho local, rejeitar pasta/dispositivo e casos sem suporte, extrair somente a extensão e fixar o perfil. Verificar orçamento e SDK antes da primeira chamada.
2. **Coordenar a preparação por leitor.** Adquirir um lock local para a combinação de tipo e configuração. Outra abertura compatível aguarda essa preparação.
3. **Planejar uma vez.** Do documento, enviar somente a extensão normalizada, junto com o perfil e o contrato técnico do leitor. A resposta deve ser um JSON com um único campo `prompt`; não é uma resposta `files[]`. Há uma única tentativa de planejamento nessa preparação, sem repetição automática ou fallback.
4. **Persistir a intenção interna.** Validar o prompt e salvar o registro de planejamento e a receita interna de forma atômica, com identidade e checksum. A receita tem nome e perfil controlados pela engine.
5. **Gerar e verificar o programa.** Usar o pipeline existente de `files[]`, projeto da engine, build e autoteste. Acrescentar os checks de leitura descritos abaixo. Falhas de build/teste podem pedir correções dentro do orçamento restante.
6. **Publicar e abrir.** Atualizar o histórico somente após aprovação. Liberar a preparação compartilhada e iniciar o executável com o caminho do documento como um argumento nativo. Cada abertura recebe seu próprio arquivo.

Planejamento, geração e reparos usam a **mesma sessão**, a mesma credencial resolvida e o mesmo limite global de tempo. A chamada única de planejamento tem contagem própria: **não desconta uma tentativa de código** de `execution.max_provider_requests`. O plano reserva até 2.048 tokens de saída, limitado também pelo valor configurado por chamada.

Sem teto explícito, o orçamento derivado de geração continua sendo tentativas × tokens por chamada, acrescido da reserva do planejamento quando necessário. Um `execution.max_output_tokens_total` explícito continua sendo um teto agregado para **plano + código** e pode reduzir o número efetivo de tentativas. A primeira preparação exige saldo para o plano e uma geração completa; não exige configurar duas tentativas de código. Se o teto explícito for insuficiente, nenhuma chamada é feita.

| Situação | Chamadas ao modelo |
|---|---|
| Primeiro leitor, código aprovado sem reparo | 1 planejamento + 1 geração |
| Primeiro leitor, um reparo necessário | 1 planejamento + 1 geração + 1 reparo, se houver orçamento |
| Primeiro leitor, limite padrão e dois reparos necessários | 1 planejamento + até 3 tentativas de código, se os limites de tokens e tempo permitirem |
| Outro documento do mesmo tipo, leitor compatível aprovado | 0 |
| Somente o conteúdo do documento mudou | 0, enquanto o leitor continua válido |
| Falha de planejamento | Sem segunda tentativa automática de planejamento |
| Falha após salvar um plano válido, seguida de nova abertura explícita | Reutiliza o plano e tenta gerar com o orçamento da nova invocação |

O padrão de três tentativas de código permite **um plano + uma geração + até dois reparos**, totalizando no máximo quatro solicitações. `--max-repairs` pode reduzir correções; não amplia o orçamento local. Uma falha HTTP ou de resposta do provider encerra a operação, sem tentativas ocultas. Providers remotos podem cobrar por cada solicitação, inclusive pelas que falham no restante do fluxo.

O cache do leitor considera a extensão normalizada, os parâmetros de geração do provider/modelo, o perfil, OS/arquitetura da engine e a versão do contrato. Extensões sem suporte são rejeitadas; nenhum hash delas é enviado ao modelo. Nome, diretório e conteúdo do documento não fazem parte dessa identidade. Alterar apenas os tetos de solicitações/tokens totais não invalida um leitor aprovado.

Esta revisão da prévia v1 usa a versão 2 do contrato interno do leitor. Leitores criados pelo contrato anterior permanecem separados no cache; a primeira abertura sob o novo contrato prepara um leitor novamente. Reutilização sem chamadas vale para leitores compatíveis com o contrato atual.

O estado interno fica em `readers/<identidade>/`, na raiz gerenciada da instalação; as revisões aprovadas usam o cache/histórico da engine. `CREXE_HOME` permite uma raiz absoluta isolada para testes. Não é necessário criar, editar ou distribuir essas receitas internas para abrir documentos.

Duas aberturas simultâneas do mesmo leitor compartilham uma preparação e, após sucesso, abrem cada documento. Se a preparação falhar ou for cancelada, quem aguardava recebe erro sem iniciar novas chamadas. Uma nova tentativa exige outra invocação explícita. O lock é liberado antes de aguardar o fechamento do aplicativo, permitindo abrir outros documentos enquanto ele permanece aberto.

## Privacidade e limites do leitor v1

Na preparação do leitor, a engine usa **somente a extensão** dos dados do documento selecionado. Não inclui nome, caminho, conteúdo, tamanho, datas ou hash do documento no planejamento, na geração ou nas correções, nem copia o documento para o histórico da receita ou o ZIP dos fontes. As requisições também levam instruções técnicas, o perfil e, nos reparos, código gerado e diagnósticos. O caminho permanece local e só chega ao executável na abertura. Nome e caminho podem aparecer na interface local para identificar o arquivo. Os checks anteriores à publicação usam arquivos de teste sintéticos, sem ler o documento escolhido.

O contrato exige um leitor **somente leitura**: abrir o arquivo selecionado imediatamente, mostrar seu conteúdo real ou um erro legível e preservar o original. Edição mínima, salvar como e interpretação completa de formatos proprietários não fazem parte desta entrega.

- A prévia lê até **65.536 bytes (64 KiB)**, mais um byte para detectar truncamento.
- UTF-8 válido sem bytes NUL é apresentado como texto; outros dados recebem uma prévia hexadecimal identificada. Um corte no meio de uma sequência UTF-8 também pode produzir a prévia hexadecimal.
- Arquivo vazio tem indicação própria; conteúdo maior que o limite deve mostrar aviso de truncamento.
- Formatos conhecidos podem receber apresentação simples, como títulos Markdown ou assunto/corpo EML. Formatos opacos podem oferecer somente texto ou bytes, sem afirmar que foram completamente interpretados.
- O contrato proíbe executar scripts do documento, buscar recursos externos e escrever no original.

Antes de aprovar uma revisão, a engine executa `--crexe-reader-check PATH` em dois textos distintos, arquivo vazio, bytes binários, arquivo acima do limite e UTF-8 inválido. A resposta JSON deve conter `kind`, `content` e `truncated`, com os bytes esperados; a engine também confere que os arquivos de teste foram preservados. A implementação gerada deve usar a mesma leitura limitada na interface e nesse modo sem janela.

A LLM pode incluir um **arquivo dummy inteiramente sintético** nos arquivos da geração normal para ajudar seus autotestes. Isso não exige outra chamada de planejamento nem autoriza copiar, resumir ou transformar o documento escolhido. O dummy não substitui o arquivo real na interface nem os testes independentes obrigatórios da engine.

Esses checks verificam o contrato observável; não são uma sandbox nem comprovam toda a interface ou a segurança do código gerado. O executável roda no host com as permissões do usuário. Diagnósticos de compilação e autoteste voltam ao modelo e podem conter caminhos locais; um programa malicioso ainda poderia acessar arquivos ou rede. A separação do documento no fluxo da engine não equivale a isolamento de um programa nativo. Consulte os [limites de segurança](../SECURITY.md).

Esta v1 demonstra o fluxo de geração de leitores. Uma evolução com execução agêntica e controles de isolamento fica para versões futuras; usar um agente, por si só, não garante isolamento. Também fica para o futuro identificar formatos por uma assinatura lida localmente nos primeiros bytes. Quatro bytes podem ajudar em alguns formatos, mas não identificam todos. Esta versão não faz essa leitura nem envia bytes do documento ao modelo para classificá-lo.

## Verificação automatizada e evidência local

Execute os comandos abaixo na revisão candidata à publicação. Registre separadamente os resultados automatizados, a geração com modelo real e os percursos gráficos em cada sistema.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib
cargo test --locked --test cli
cargo test --locked --test kept_reader
cargo test --locked --test open_generating -- --ignored --nocapture
```

O último comando requer .NET 8 SDK e usa um servidor local controlado, sem LLM real, API paga ou interação com janelas. Cobre cache, extensão desconhecida válida, rejeição de casos sem suporte sem chamadas ao modelo, texto/vazio/binário/truncamento, planejamento separado das tentativas de código, privacidade/exportação, concorrência com sucesso/falha, pré-requisitos inválidos, recusa de `.crexe` nessa ação e rejeição de leitor com conteúdo fixo. Há um caso adicional em Linux para caminho com bytes não UTF-8; o sistema de arquivos do runner macOS rejeita esse nome antes da execução da engine. Os testes unitários do transporte cobrem os três providers, o contrato de planejamento e os tetos de orçamento.

A [configuração da CI](../.github/workflows/ci.yml) inclui a suíte de leitores nos runners; a existência dessa etapa não substitui conferir o resultado de uma execução no commit candidato.

`kept_reader` cobre os limites públicos dos novos comandos, sem instalar ou registrar aplicações reais. Os unitários de persistência verificam payload/índice, integridade e recuperação; os unitários do registro usam chaves de teste isoladas. A confirmação de programa padrão continua sendo um ensaio gráfico distinto.

No Windows, a suíte ignorada `open_generating` inclui `windows_kept_reader_roundtrip`: instala uma engine em uma pasta temporária, gera um leitor console e registra somente um candidato para uma extensão sintética única. O teste não registra os menus gerais, não abre Configurações e não escolhe um padrão. Confere o comando registrado, a ausência de `UserChoice`, a reabertura pela engine instalada sem `CREXE_HOME`, sem configuração/credenciais e após remover o cache temporário, a privacidade e a remoção do candidato. Um guard tenta limpar o registro mesmo em falha. Esse teste é integração real com um candidato temporário; não é a confirmação gráfica do padrão e deve ser executado em ambiente de teste.

## Aceitação manual: CLI, menu e seletor

Use arquivos sintéticos e uma pasta nova. Registre commit/pacote, host, perfil, SDK, provider/modelo, resultado e tempos. Para uma primeira geração verificável, use um tipo ainda não preparado ou uma instalação de teste isolada, sem remover o cache pessoal.

| Percurso | Ação e resultado a conferir |
|---|---|
| CLI | Abrir um arquivo com espaços/acentos e outro iniciado por hífen usando `--`; o conteúdo e o título correspondem ao arquivo selecionado. |
| Reutilização | Abrir outro arquivo com a mesma extensão e conteúdo diferente; o leitor mostra o segundo conteúdo, sem planejamento, geração ou recompilação. |
| Dados alterados | Modificar apenas o conteúdo do segundo documento e reabrir; a prévia muda e o leitor continua vindo do cache. |
| Limites | Abrir vazio, binário e arquivo maior que 64 KiB; conferir indicações de vazio, hexadecimal e truncamento. Comparar os hashes dos originais antes/depois. |
| Windows Explorer | Após instalação/associação, clicar com o botão direito e selecionar **Gerar com CREXE**; no Windows 11, conferir **Mostrar mais opções**. O documento chega ao leitor sem nova seleção e seu aplicativo padrão permanece igual. |
| Windows: manter leitor | Em usuário/VM de teste, usar uma extensão inventada e **Gerar com CREXE e Manter como Programa Padrão**. Conferir o leitor antes de abrir a escolha do sistema. Não afirmar que já é padrão só porque apareceu em Configurações. |
| Windows: confirmar e reabrir | Confirmar o candidato nas Configurações e dar dois cliques em um segundo arquivo da mesma extensão. Registrar conteúdo/título corretos, zero chamadas ao provider e nenhuma compilação. Repetir após trocar modelo e retirar credenciais, e após limpar somente o cache de geração do teste. |
| Windows: cancelar e remover | Cancelar a escolha deixa o padrão anterior intacto. Escolher outro app é respeitado. Testar `reader-defaults forget` e `unassociate` sem remover registros alheios; não tocar os padrões pessoais para executar esse ensaio. |
| Seletor da engine | Abrir `configure`, selecionar o documento e acompanhar até o leitor abrir. A janela da engine continua disponível; depois da preparação, selecionar outro documento. |
| Configuração pendente | Editar uma preferência sem salvar e usar o seletor; a abertura usa a configuração salva e a edição pendente continua na tela. |
| Cancelamento do seletor | Abrir e cancelar a seleção; nenhuma geração é iniciada. |
| Tipo sem suporte | Selecionar arquivo sem extensão, com extensão inválida, receita `.crexe`, pacote `.crexp` ou bundle `.crexb`; conferir o aviso e ausência de chamadas ao modelo. |
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
