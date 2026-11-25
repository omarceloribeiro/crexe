# Contribuir

A implementação ativa é o pacote Cargo na raiz, derivado de `baseline/pre-v1`. Experimentos estão em `docs/archive`; arquivos contextuais da outra máquina permanecem privados e ignorados.

A engine é [MIT](LICENSE). Contribuições à engine seguem essa licença, com os avisos de copyright e licença aplicáveis. Materiais fornecidos pelo CREXE que são incorporados à saída e receitas oficiais têm também a concessão MIT-0 delimitada em [LICENSE-GENERATED](LICENSE-GENERATED); contribuições a esses materiais devem manter explicitamente essa concessão, para que o usuário escolha a licença do aplicativo sem obrigação de atribuição ao CREXE.

Envie somente contribuições que você tenha direito de licenciar nesses termos. A licença não transfere automaticamente a titularidade ao mantenedor, e este projeto não exige cessão de direitos ou um CLA separado. Identifique material e dependências de terceiros, preservando seus avisos; não incorporar conteúdo com condições incompatíveis aos materiais designados MIT-0.

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
