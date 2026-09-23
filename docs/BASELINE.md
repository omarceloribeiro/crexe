# Baseline anterior à implementação da v1

Data: 2026-09-23. Branch de trabalho: `feature/v1`. Referência fixa: tag anotada `baseline/pre-v1`. Essa tag identifica o commit de preservação do projeto e do plano, sem representar uma release pronta para distribuição.

## Conteúdo preservado no Git

- Pacote original em `src/`, incluindo código, exemplo YAML, scripts e README, sem alterações na implementação.
- Variante experimental arquivada em `docs/legado/`, apenas como histórico.
- RFCs, handoffs, artigos, manifesto, estratégia e demais documentos recebidos, inclusive duplicatas ainda não reorganizadas.
- [Plano da v1](PLANO_V1.md), com as decisões tomadas até este ponto.
- Relatórios e logs de validação em `docs/validation/`, incluindo a reprodução do pacote SMTP de referência.
- Auditoria reproduzível em `tests/baseline-audit/`, com o lockfile usado na validação Linux.
- Licença original e regras de exclusão/proteção do snapshot. `.gitattributes` preserva os bytes e finais de linha dos arquivos importados; `LICENSE` mantém a normalização do commit inicial. Eventual normalização dos demais arquivos fica para uma alteração própria.

O baseline de implementação é **exclusivamente `src/`**. A variante experimental, os relatos de uso e os aplicativos de referência não substituem esse código.

## Cópia local completa e exclusões do remoto

Antes de preparar o commit, foi criado e verificado um ZIP local com todos os 738 arquivos e 246 diretórios do estado recebido, exceto os metadados `.git/`. Um manifesto local registra tamanho e SHA-256 dos arquivos. O snapshot fica em `%LOCALAPPDATA%\CREXE\backups\`, fora do repositório, e não é enviado ao GitHub.

A pasta `docs/legado/TestesDeOutraMaquina/` permanece intacta no disco e nesse backup. Seus 677 arquivos incluem programas compilados, ZIPs, caches e perfis de aplicações/WebView2 com arquivos de cookies/login. Ela está excluída do Git, conforme seu papel de contexto local não canônico. Os fontes, documentos e evidências do projeto foram preservados no commit; não houve remoção dos arquivos locais.

## Evidências e limites

O autor validou **todo o ciclo na mesma máquina original, diferente da máquina desta sessão**: compilou a engine, associou a extensão, abriu o YAML por duplo clique, gerou a calculadora, alterou a intenção para fundo verde, gerou novamente e reabriu a versão verde pelo cache. Durante o processamento, o terminal da engine ficava visível; conforme o relato, fechava quando a calculadora abria. O cache foi inspecionado, mas seu caminho não foi registrado. O [relatório do baseline](validation/2026-09-23-baseline.md) descreve esse percurso e distingue relato, artefatos históricos e testes locais.

Na sessão atual, a auditoria Linux resultou em 13 verificações aprovadas e 14 requisitos não atendidos. O baseline ainda não é uma release validada para o público. Não houve alteração de código para corrigir esses resultados antes deste commit. Os demais limites de OpenAI, Ollama, associação Windows e UI estão nos relatórios e no plano.

## Comparar durante a implementação

Comparar commits posteriores com esta referência fixa:

```sh
git diff --stat baseline/pre-v1..HEAD
git diff baseline/pre-v1..HEAD -- src docs tests
```

Incluir também alterações locais de arquivos rastreados na comparação:

```sh
git diff baseline/pre-v1 -- .
git status --short
```

`git status` também mostra arquivos novos ainda não rastreados, que não aparecem no diff comum. A tag deve permanecer apontando para o baseline; não movê-la durante a implementação. `feature/v1` pode receber commits incrementais e continuar publicada no remoto. Ao concluir e validar o plano, consolidar a entrega em um único commit na `main`, conforme a intenção do autor.
