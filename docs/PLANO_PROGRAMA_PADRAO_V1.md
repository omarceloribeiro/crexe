# Plano v1 — Gerar com CREXE e manter como programa padrão

[English](en/PLANO_PROGRAMA_PADRAO_V1.md) · [Gerar com CREXE](GERAR_COM_CREXE.md)

**Estado: implementação mínima adicionada; aceitação gráfica completa ainda pendente.** Plano elaborado em 25/11/2026 sobre a base `28e5356` e adotado nesta entrega. Os comandos estão descritos no [guia atualizado](GERAR_COM_CREXE.md); este documento preserva as decisões e os critérios de aceite, sem alegar uma confirmação gráfica já realizada. O [registro de validação](validation/2026-11-25-kept-reader.md) separa os testes aprovados das etapas ainda não comprovadas.

## Decisão recomendada

O MVP cabe antes da publicação, com escopo Windows e uma confirmação final na interface de aplicativos padrão do sistema. A experiência pretendida é: gerar um leitor uma vez, escolhê-lo como padrão para a extensão e abrir outros documentos desse tipo por duplo clique, sem LLM ou compilação.

Adicionar o segundo menu é pequeno; preservar o leitor e conectar a escolha do Windows exigem trabalho adicional. Não basta registrar `open-generating` como comando padrão: esse fluxo pode gerar novamente quando muda a configuração do modelo, o contrato ou o cache.

O critério de entrada na entrega é o percurso completo aprovado no Windows, incluindo confirmação, cancelamento e segundo documento. Uma opção de menu que apenas abre Configurações, sem oferecer um leitor persistente utilizável, não conclui esta feature.

## Limite imposto pelo Windows

O Windows exige que a escolha do aplicativo padrão aconteça na interface do sistema. A documentação atual informa que alterações programáticas sem essa interação não são permitidas e que os dados de preferência são protegidos. O CREXE deve registrar um candidato e encaminhar a escolha ao Windows; não escrever `UserChoice`, calcular hashes privados nem contornar UCPD. [Microsoft: Windows app defaults platform](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform)

`SHOpenWithDialog` não resolve a confirmação: desde Windows 10, suas flags de registro não definem o padrão. A interface **Abrir com → Escolher outro aplicativo → Sempre**, aberta pelo usuário no Explorer, continua sendo um caminho manual válido; ela não é a mesma garantia oferecida pela API. [OPENASINFO](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-openasinfo) · [Explorer e Abrir com](https://support.microsoft.com/en-us/accessibility/windows/use-a-screen-reader-to-explore-and-navigate-file-explorer-in-windows)

Para o MVP, usar `ms-settings:defaultapps`. No Windows 11 com suporte ao deep link, adicionar `?registeredAppUser=<nome registrado codificado>` para abrir a página do leitor. Em versões anteriores, abrir a página geral e mostrar instruções para selecionar a extensão. Não prometer uma quantidade fixa de cliques em todas as versões. [Deep link oficial](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-default-apps-settings) · [Escolher por extensão](https://support.microsoft.com/en-us/windows/apps/change-default-apps-in-windows)

## Implementação entregue

| Componente | Arquivo | Responsabilidade |
|---|---|---|
| CLI e controle do fluxo | `src/lib.rs`, `src/reader.rs` | Ação explícita de manter, launcher separado e comandos de gerenciamento. |
| Revisão aceita | `src/orchestration.rs` | Callback recebe o caminho da revisão aprovada; o fluxo de manter adia a abertura até o payload persistente existir. |
| Armazenamento e launcher | `src/kept_reader.rs` | Verifica revisão, copia fontes/receita/payload, registra candidato via callback e publica índice; abre sem provider ou compilador. |
| Raiz instalada | `src/installation.rs` | Recupera a raiz pelo executável instalado quando o Explorer não herda `CREXE_HOME`; exige versão instalada correspondente na preparação. |
| Menu Windows | `src/installation/windows.rs` | Dois verbos separados, seleção única, caminho e `%1` entre aspas. |
| Candidato e Settings | `src/installation/reader_defaults.rs` | ProgID/capacidades por extensão, propriedade, rollback e URI de Configurações; sem `UserChoice`. |
| Abertura observável | `src/executor.rs`, `src/presentation.rs` | Argumento nativo e observação de janela; o diálogo não é prova de escolha de padrão. |

A passagem entre geração e persistência usa o callback da revisão aceita. O store adquire também o lock do cache e confere que essa revisão continua sendo a aceita antes de copiá-la.

## Experiência do MVP

1. O Explorer mostra **Gerar com CREXE** e **Gerar com CREXE e Manter como Programa Padrão**. A opção atual continua sem alterar o aplicativo padrão. No Windows 11, os verbos clássicos podem estar em **Mostrar mais opções**.
2. A nova opção recebe um arquivo local com extensão suportada pelo fluxo existente. A associação será para todos os arquivos dessa extensão, não apenas para esse documento.
3. A engine gera ou reutiliza o leitor, compila e verifica o contrato com arquivos sintéticos. O documento real só é fornecido ao aplicativo na abertura.
4. A engine preserva a revisão aceita em uma área de programas mantidos, registra o leitor como candidato e abre o documento. Não anunciar sucesso de execução apenas porque houve build.
5. Depois de observar uma janela utilizável do leitor, mostra **Leitor mantido — CREXE** com **Sim/Não**, oferecendo abrir Configurações de aplicativos padrão. Se não confirmar uma janela utilizável, mantém o leitor e informa o comando `reader-defaults choose`. A escolha efetiva acontece no Windows.
6. Após a escolha, duplo clique em outro `.xyz` abre o leitor mantido e mostra esse novo conteúdo. Não chama o modelo, não compila e não precisa das credenciais usadas na geração.
7. Cancelar ou fechar Configurações deixa o leitor disponível como candidato, sem declarar que virou padrão. Se o usuário escolher outro app futuramente, o CREXE respeita essa escolha e não tenta recuperá-la.

No MVP, a nova ação explícita com `--ui` oferece o diálogo quando observa uma janela utilizável do leitor, inclusive ao reutilizar um leitor. A CLI sem `--ui` informa o comando para abrir Configurações. A otimização de dispensar a confirmação quando o leitor parece ser padrão foi removida: consultas de associação do Shell podem retornar um candidato de fallback para uma extensão inédita antes de qualquer escolha do usuário. O duplo clique normal apenas abre o leitor e não mostra esse convite. A engine não anuncia que a escolha foi confirmada por inferência do registro.

## Contratos da implementação mínima

Comandos da implementação mínima:

```text
crexe open-generating --keep-as-default --ui -- "arquivo.xyz"
crexe open-kept-reader --extension xyz --ui -- "outro arquivo.xyz"
crexe reader-defaults list
crexe reader-defaults choose --extension xyz
crexe reader-defaults forget --extension xyz
```

`--keep-as-default` somente no Windows e sem combinação com `--no-run` ou `--export`. A segunda entrada é exclusivamente de execução: não carrega provider, não cria `Session`, não recompila e não recai em `open-generating`. Perfil/contrato antigo, artefato ausente, runtime indisponível ou manifesto inválido resultam em erro local e orientação para uma nova preparação explícita.

Uma nova ação de manter reutiliza um leitor íntegro que já esteja mantido para a extensão, mesmo quando o modelo mudou. Não há atualização automática nem comando de substituição direta nesta versão. Para preparar outro leitor deliberadamente, é necessário esquecer o vínculo atual e solicitar uma nova preparação. Objetos antigos permanecem no disco.

## Persistência e execução

Estrutura implementada sob `installation::reader_root()`:

```text
reader-apps/
  objects/<sha256-do-manifesto>/
    manifest.json
    app/...
    source/...
    recipe/original.crexe
    recipe/resolved.yaml
  extensions/<sha256-da-extensao>.json
  extensions/<sha256-da-extensao>.lock
  state/<sha256-da-extensao>/
```

O manifesto interno registra schema, contrato, extensão, OS/arquitetura, perfil, entrypoint, nomes das variáveis secretas a filtrar e hashes dos arquivos. O hash do manifesto identifica o objeto e incorpora esses hashes. Não contém o nome/caminho do documento escolhido nem valores de credenciais. O runtime necessário é determinado pelo perfil; o payload .NET inclui os arquivos de publicação da aplicação, sem embutir o runtime .NET.

O store valida a revisão aceita, seus fontes e inventário, copia para staging, verifica o executável e o objeto final, registra o candidato e só então publica o índice por substituição atômica. Falhas conservam o índice anterior; se a primeira publicação do índice falhar após o registro, tenta remover o candidato novo. O manifesto público CREXB não foi ampliado; a validação de binário nativo é reutilizada.

A preparação e a remoção usam lock por extensão. Uma operação conflitante recebe erro para tentar depois; a abertura do aplicativo não mantém esse lock durante sua vida. A preparação comum de leitores conserva sua coordenação já existente.

Aberturas consultam o índice da extensão e verificam o objeto, sem recalcular identidade a partir de provider/modelo. O argumento do documento é nativo, nunca texto de shell, e não passa por `ShellExecute` dentro do handler. O diretório de trabalho separado fica em `state/`; a engine filtra credenciais herdadas usando nomes comuns e os nomes salvos no manifesto, sem resolver uma credencial para abrir.

Limpar `readers/` e o cache de geração não remove `reader-apps/`. Os objetos, fontes, receita e diretório de estado sobrevivem inclusive a `forget`; esta versão não inclui coleta automática desses arquivos, evitando apagar dados de processos ainda abertos.

## Registro, remoção e recuperação

Registrar por usuário, sem administrador: ProgID próprio, comando do launcher, nome amigável **CREXE — Leitor .xyz**, `OpenWithProgids`, `Capabilities/FileAssociations` e `HKCU\Software\RegisteredApplications`. Somente extensões com leitor aceito entram nas capacidades. O registro de candidato e sua propriedade devem ser separados do padrão escolhido pelo sistema. [Registro de capacidades](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs) · [Registro de aplicações](https://learn.microsoft.com/en-us/windows/win32/shell/app-registration)

O guia antigo de Default Programs é referência para a estrutura do registro, não para aplicar padrões no Windows atual. A API atual de deep link confirma o uso de `RegisteredApplications` por usuário. A fixture gráfica local já mostrou o candidato com título correto em Configurações; a seleção efetiva e o duplo clique continuam pendentes no registro de validação.

O registro verifica propriedade antes de alterar e captura os valores anteriores em memória para rollback em caso de erro. Os metadados de propriedade ficam no próprio registro; não há `registrations.json`, journal durável ou transação persistida. Um lock em `%LOCALAPPDATA%\CREXE\registry.lock`, independente de `CREXE_HOME`, serializa as alterações de registro entre instalações do mesmo usuário. Operações são idempotentes e enumeram apenas candidatos de propriedade da instalação. Em falha ao abrir Configurações, o leitor permanece preparado e a mensagem explica como concluir manualmente; não gera outra vez.

`unassociate` remove os dois menus e registros de candidatos pertencentes à instalação, preservando os índices e objetos mantidos. `forget` move o índice para uma área recuperável antes de remover o candidato; uma falha no registro tenta restaurar o índice e conserva uma cópia se a restauração falhar. Após sucesso, remove o índice ativo; objetos e estado continuam no disco. Nenhum deles altera `UserChoice`, escolhas de outros aplicativos ou remove chaves de extensão compartilhadas com conteúdo alheio. Se o leitor era o padrão, a pessoa deve escolher outro no Windows; não há restauração automática do aplicativo anterior. [Remoção de ProgIDs](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application)

Para o MVP, preservar as recusas existentes de ausência de extensão e `.crexe`/`.crexp`/`.crexb`. Restringir a nova opção a documentos: recusar explicitamente tipos executáveis, scripts, instaladores e atalhos do sistema, sem mudar a capacidade do comando original de fazer uma prévia de bytes. O conjunto dessa restrição deve ser declarado e testado; não prometer ser leitor padrão de qualquer extensão do Windows.

## Privacidade e escopo

Do arquivo escolhido, somente a extensão vai à LLM. Nenhum nome, caminho, conteúdo, tamanho, data ou hash entra no planejamento, geração ou reparos. O plano base tem uma chamada; as tentativas normais de geração continuam com os limites existentes. Fixtures e dummies são sintéticos.

Depois de mantido, o launcher não deve sequer inicializar a integração de LLM. O documento permanece somente leitura, com os limites de prévia atuais. Essa persistência não acrescenta sandbox, edição, suporte integral a formatos proprietários ou garantia de segurança do código gerado.

Ficam fora do MVP: associações macOS/Linux, menu moderno com extensão COM, captura de tipos sem extensão, classificação por bytes, substituição silenciosa de padrões, publicação na Store, gestão corporativa de políticas e galeria de múltiplos leitores por extensão. O menu atual e a geração comum continuam disponíveis nas plataformas já suportadas.

## Estado das etapas

1. **Persistência, launcher, menu e registro:** implementados, com testes automatizados de armazenamento, comandos e propriedade/rollback do registro.
2. **Geração e reutilização:** o roundtrip Windows com provider controlado aprovou uma chamada de planejamento e uma de geração, leitura de outros documentos sem novas requisições e sobrevivência à remoção do cache/configuração.
3. **Primeira janela e Settings:** a fixture WinForms mostrou o primeiro conteúdo e a tela de Settings com o candidato correto. O binário seguinte mostrou também o leitor e o diálogo completo Sim/Não. Isso não comprova seleção nem cancelamento do padrão.
4. **Aceitação gráfica restante:** confirmar/cancelar escolha, duplo clique em segundo documento, mudança para outro app e remoção, registrando versão/build do Windows. A automação disponível não conseguiu operar a tela de Settings nem os controles do convite com segurança.
5. **Entrega:** a suíte local final aprovou 148 testes e mais 9 testes de geração executados explicitamente. Registrar a última conferência de formatação/clippy após o ajuste do teste de timeout e conferir CI. Não declarar suporte gráfico completo a Windows 10 e 11 com base somente nesses testes locais.

## Validação necessária

| Grupo | Critério |
|---|---|
| Privacidade | Servidor de provider controlado captura todas as requisições; nenhum marcador do documento real aparece. Um plano e tentativas de código normais. |
| Reutilização | Primeiro arquivo gera; segundo e terceiro da mesma extensão abrem seu conteúdo com zero requisições, inclusive com provider/modelo alterado e credenciais indisponíveis. |
| Independência do cache | Remover somente o cache de geração do teste e abrir o leitor mantido. Nenhuma reconstrução ou geração. |
| Dados e argumentos | Espaços, Unicode, hífen inicial e metacaracteres não viram shell; extensão errada, arquivo removido e diretório são tratados sem geração. |
| Persistência | Interromper staging, adulterar manifesto/binário, trocar link ou faltar runtime produz erro local; índice/revisão anterior preservados. |
| Concorrência | Duas preparações coordenadas; dois documentos abertos simultaneamente; atualização não quebra processo já aberto. |
| Registro isolado | Verificar candidatos, comandos, ownership, rollback, instalação repetida e remoção em fixture HKCU privada; nenhum teste unitário toca `Software\Classes` real. |
| Interface Windows | Com extensão inédita, aceitar padrão, cancelar, selecionar outro app, reabrir e remover. Registrar versão/build do OS e evidência do duplo clique real. |
| Regressão | `cargo fmt`, clippy, unitários, CLI, `open_generating` e suites de associação; CI nos três sistemas para evitar regressões de compilação. |
| Publicação | Docs/site PT e EN descrevem a confirmação do Windows e a execução posterior sem LLM; não anunciam associação automática universal. |

Testes de registro simulados e `ShellExecute` não substituem a escolha real na interface do Windows. O release só deve anunciar o novo fluxo depois dessa prova, preservando a diferença entre teste automatizado e aceitação manual.

## Resultado esperado para a demonstração

Duplo clique no formato desconhecido → Windows não encontra aplicativo → novo menu do CREXE → geração e leitura do primeiro documento → confirmação de padrão no Windows → duplo clique em um segundo documento da mesma extensão → leitor abre imediatamente com o conteúdo novo.

É uma adição de escopo moderado e valor claro para o v1. A confirmação do sistema faz parte do fluxo; o trabalho essencial é garantir que o programa mantido continue funcionando depois da sessão de geração.
