# CREXB — Creative Executable Bundle

**Gerado uma vez. Aberto quantas vezes quiser.** CREXB reúne a receita e uma versão aceita do aplicativo em um ZIP controlado. Em um computador compatível, a engine valida o bundle, prepara o cache e abre o programa sem chamar IA nem recompilar.

**Status: prévia mínima na branch `feature/v1`.** Este guia descreve o subconjunto entregue; o [handoff completo original](../formatos/originais/CREXB-HANDOFF-COMPLETO.md) permanece disponível em português, sem alterações. A proposta completa inclui funcionalidades futuras e discussões de licença anteriores. A [política vigente](../LICENCIAMENTO_E_ECOSSISTEMA.md) usa MIT para a engine e MIT-0 para os materiais de saída designados.

## Compartilhar a versão aceita

Uma receita permite gerar de novo. Um bundle também preserva um resultado específico. Isso permite entregar a mesma versão para uma turma, uma equipe ou outro computador compatível sem exigir nova geração.

O exportador usa a última revisão **aceita**, com o snapshot da receita original registrado pela engine. Uma tentativa que falhou não substitui esse resultado. Revisões antigas, criadas antes do registro desse snapshot, precisam de uma regeneração explícita antes da primeira exportação como CREXB. O exportador não presume que a receita atual em disco corresponde ao binário do cache.

Para CREXP, editar o pacote muda o hash da materialização, mas mantém a linhagem pelo caminho original do arquivo. Assim, se uma nova tentativa falhar, a exportação ainda usa a última revisão aceita e a receita exata registrada naquela geração, não o conteúdo novo que falhou.

Aceito significa que a revisão passou pelos critérios implementados do pipeline. Não significa auditoria completa, certificação de segurança ou prova de que toda a intenção foi cumprida. A evidência deve declarar os testes realmente executados, sem inventar validações de interface ou aceitação.

O manifesto da prévia registra `applicationStart: not-verified` e `acceptance: not-required`; build e autotestes não equivalem a aprovação humana do programa. O bundle inclui a receita original, uma representação resolvida, fontes, referência aos autotestes nos fontes, evidências e payload. `tests/README.md` identifica os testes `--crexe-self-test` dentro de `source/`, sem inventar uma suíte adicional. Abrir um bundle não repete esses testes.

## Escopo da prévia

O bundle mínimo tem uma plataforma e arquitetura por arquivo, manifesto, receita preservada, payload, metadados de lançamento e hashes. A prévia atende os perfis `cpp-win32`, `objc-cocoa`, `c-gtk`, `dotnet-console` e `dotnet-winforms`, com apphost nos perfis .NET 8. OpenGL e comandos customizados de receitas YAML ficam fora deste primeiro subconjunto.

O computador de destino precisa da engine e das dependências de runtime do aplicativo. Um app .NET dependente do framework exige o runtime .NET compatível; um programa nativo pode exigir bibliotecas do sistema. Abrir o bundle não exige SDK de compilação nem provider de IA. Isso não torna o payload universal nem autossuficiente automaticamente.

## Exportar, verificar e abrir

Depois de gerar e aceitar uma revisão, na raiz do checkout:

```powershell
.\target\release\crexe.exe bundle create calculadora.crexe --target current --output calculadora.crexb
.\target\release\crexe.exe bundle inspect calculadora.crexb
.\target\release\crexe.exe bundle verify calculadora.crexb
.\target\release\crexe.exe bundle run calculadora.crexb
```

Em macOS/Linux, use `./target/release/crexe`. Em uma distribuição da engine, use o executável da pasta extraída. `bundle create` aceita `--cache-dir` para localizar a revisão; a abertura usa a raiz padrão da engine, respeitando `CREXE_HOME`. O comando de criação usa uma revisão já aceita; não faz geração silenciosa para preencher um cache ausente.

O fluxo de execução também reconhece `.crexb` e bundles renomeados para `.crexe` pelo conteúdo. O manifesto em `META-INF/crexe-bundle.json` diferencia CREXB de um pacote SDD CREXP. Um ZIP sem marcador reconhecido ou com marcadores conflitantes é rejeitado.

Um arquivo com extensão `.crexb` que não contém o manifesto de bundle é rejeitado; ele nunca inicia geração como fallback. Na abertura, a receita incluída permanece inerte: seus bytes e hash são conferidos, sem consolidar novamente os documentos SDD.

No Windows, `install` e `associate` registram também `.crexp` e `.crexb` para abertura por duplo clique. O editor textual não edita esses ZIPs, e não há associação global de arquivos `.zip`. Para apenas verificar um bundle, use `bundle verify` ou o fluxo normal de execução com `--no-run`; esse modo não hidrata nem executa o payload.

## Cache e dados do aplicativo

A hidratação valida os arquivos e prepara o payload em `bundles/objects/sha256/<hash-do-ZIP>` sob o cache configurado. Um cache válido pode ser reutilizado, sem nova geração. Dados mutáveis ficam separados, em `apps/<hash-do-id>/{data,config,state}` sob a raiz de instalação da engine; os caminhos concretos dependem da configuração e da plataforma. O processo recebe os diretórios por variáveis `CREXE_*` e usa a pasta de dados como diretório de trabalho.

Nesta prévia, a identidade do bundle deriva do caminho original do arquivo de receita usado na produção, não de `package.id`. Mover ou renomear essa receita antes de produzir outro bundle pode criar outra identidade de dados. Identidade pública estável e migração de dados pertencem à evolução do formato.

O aplicativo não deve tratar a pasta do binário como armazenamento de dados. Assets devem ser localizados a partir da base do executável, não presumindo que o diretório de trabalho é a pasta do payload. Essa separação evita confundir arquivos distribuídos com dados produzidos durante o uso.

Cache de bundle incompleto, alterado ou com arquivos extras é bloqueado. A prévia não o apaga silenciosamente: a remoção explícita apenas daquele objeto de cache permite uma nova hidratação, preservando os dados separados do aplicativo. O exportador também recusa revisões com arquivos não listados no inventário.

O lock do bundle permanece durante a execução do aplicativo. Uma segunda abertura simultânea do mesmo bundle é recusada até o primeiro processo terminar; gerenciamento de múltiplas instâncias fica para uma evolução futura.

## Compatibilidade e regeneração explícita

Um payload Windows não é executado como se fosse Linux. A primeira versão exige compatibilidade do target; não oferece emulação nem instala runtimes automaticamente. Incompatibilidade bloqueia a abertura e informa o motivo.

Para voltar à receita incluída, use a extração explícita:

```powershell
.\target\release\crexe.exe bundle extract-recipe calculadora.crexb --output recuperada.crexe
.\target\release\crexe.exe inspect recuperada.crexe
.\target\release\crexe.exe exec recuperada.crexe
```

Escolha a extensão de saída conforme o tipo da receita informado pela inspeção (`.crexe` ou `.crexp`). A última linha inicia uma nova geração normal: pode chamar o modelo, usar rede e requer os SDKs. O bundle original não é alterado. A interface guiada de regeneração, múltiplos targets e seleção avançada de runtimes pertencem ao roadmap.

## Integridade e confiança

Hashes detectam alterações acidentais e inconsistências do conteúdo declarado. **Não autenticam o autor:** alguém que altera todos os arquivos também pode recalcular hashes. A prévia não verifica assinaturas e recusa bundles que exigem uma assinatura não suportada.

O subconjunto inicial exige caminhos internos ASCII portáveis, até 1.024 entradas, arquivos individuais de até 128 MiB e ZIP de até 256 MiB. O launch admite o entrypoint do perfil, sem argumentos customizados. Bundles que contêm assinaturas também são recusados nesta prévia, em vez de serem abertos com uma falsa indicação de verificação.

CREXB contém software executável. A presença da receita não o torna seguro, e a engine atual não oferece sandbox do aplicativo. Execute apenas conteúdo em que confia. A validação do ZIP e dos caminhos não substitui os [limites de segurança](../../SECURITY.md).

## Publicação e privacidade

Ao compartilhar um bundle, a receita também é compartilhada. Revise os documentos e arquivos incluídos antes de distribuí-lo. O scanner de segredos, a redação automática de logs, SBOM, assinatura e verificação de publisher são propostas futuras.

Você escolhe a licença do aplicativo, inclusive proprietária, respeitando as licenças de receitas, dependências e componentes incluídos. A MIT da engine e a MIT-0 dos materiais de saída não substituem as condições desses terceiros.

## Downloads da proposta completa

- [Handoff CREXB completo — Markdown original em português](../formatos/originais/CREXB-HANDOFF-COMPLETO.md).
- [JSON Schema original do manifesto v1](../formatos/originais/crexb-manifest-v1.schema.json).
- [Manifesto original de exemplo](../formatos/originais/crexb-manifest.example.json).
- [Hashes SHA-256 dos quatro originais](../formatos/originais/checksums.sha256).

**O manifesto de exemplo usa placeholders e referências ilustrativas. Ele não é um aplicativo CREXB executável e não contém um build validado.** O schema é um draft da proposta completa, não uma promessa de que todas as combinações por ele aceitas funcionam na prévia. O [roadmap](roadmap-formatos.md) identifica as etapas restantes. O seletor de idioma do site oferece o guia mantido integralmente em português e inglês; o handoff completo permanece no original em português.
