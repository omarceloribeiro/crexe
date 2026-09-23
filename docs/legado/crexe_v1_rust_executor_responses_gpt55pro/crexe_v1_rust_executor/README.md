# CREXE v1 — Rust Executor

Esta versão substitui o runtime Python por um executor em **Rust**.

Ela mantém a ideia da melhor versão Python:

- `.crexe` em YAML
- `prompt_core` isolado
- selector por OS
- LLM online OpenAI-compatible
- cache por fingerprint
- geração de arquivos
- build/run por target
- command allowlist

## Estratégia do exemplo

O exemplo `examples/calculator_native.crexe` gera uma calculadora desktop.

No Windows, ela gera uma janela nativa em C++ usando **Win32 API**, compilável com **g++/MinGW-w64**, sem Visual Studio, sem `cl.exe` e sem CMake.

Observação: “Windows Forms C++” no sentido estrito normalmente significa C++/CLI/.NET e geralmente exige Visual Studio. Para manter portabilidade, este exemplo usa uma janela nativa estilo formulário com Win32 API.

## Requisitos

### Executor

- Rust/Cargo instalado

Verificar:

```bash
cargo --version
```

### Windows target

- `g++` no PATH via MinGW-w64/MSYS2

Verificar:

```cmd
g++ --version
```

### macOS target

- Xcode Command Line Tools

```bash
xcode-select --install
```

### Linux target

- GCC, pkg-config e GTK3 dev

Ubuntu/Debian:

```bash
sudo apt install build-essential pkg-config libgtk-3-dev
```

## API key

### Windows PowerShell

```powershell
setx OPENAI_API_KEY "SUA_CHAVE"
```

Depois abra um novo terminal.

### macOS/Linux

```bash
export OPENAI_API_KEY="SUA_CHAVE"
```

## Rodar pelo Cargo

```bash
cargo run --release -- exec examples/calculator_native.crexe
```

Com overrides:

```bash
cargo run --release -- exec examples/calculator_native.crexe --set AppName=MinhaCalc --set WindowTitle="Minha Calculadora"
```

## Gerar executável do executor

```bash
cargo build --release
```

Depois:

### Windows

```cmd
target\release\crexe.exe exec examples\calculator_native.crexe
```

### macOS/Linux

```bash
./target/release/crexe exec examples/calculator_native.crexe
```

## Cache

O cache fica em:

- Windows: `%LOCALAPPDATA%/CREXE/cache`
- macOS: `~/Library/Caches/CREXE`
- Linux: `~/.cache/crexe`

Se o `.crexe`, inputs, versão do runtime e configuração do modelo não mudarem, a próxima execução pula geração/build e executa direto.


## Patch CMD Fixed

Esta versão mantém CMD e corrige:

- retry automático `max_tokens` -> `max_completion_tokens`;
- retry sem `temperature` se o modelo rejeitar esse parâmetro;
- `--rebuild`;
- fallback de execução: se `build\{{AppName}}.exe` não existir, procura um único `.exe` dentro da pasta `build`.

Comandos:

```cmd
build-executor.cmd
run-example.cmd
run-example-rebuild.cmd
```

Ou direto:

```cmd
target\release\crexe.exe exec examples\calculator_native.crexe --rebuild
```


## Drag-and-drop / Abrir com no Windows

Esta versão aceita dois formatos:

```cmd
target\release\crexe.exe exec examples\calculator_native.crexe
```

e também:

```cmd
target\release\crexe.exe examples\calculator_native.crexe
```

O segundo formato é o usado pelo Windows quando você:

- arrasta um arquivo `.crexe` em cima do `crexe.exe`;
- configura `.crexe` para abrir com `crexe.exe`;
- dá dois cliques em um `.crexe` associado ao runtime.

### Testar sem associar extensão

```cmd
test-drag-openwith.cmd
```

### Associar `.crexe` ao runtime no usuário atual

```cmd
associate-crexe.cmd
```

Depois disso, qualquer `.crexe` deve abrir com o runtime, independentemente da pasta onde o arquivo `.crexe` estiver.

### Remover associação

```cmd
unassociate-crexe.cmd
```


## Fix: running generated executable from cache

This version keeps the command allowlist, but explicitly allows `run` commands that point to generated executables inside the CREXE workspace/cache.

This fixes:

```cmd
Cache hit: skipping generate/build
Error: Command not allowed by policy: build\CrexeCalculator.exe
```

That error is not caused by CMD or PowerShell. It is the CREXE runtime policy blocking the generated executable before launching it.


## Patch policyfix2

Corrige erro de compilação:

```text
error[E0425]: cannot find function `is_inside` in this scope
```

A função `is_inside(root, child)` agora está presente no executor.


## Patch policyfix3

Corrige erro no Windows ao abrir executável gerado no cache:

```text
Error: Failed to start command: build\CrexeCalculator.exe
Caused by:
    O sistema não pode encontrar o caminho especificado. (os error 3)
```

Causa:
`std::process::Command` no Windows não deve depender de `current_dir` para resolver o caminho relativo do programa executável.

Correção:
Antes de executar, o runtime transforma:

```text
build\CrexeCalculator.exe
```

em caminho absoluto:

```text
C:\Users\...\AppData\Local\CREXE\cache\...\build\CrexeCalculator.exe
```


## Patch minimal: absolute generated executable path

Esta versão parte da `policyfix3` e aplica somente uma correção mínima:

Quando o comando de execução aponta para um executável gerado dentro do cache, por exemplo:

```cmd
build\CrexeCalculator.exe
```

o runtime converte para caminho absoluto antes de chamar `Command::new(...)`:

```cmd
C:\Users\...\AppData\Local\CREXE\cache\...\build\CrexeCalculator.exe
```

Isso corrige o erro:

```text
Failed to start command: build\CrexeCalculator.exe
O sistema não pode encontrar o caminho especificado. (os error 3)
```


## Responses API support

Esta versão adiciona suporte a:

```yaml
generator:
  provider: "openai"
  transport: "responses"
  model: "gpt-5.5-pro"
  maxOutputTokens: 6000
```

O exemplo principal `examples\calculator_native.crexe` agora usa `gpt-5.5-pro` com `/v1/responses`.

Para rodar o exemplo Responses:

```cmd
run-example-responses.cmd
```

Para rodar o exemplo legado Chat Completions:

```cmd
run-example-chat.cmd
```

Transportes suportados:

- `transport: "responses"` -> POST `/v1/responses`, usa `max_output_tokens`.
- `transport: "chat_completions"` -> POST `/v1/chat/completions`, mantém compatibilidade com `max_tokens` / `max_completion_tokens`.
