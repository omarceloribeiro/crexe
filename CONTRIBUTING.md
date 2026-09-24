# Contribuir

A implementação ativa é o pacote Cargo na raiz, derivado de `baseline/pre-v1`. Experimentos estão em `docs/archive`; arquivos contextuais da outra máquina permanecem privados e ignorados.

Use `rust-toolchain.toml`. Antes de propor alterações:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Testes Rust usam providers HTTP locais com respostas controladas e compilação nativa, sem saldo OpenAI ou Ollama instalado. Testes reais são opt-in e registram provider, parâmetros, chamadas, builds/testes e limitações. Não versionar chaves, `.env`, configurações privadas, caches ou binários de apps.

Especializações usam `#[cfg]` e dependências Cargo por target. Não remover módulos para compilar em outra plataforma. A v1 usa pasta temporária no host, sem Docker/WSL; não utilizar Windows Sandbox em código, desenvolvimento ou testes.

`tests/windows_association.py` e `tests/windows_progress.py` são verificações explícitas no host. A auditoria histórica `tests/baseline-audit` é exclusiva do baseline e do ambiente indicado em seu README; não executar diretamente no host.

Commits intermediários em `feature/v1` servem de recuperação. A integração em `main` será squash após os critérios de entrega; não mover a tag de baseline.
