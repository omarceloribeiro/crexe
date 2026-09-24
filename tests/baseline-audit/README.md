# Auditoria do baseline

Esta bateria caracteriza o pacote Rust original da tag `baseline/pre-v1`, cujo diretório era `src/`. O pacote ativo foi reorganizado; a suíte atual roda com `cargo test --locked` na raiz. Esta auditoria histórica não altera a engine, não usa OpenAI, não faz downloads de modelos e não lê segredos locais. Um servidor HTTP falso devolve fontes C mínimos, que são compilados e executados dentro de um contêiner descartável.

Os cenários de contenção usam exclusivamente arquivos sentinela dentro do contêiner. **Não execute `audit.py` diretamente no host.** Há testes que deliberadamente verificam escrita e execução fora do workspace da engine, mas dentro do espaço descartável do contêiner.

O resultado inicial foi **13 PASS / 14 FAIL em 27 verificações**. FAIL significa requisito de robustez ou de distribuição ainda não atendido; não significa que o exemplo funcional deixou de funcionar. Alguns cenários verificam capacidades ainda ausentes, como `--version` e preflight. O processo retorna código 1 enquanto houver falhas.

## Executar

Requer Docker com contêineres Linux. Em PowerShell, a partir da raiz atual do repositório:

```powershell
$repoRoot = (Get-Location).Path
$reportDir = Join-Path ([IO.Path]::GetTempPath()) ('crexe-audit-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $reportDir | Out-Null
git archive --format=zip --output="$reportDir\baseline.zip" baseline/pre-v1 src
Expand-Archive -LiteralPath "$reportDir\baseline.zip" -DestinationPath "$reportDir\snapshot"
docker run --rm `
  --mount "type=bind,source=$reportDir\snapshot\src,target=/baseline,readonly" `
  --mount "type=bind,source=$repoRoot\tests\baseline-audit,target=/audit,readonly" `
  --mount "type=bind,source=$reportDir,target=/qa" `
  rust:1-slim-bookworm@sha256:ff521445a372125ed4f76e1453a1f8098f2d05332d1601d30db1c1f62757e730 `
  sh /audit/run.sh
Get-Content -LiteralPath (Join-Path $reportDir 'results.json')
```

O preparo baixa Python e dependências Cargo; a execução dos cenários usa somente o servidor HTTP em loopback. Nenhuma chave é passada ao contêiner. O lockfile desta pasta registra a resolução de dependências usada na auditoria, sem introduzir ainda um lockfile no pacote original.

Os fontes e exemplos originais ficam montados como somente leitura. Logs e resultado JSON são gravados apenas em `$reportDir`. O teste cria seus arquivos temporários em `/work`, descartado com o contêiner.

## Limites

- Valida Linux x86_64 com programa C mínimo e provider simulado.
- Não comprova GUI Windows, registro de extensão, duplo clique, macOS, ARM64 ou um modelo real.
- A compatibilidade do YAML original é verificada até o erro de credencial ausente; não há chamada remota para esse exemplo.
- A resolução de dependências Rust está fixada; o repositório Debian usado para instalar Python continua sujeito a atualizações.
- Depois da reorganização da engine, adaptar os caminhos e migrar os cenários relevantes para a suíte oficial.
