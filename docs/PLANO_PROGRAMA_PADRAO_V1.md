# Plano v1 — Gerar com CREXE e manter como programa padrão

[English](en/PLANO_PROGRAMA_PADRAO_V1.md) · [Gerar com CREXE](GERAR_COM_CREXE.md)

**Estado: proposta de implementação; a funcionalidade ainda não está implementada.** Análise de 25/11/2026 sobre a revisão `28e5356`. Este documento não altera associações do Windows nem instala leitores.

## Decisão recomendada

O MVP cabe antes da publicação, com escopo Windows e uma confirmação final na interface de aplicativos padrão do sistema. A experiência pretendida é: gerar um leitor uma vez, escolhê-lo como padrão para a extensão e abrir outros documentos desse tipo por duplo clique, sem LLM ou compilação.

Adicionar o segundo menu é pequeno; preservar o leitor e conectar a escolha do Windows exigem trabalho adicional. Não basta registrar `open-generating` como comando padrão: esse fluxo pode gerar novamente quando muda a configuração do modelo, o contrato ou o cache.

O critério de entrada na entrega é o percurso completo aprovado no Windows, incluindo confirmação, cancelamento e segundo documento. Uma opção de menu que apenas abre Configurações, sem oferecer um leitor persistente utilizável, não conclui esta feature.

## Limite imposto pelo Windows

O Windows exige que a escolha do aplicativo padrão aconteça na interface do sistema. A documentação atual informa que alterações programáticas sem essa interação não são permitidas e que os dados de preferência são protegidos. O CREXE deve registrar um candidato e encaminhar a escolha ao Windows; não escrever `UserChoice`, calcular hashes privados nem contornar UCPD. [Microsoft: Windows app defaults platform](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform)

`SHOpenWithDialog` não resolve a confirmação: desde Windows 10, suas flags de registro não definem o padrão. A interface **Abrir com → Escolher outro aplicativo → Sempre**, aberta pelo usuário no Explorer, continua sendo um caminho manual válido; ela não é a mesma garantia oferecida pela API. [OPENASINFO](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-openasinfo) · [Explorer e Abrir com](https://support.microsoft.com/en-us/accessibility/windows/use-a-screen-reader-to-explore-and-navigate-file-explorer-in-windows)

Para o MVP, usar `ms-settings:defaultapps`. No Windows 11 com suporte ao deep link, adicionar `?registeredAppUser=<nome registrado codificado>` para abrir a página do leitor. Em versões anteriores, abrir a página geral e mostrar instruções para selecionar a extensão. Não prometer uma quantidade fixa de cliques em todas as versões. [Deep link oficial](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-default-apps-settings) · [Escolher por extensão](https://support.microsoft.com/en-us/windows/apps/change-default-apps-in-windows)

## O que o código já oferece

Referências de linhas da revisão analisada:

| Base existente | Local | Consequência para o plano |
|---|---|---|
| Menu por usuário, seleção única, comando com `%1` entre aspas e `--` | `src/installation/windows.rs:58`, `:149` | Acrescentar um verbo separado, preservando o menu atual. |
| Instalação em caminho estável | `src/installation.rs:43` | Apontar o registro para a engine instalada, nunca para o ZIP extraído temporariamente. |
| Planejamento só recebe a extensão; um plano salvo | `src/reader.rs:82`, `:154`, `:218` | Reutilizar o contrato de privacidade e a chamada única. |
| Identidade inclui provider/modelo e contrato | `src/reader.rs:196` | O padrão não pode depender dessa identidade ser recalculada em cada abertura. |
| Build e testes antes de aceitar a revisão | `src/orchestration.rs:458`, `:509` | Persistir somente a revisão aceita; falhas conservam o leitor anterior. |
| Callback de preparação anterior ao lançamento | `src/orchestration.rs:557` | Não confundir “preparado” com aplicativo iniciado com sucesso. |
| Argumento do documento separado do comando | `src/executor.rs:116`, `:190` | Reutilizar `OsString` e lançamento direto, sem shell. |
| Revisões com hashes e publicação atômica | `src/cache.rs:55`, `:96` | Reaproveitar validação e cópia limitada em uma área persistente. |
| CREXB já exporta payload aceito | `src/crexb.rs:343` | Reaproveitar helpers quando fizer sentido; seu lançamento público atual não recebe documento (`:975`). |
| Registro isolado para testes | `src/installation/windows.rs:228` | Testes automatizados não precisam mudar os padrões reais do usuário. |

## Experiência do MVP

1. O Explorer mostra **Gerar com CREXE** e **Gerar com CREXE e Manter como Programa Padrão**. A opção atual continua sem alterar o aplicativo padrão. No Windows 11, os verbos clássicos podem estar em **Mostrar mais opções**.
2. A nova opção recebe um arquivo local com extensão suportada pelo fluxo existente. A associação será para todos os arquivos dessa extensão, não apenas para esse documento.
3. A engine gera ou reutiliza o leitor, compila e verifica o contrato com arquivos sintéticos. O documento real só é fornecido ao aplicativo na abertura.
4. A engine preserva a revisão aceita em uma área de programas mantidos, registra o leitor como candidato e abre o documento. Não anunciar sucesso de execução apenas porque houve build.
5. Uma mensagem curta explica: “Leitor preparado para arquivos .xyz. Confirme no Windows para usá-lo como padrão.” O botão **Escolher como padrão no Windows** abre Configurações depois da preparação. Isso também permite que a pessoa veja o leitor antes de escolher.
6. Após a escolha, duplo clique em outro `.xyz` abre o leitor mantido e mostra esse novo conteúdo. Não chama o modelo, não compila e não precisa das credenciais usadas na geração.
7. Cancelar ou fechar Configurações deixa o leitor disponível como candidato, sem declarar que virou padrão. Se o usuário escolher outro app futuramente, o CREXE respeita essa escolha e não tenta recuperá-la.

Se o leitor já está instalado e é o padrão efetivo, a nova ação pode abrir diretamente e informar o estado. Uma consulta ao padrão deve usar a API do Windows, sem inferir sucesso pela mera presença de nossas chaves. [QueryCurrentDefault](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iapplicationassociationregistration-querycurrentdefault)

## Contratos propostos

Nomes abaixo são propostas para a implementação, não comandos disponíveis hoje:

```text
crexe open-generating --keep-as-default --ui -- "arquivo.xyz"
crexe open-kept-reader --ui -- "outro arquivo.xyz"
crexe reader-defaults list
crexe reader-defaults forget --extension xyz
```

`--keep-as-default` somente no Windows e sem combinação com `--no-run`. A segunda entrada é exclusivamente de execução: não carrega provider, não cria `Session`, não recompila e não recai em `open-generating`. Perfil/contrato antigo, artefato ausente, runtime indisponível ou manifesto inválido resultam em erro local e orientação para uma nova preparação explícita.

Separar preparação de abertura em `reader.rs`, retornando um `PreparedReader` com a revisão aceita, perfil, target e contrato. A instalação do leitor recebe essa estrutura, não um caminho de tentativa escolhido por convenção. Preservar os locks e a concorrência do fluxo existente.

## Persistência e execução

Proposta de estrutura, usando a raiz de `installation::root()`:

```text
reader-apps/
  objects/<hash-do-payload>/
    manifest.json
    app/...
    recipe/...
  extensions/<hash-da-extensao>.json
  registrations.json
```

O manifesto tem versão própria, extensão normalizada, perfil, OS/arquitetura, versão do contrato, entrypoint relativo, runtime exigido e hashes dos arquivos. A chave do índice depende da extensão, não do provider atual. Não guardar nome/caminho do documento nem segredos. O payload inclui todos os arquivos de runtime necessários ao perfil, não só um `.exe`.

Copiar a revisão aceita para staging sob a mesma raiz, validar arquivos, hashes, caminhos e ausência de links/reparse points, e publicar o índice atomicamente. Limpeza do cache de geração não deve remover um leitor mantido. Atualização da engine deve preservar esse diretório; atualização do leitor é uma ação explícita e só troca o índice depois de passar nos testes. Arquivos ainda usados por um processo permanecem até uma limpeza segura posterior.

O launcher valida o índice e o payload e executa somente o binário permitido pelo contrato. O novo caminho é passado como um argumento nativo, nunca como template ou texto de `cmd`/PowerShell. Não abrir o documento com `ShellExecute` dentro do handler, pois isso recursaria para a própria associação. A engine mantém a remoção de segredos do ambiente; os nomes de variáveis configuradas podem ser preservados como metadados locais, nunca seus valores. Não depender da resolução de uma credencial para abrir o leitor.

Reusar o tratamento de payload/runtime do CREXB é possível, mas não ampliar o manifesto público nem permitir argumentos arbitrários vindos de um bundle só para esta feature. O parâmetro do documento deve continuar vindo exclusivamente da invocação local do launcher. Aberturas simultâneas usam o mesmo payload imutável sem manter um lock exclusivo durante toda a vida dos aplicativos.

## Registro, remoção e recuperação

Registrar por usuário, sem administrador: ProgID próprio, comando do launcher, nome amigável **CREXE — Leitor .xyz**, `OpenWithProgids`, `Capabilities/FileAssociations` e `HKCU\Software\RegisteredApplications`. Somente extensões com leitor aceito entram nas capacidades. O registro de candidato e sua propriedade devem ser separados do padrão escolhido pelo sistema. [Registro de capacidades](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs) · [Registro de aplicações](https://learn.microsoft.com/en-us/windows/win32/shell/app-registration)

O guia antigo de Default Programs é referência para a estrutura do registro, não para aplicar padrões no Windows atual. A API atual de deep link confirma o uso de `RegisteredApplications` por usuário. A primeira prova técnica da implementação deve confirmar que uma extensão inédita aparece em Configurações com o leitor correto.

Prever um journal local das entradas criadas, checagem de proprietário antes de alterar e operações idempotentes. Em falha de registro, não trocar um leitor anterior nem deixar chaves apontando para payload inexistente. Em falha ao abrir Configurações, manter a preparação e explicar como concluir manualmente; não gerar outra vez.

`unassociate` deve remover também os dois menus e os registros de leitores pertencentes à instalação. Não apagar chaves de extensão compartilhadas, registros de outros aplicativos ou `UserChoice`. Os payloads mantidos podem permanecer até `forget`, que remove apenas aquele leitor e suas entradas. Se ele ainda for o padrão, orientar a pessoa a escolher outro no Windows; não prometer restauração automática do aplicativo anterior. [Remoção de ProgIDs](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application)

Para o MVP, preservar as recusas existentes de ausência de extensão e `.crexe`/`.crexp`/`.crexb`. Restringir a nova opção a documentos: recusar explicitamente tipos executáveis, scripts, instaladores e atalhos do sistema, sem mudar a capacidade do comando original de fazer uma prévia de bytes. O conjunto dessa restrição deve ser declarado e testado; não prometer ser leitor padrão de qualquer extensão do Windows.

## Privacidade e escopo

Do arquivo escolhido, somente a extensão vai à LLM. Nenhum nome, caminho, conteúdo, tamanho, data ou hash entra no planejamento, geração ou reparos. O plano base tem uma chamada; as tentativas normais de geração continuam com os limites existentes. Fixtures e dummies são sintéticos.

Depois de mantido, o launcher não deve sequer inicializar a integração de LLM. O documento permanece somente leitura, com os limites de prévia atuais. Essa persistência não acrescenta sandbox, edição, suporte integral a formatos proprietários ou garantia de segurança do código gerado.

Ficam fora do MVP: associações macOS/Linux, menu moderno com extensão COM, captura de tipos sem extensão, classificação por bytes, substituição silenciosa de padrões, publicação na Store, gestão corporativa de políticas e galeria de múltiplos leitores por extensão. O menu atual e a geração comum continuam disponíveis nas plataformas já suportadas.

## Etapas de implementação

1. **Prova do registro no Windows.** Em usuário/VM de teste, registrar um leitor fixture para uma extensão inventada, abrir Configurações, selecionar o padrão e validar o duplo clique. Conferir Windows 11 e fallback Windows 10 antes de prometer suporte aos dois.
2. **Persistência e launcher.** Implementar o manifesto, promoção atômica e execução local independente de provider/cache. Essa é a parte central do MVP.
3. **Integração com geração.** Extrair o resultado preparado sem alterar orçamento, privacidade, validação e concorrência existentes. Só promover após aceitação; preservar revisão anterior nas falhas.
4. **Menu e escolha do sistema.** Adicionar o verbo, registro de capacidades, nome amigável, abertura de Settings e mensagens distintas para preparado, escolha pendente e padrão confirmado. Não usar o callback genérico de processo terminado como prova de janela aberta.
5. **Remoção, docs e entrega.** Cobrir `unassociate`/`forget`, atualizar guias e site PT/EN, executar testes e publicar a mudança em commit próprio, separado do contador.

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
