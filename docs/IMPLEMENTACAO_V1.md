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

## Incremento DeepSeek — 26/09/2026

Implementado o [plano DeepSeek](PLANO_DEEPSEEK_V1.md): adaptador próprio, preset `deepseek`, modelo inicial `deepseek-flash`, raciocínio opcional e configuração pela mesma tela/cofre. Configurações antigas oferecem o novo preset sem modificar o provider ativo até salvar. A variável `crexe_deepseek_api_key` e o alias convencional são filtrados dos subprocessos mesmo quando o perfil não está salvo. Erros HTTP são classificados sem expor seus corpos; não há fallback ou repetição automática.

Validação local: fmt, clippy com warnings negados e **47 testes** aprovados (34 unitários, 13 CLI), mais o teste explícito do cofre Windows com chave sintética. Os testes controlados incluem payload/modos DeepSeek, catálogo, migração de configuração antiga, vários arquivos, reparo real pelo compilador, cache, troca de credencial, ZIP recompilável e rejeição de saídas inválidas sem publicar revisão ou repetir chamadas.

Ensaio real autorizado: catálogo confirmou `deepseek-flash`; calculadora branca e alteração verde precisaram de **uma chamada cada, sem reparos**, em 16,875 s e 13,968 s incluindo build/autoteste/ZIP. As duas revisões reabriram pelo cache sem IA e expuseram uma janela nativa. Os dois ZIPs passaram por `build.bat`, `test.bat` e `publish.bat`. Um teste .NET independente confirmou seis casos aritméticos, divisão por zero, recuperação após limpar e cor real do objeto Form: branco `(255,255,255)` e verde `(0,120,60)`. Fontes gerados separados em quatro arquivos C#; houve avisos de nulidade, sem erros de build.

Uso informado pela API: 703 tokens de entrada e 7490 de saída. Custo estimado pelas tarifas oficiais consultadas: US$ 0,004581 fora de pico ou US$ 0,009162 em pico; não é conferência do débito da conta. Foram usadas apenas duas das quatro chamadas autorizadas e não houve compra de créditos. Chave resolvida da variável de usuário Windows, sem persistência nos relatórios.

Limitação: a ferramenta de inspeção visual não iniciou nesta sessão. A janela de configuração abriu, mas interação visual de selecionar/salvar não foi repetida neste incremento; o editor e cofre foram verificados por testes. As cores dos apps foram verificadas por código independente, não por captura de tela. Interação desktop em Linux/macOS também permanece pendente. [Relatório detalhado](validation/2026-09-26-deepseek.json).

O commit de implementação `a64ab79` passou a [CI nos três sistemas](https://github.com/omarceloribeiro/crexe/actions/runs/36248882234): fmt, clippy, suíte, fixture com SDK nativo, build release e empacotamento; Windows também passou cofre sintético e inspeção das dependências DLL. Pacotes dos três runners baixados, SHA-256 e hashes internos conferidos. O ZIP Windows local passou instalação/reinstalação e `doctor --provider deepseek` com PATH vazio, preservando o provider local como padrão. [Relatório dos pacotes](validation/2026-09-26-deepseek-packages.json).

## Critérios que continuam abertos

### Incremento de experiência de uso — 25/09/2026

O [plano de UX](PLANO_UX_V1.md) foi implementado na `feature/v1`: configuração nativa por `crexe configure`/sem argumentos, importação explícita de TOML, salvamento atômico com detecção de conflito, cofre do sistema e lançadores nos pacotes. Reinstalações preservam a configuração ativa. O exemplo editado no pacote pode ser importado para revisão e salvo pela tela.

A preparação agora tem uma trava por caminho canônico do `.crexe` e notificação entre processos. Dez chamadas próximas compartilham uma preparação; intenção/opções alteradas pedem concluir/cancelar antes de reabrir. A trava termina na abertura inicial, não no fechamento do app. No Windows, a engine acompanha por até 15 segundos a janela do processo/descendentes, restaura minimização, solicita foco e sinaliza na barra de tarefas se o OS negar ativação.

Validação local deste incremento: 40 testes Rust aprovados (29 unitários e 11 de integração), fmt/clippy sem erros; teste adicional com chave sintética no Windows Credential Manager aprovado e entrada removida. A suíte cobre dez aberturas, alteração durante preparação, recuperação após crash, importação/preservação de regras, conflito externo, falha de escrita/cofre, rotação/remoção de chave, precedência e identidade de cache. Consultas de modelos usam GET em servidores locais controlados, sem geração paga.

`tests/windows_ux.py` abriu a tela nativa e verificou cancelamento sem salvar; gerou/compilou uma fixture .NET Windows Forms com provider controlado, abriu três instâncias com uma geração, incluindo janela inicialmente minimizada e início lento. As três ficaram restauradas. Neste desktop, o Windows recusou foreground nas duas aberturas normais; a abertura inicialmente minimizada recebeu foco. A engine registrou a recusa e aplicou o sinal na barra de tarefas. Isso precisa ser revalidado no Explorer fora do Codex, sem afirmar foco garantido. `tests/windows_progress.py` confirmou cancelamento em 0,171 s e preservação da revisão anterior. Nenhuma API paga foi chamada nesses ensaios.

`tests/windows_configuration.ps1` usou acessibilidade nativa para acionar **Salvar** na janela real, após importar um TOML, e confirmou pela CLI o novo provider/modelo. Passou também no binário release. A geração mantém em memória a chave da primeira requisição para seus reparos, mesmo se a configuração rotacionar/remover essa entrada no cofre durante a chamada.

O commit `2000f4d` passou em Windows, Linux e macOS na [CI 36208001411](https://github.com/omarceloribeiro/crexe/actions/runs/36208001411), incluindo fmt, clippy, suíte Rust, fixture nativa, release e pacotes. Os três artefatos foram baixados e tiveram checksum/hashes internos conferidos; os lançadores de configuração estão presentes. [Registro dos pacotes](validation/2026-09-25-desktop-packages.json). O Windows release passou novamente nos testes de abertura e configuração, sem nova dependência de redistribuível VC++. A cópia de desenvolvimento instalada foi atualizada com os mesmos bytes e preservação das preferências.

A compilação/empacotamento da tela nos três OS e a aceitação visual em macOS/Linux são verificações distintas. A aceitação nesses desktops, em máquina limpa e fora do Codex, continua pendente.

Em andamento: perfis experimentais `cpp-win32`, `objc-cocoa` e `c-gtk`, com ensaio de múltiplos fontes/build/teste/ZIP/publish por SDK nativo na CI. O teste é separado da geração por modelo real e da interação com GUI. A suíte local agora tem 31 testes aprovados (23 unitários e 8 de integração); o teste de SDK nativo é explicitamente ignorado quando suas ferramentas não foram preparadas.

Atualização de 25/09/2026:

- **Ollama real / regra de três:** Markdown gerou dois fontes C#, precisou de uma correção de build e concluiu em 336,772 s. Build final com zero erros e dois avisos, autoteste aprovado. GUI verificada com três resultados conhecidos (incluindo negativo), A/B zero, texto inválido e campo vazio. Reabertura usou cache. ZIP recompilou e executou test/publish; operações CLI build/publish/export passaram sem novas chamadas. Foram duas requisições locais, nenhuma chamada paga. [Relatório](validation/2026-09-25-regra-tres-ollama.json) e [captura](validation/2026-09-25-regra-tres-ollama.png).
- Associação ShellExecute e janela de espera repetidas no binário do commit `7fff749`: WHITE/WHITE/GREEN/GREEN, duas gerações controladas, cancelamento em 0,125 s e preservação do cache. Não substitui clique manual no Explorer.
- No ensaio nativo do commit `7fff749`, Ubuntu/GTK e macOS/Cocoa passaram. O job Windows foi cancelado após prender no novo teste. A fixture C++ foi corrigida para interpretar argumentos com `CommandLineToArgvW`, inclusive aspas do BAT; etapas agora imprimem progresso e têm timeout de 120 s, além do limite do job. A correção passou nos três OS no commit `04be8aa`; a CI do commit `6d9a8e6` também concluiu com sucesso: [execução 36090613840](https://github.com/omarceloribeiro/crexe/actions/runs/36090613840). O ensaio C++ completo levou cerca de 5,7 s no runner após preparo do SDK.
- Leituras do `.crexe` agora são limitadas antes e durante a leitura, com UTF-8 obrigatório. Logs acima do limite também são rejeitados quando o processo termina rapidamente; o teste verifica que nenhum cache é publicado nesse caso.
- Tipos de políticas/estruturas YAML foram reforçados: políticas desconhecidas, string no lugar de booleano e operações test/publish inválidas falham antes da geração. `generate` limita a chamada de acordo com o menor prazo local/receita; allowlist vazia não libera ferramentas.
- O build Windows passou a incluir o runtime C estaticamente. Inspeção do binário confirmou a remoção da dependência `VCRUNTIME140.dll`. Clippy, 31 testes locais e release passaram com essa configuração; instalação/associação e janela também foram repetidas, com cancelamento em 0,109 s. Os scripts de build foram verificados a partir de outra pasta. Snapshots nativos com SHA-256 e origem fixa do fonte estão sendo preparados pela CI, sem publicação de release.
- A primeira CI de empacotamento (`e748386`) passou compilação, testes e perfis nativos nos três sistemas, mas recusou o checkout por uma regra de normalização de dois relatórios históricos. As exceções em `.gitattributes` preservam os bytes originais desses relatórios; o requisito de checkout limpo para empacotar permanece.
- Correção confirmada no checkpoint `845e1bc`: [CI dos três sistemas aprovada](https://github.com/omarceloribeiro/crexe/actions/runs/36096064360), pacotes baixados e hashes conferidos. O executável Windows compilado no GitHub foi instalado em raiz de teste neste host e passou version/doctor/inspect com PATH vazio. [Relatório](validation/2026-09-25-engine-packages.json). O [roteiro manual Windows](VALIDACAO_INSTALACAO_WINDOWS.md) cobre os ensaios pendentes fora do Codex; esses ensaios ainda não foram executados.
- Diagnóstico ampliado: `doctor`/`inspect` distinguem arquitetura da engine/host e informam requisitos do perfil. `doctor --check` verifica o SDK com a política e os limites das consultas locais, sem provider. A suíte local tem agora 32 testes (23 unitários e 9 de integração), incluindo ausência/incompatibilidade de SDK, bloqueio por política, credenciais removidas da consulta e nenhuma chamada ao provider. Detecção do host não muda o target de build; cenários de emulação permanecem pendentes.

1. Robustez e qualidade da geração local de apps desktop, contratos de testes independentes, contabilização de consumo efetivo e opção de correção granular; streaming não é tratado como solução comprovada de sintaxe.
2. Validação dos perfis experimentais C++/Win32, Cocoa/macOS e GTK/Linux, seleção por inventário mais completo, triagem neutra de requisitos e diagnóstico de dependências de terceiros. `ARCH` informa a arquitetura da engine; a observação separada do host ainda precisa de ensaios em emulação/VM.
3. Validação de campos restantes do YAML, dependências além da biblioteca padrão/.NET, retenção/limpeza de cache e relatórios completos de execução.
4. Expansão do contrato de projetos para outros perfis/dependências e distribuição, sem prometer portabilidade implícita de binários.
5. Matriz de apps/arquiteturas, instalação a partir de sessão normal do usuário, clique manual no Explorer, máquina limpa e recuperação após interrupção abrupta. Há virtualização de caminhos AppData no ambiente do Codex; testes feitos por ele não substituem a verificação fora do app.
6. Reconciliação normativa de todas as RFCs, matriz pública de suporte, release com checksums e conteúdo final para `crexe.org`. Sem publicação do site, alteração do domínio ou integração em `main` nesta etapa.

O .env informado pelo autor não estava visível no checkout durante a consulta; a chamada autorizada usou a variável `crexe_openai_api_key_env` já existente no ambiente de usuário Windows. Seu valor não foi registrado em arquivos de configuração, documentação ou logs da engine.
