# CREXE v1 Schema Example

This is a representative `.crexe` file structure.

```yaml
version: 1.0

meta:
  id: "crexe.examples.calculator.native"
  name: "Native Calculator"
  description: "Generates a native desktop calculator per OS using an online LLM."
  license: "UNLICENSED"

inputs:
  AppName:
    type: string
    default: "CrexeCalculator"
  AppVersion:
    type: string
    default: "1.0.0"
  WindowTitle:
    type: string
    default: "CREXE Calculator"
  ForceTarget:
    type: string
    default: ""

workspace:
  mode: cache
  cache:
    strategy: "sha256(spec+inputs+runtime+model)"
    root:
      windows: "%LOCALAPPDATA%/CREXE/cache"
      macos: "~/Library/Caches/CREXE"
      linux: "~/.cache/crexe"
    layout: "{{fingerprint}}"
    markers:
      manifest: "crexe.manifest.json"
      build_ok: ".build_ok"

policies:
  timeoutSeconds:
    generate: 180
    build: 300
    run: 300
  allowNetwork: true
  commandAllowlist:
    windows: ["cmd", "cl", "link"]
    macos: ["sh", "clang"]
    linux: ["sh", "gcc", "pkg-config"]

selectors:
  target:
    rules:
      - when: "{{ForceTarget}} == 'windows'"
        use: "windows"
      - when: "{{ForceTarget}} == 'macos'"
        use: "macos"
      - when: "{{ForceTarget}} == 'linux'"
        use: "linux"
      - when: "{{OS}} == 'windows'"
        use: "windows"
      - when: "{{OS}} == 'macos'"
        use: "macos"
      - when: "{{OS}} == 'linux'"
        use: "linux"
    fallback: "linux"

prompt_core:
  app_definition: |
    You will generate a native desktop calculator.
  output_contract: |
    Return only JSON with files[].

prompt:
  system: |
    You are a project generator for CREXE.
  user_by_target:
    windows: |
      {{prompt_core.app_definition}}
      Target: Windows Win32 C.
      {{prompt_core.output_contract}}
    macos: |
      {{prompt_core.app_definition}}
      Target: macOS Cocoa Objective-C.
      {{prompt_core.output_contract}}
    linux: |
      {{prompt_core.app_definition}}
      Target: Linux GTK3 C.
      {{prompt_core.output_contract}}

generator:
  mode: "online"
  provider: "openai"
  transport: "responses"
  baseUrl: "https://api.openai.com/v1"
  apiKeyEnv: "OPENAI_API_KEY"
  model: "gpt-5.5-pro"
  maxOutputTokens: 6000

targets:
  windows:
    build:
      steps:
        - cmd: ["cmd", "/c", "if not exist build mkdir build"]
        - cmd: ["cmd", "/c", "cl /nologo /O2 src\\main.c /Fe:build\\{{AppName}}.exe user32.lib gdi32.lib"]
    run:
      cmd: ["build\\{{AppName}}.exe"]
```
