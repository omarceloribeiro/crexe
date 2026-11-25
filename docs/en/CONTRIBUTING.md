# Contributing

The active implementation is the Cargo package at the repository root, derived from `baseline/pre-v1`. Experiments are in `docs/archive`; contextual files from the other machine remain private and ignored.

The engine is GNU GPL v3. CREXE-provided materials incorporated into output and official recipes also have the MIT-0 grant scoped in [LICENSE-GENERATED](../../LICENSE-GENERATED). Contributions to those materials must preserve that freedom so users can choose their application's license. Do not incorporate third-party content with incompatible terms into those templates or examples; dependencies with their own licenses must remain separately identified.

Use `rust-toolchain.toml`. Before proposing changes:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Rust tests use local HTTP providers with controlled responses and native compilation, without OpenAI credit or an installed Ollama service. Real tests are opt-in and record the provider, parameters, calls, builds/tests, and limitations. Do not commit keys, `.env`, private configurations, caches, or application binaries.

Platform specializations use `#[cfg]` and target-specific Cargo dependencies. Do not remove modules to compile on another platform. V1 uses a temporary directory on the host, without Docker/WSL; do not use Windows Sandbox in code, development, or tests.

`tests/windows_association.py` and `tests/windows_progress.py` are explicit checks on the host. The historical audit in `tests/baseline-audit` is exclusive to the baseline and the environment specified in its README; do not run it directly on the host.

Intermediate commits on `feature/v1` provide recovery points. Integration into `main` will be squashed after the delivery criteria are met; do not move the baseline tag.
