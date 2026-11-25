# CREXE — Creative Executable Specification

**Status:** Draft 0.1  
**Category:** Informational / Proposed Standard  
**File extension:** `.crexe`  
**Proposed media type:** `application/crexe+yaml`  
**Alternative experimental media type:** `application/x-crexe`

---

## 1. Abstract

This document proposes **CREXE**, a Creative Executable file format and execution model for distributing software as generative specifications instead of precompiled binaries.

A `.crexe` file defines:

- application intent
- behavior rules
- target platform conditions
- generator configuration
- toolchain requirements
- build instructions
- run instructions
- cache and security policies

Instead of distributing binaries, CREXE allows software to be generated and compiled locally for the host operating system and architecture.

---

## 2. Motivation

Traditional software distribution relies on precompiled artifacts:

```text
app.exe
app.dmg
app.deb
app.apk
```

This creates fragmentation across:

- operating systems
- CPU architectures
- runtime versions
- dependency versions
- packaging ecosystems

CREXE proposes a different model:

> Distribute intent. Compile locally.

Instead of shipping multiple binaries, a publisher ships:

```text
application.crexe
```

The host runtime generates and builds the appropriate native application for the current environment.

---

## 3. Terminology

### CREXE

A Creative Executable specification file.

### Runtime

The program that reads, generates, builds, caches, and runs `.crexe` files.

### Prompt Core

The OS-independent definition of the application and its behavioral rules.

### Target

An OS/toolchain-specific implementation path.

### Generator

The engine that turns a CREXE prompt into project files. In v1, this is typically an online LLM.

### Workspace

A local directory where generated files, build artifacts, logs, and manifest files are stored.

### Fingerprint

A deterministic hash used to decide whether regeneration/rebuild is required.

---

## 4. Core Principles

### 4.1 Intent over binary

The `.crexe` file describes what the application is, not a fixed binary representation of it.

### 4.2 Local compilation

Binaries are produced on the host machine.

### 4.3 OS-aware generation

The runtime injects environment variables such as:

- `OS`
- `ARCH`
- `USER_HOME`
- `RUNTIME_VERSION`

### 4.4 Single prompt, multiple native outputs

A CREXE file can define one prompt core and different target-specific wrappers.

Example:

- Windows → C + Win32 API
- macOS → Objective-C + Cocoa
- Linux → C + GTK3

### 4.5 Deterministic caching

If the specification and effective inputs do not change, the runtime should reuse the cached build.

### 4.6 Human-readable format

The recommended v1 format is YAML.

---

## 5. File structure

A CREXE file SHOULD include:

```yaml
version: 1.0

meta:
  id: "example.app"
  name: "Example App"
  description: "Description"

inputs:
  AppName:
    type: string
    default: "ExampleApp"

workspace:
  mode: cache

policies:
  timeoutSeconds:
    generate: 180
    build: 300
    run: 300

selectors:
  target:
    rules:
      - when: "{{OS}} == 'windows'"
        use: "windows"

prompt_core:
  app_definition: |
    Define the application here.

prompt:
  system: |
    You are a project generator.
  user_by_target:
    windows: |
      {{prompt_core.app_definition}}
      Generate Windows implementation.

generator:
  mode: online
  transport: responses
  model: gpt-5.5-pro

targets:
  windows:
    build:
      steps:
        - cmd: ["cmd", "/c", "..."]
    run:
      cmd: ["build\\ExampleApp.exe"]
```

---

## 6. Execution model

When executed, the runtime SHOULD:

1. Load the `.crexe` file.
2. Validate schema.
3. Resolve inputs and overrides.
4. Detect OS and architecture.
5. Select target.
6. Compute fingerprint.
7. If cache hit, run cached executable.
8. If cache miss:
   - build final prompt
   - call generator
   - validate generator output
   - write files to workspace
   - build
   - store manifest
   - run

---

## 7. Generator output contract

The generator MUST return a JSON object:

```json
{
  "files": [
    {
      "path": "relative/path",
      "content": "file content"
    }
  ],
  "hints": {
    "entrypoint": "optional",
    "notes": "optional"
  }
}
```

The runtime MUST reject:

- markdown outside JSON in strict mode
- absolute output paths
- `../` traversal
- empty `files`
- non-string `content`

---

## 8. Caching

A runtime SHOULD compute fingerprint from:

- CREXE file hash
- effective input values
- runtime version
- generator provider
- generator model
- generator transport
- relevant generation parameters

---

## 9. Security considerations

CREXE implementations MUST treat every `.crexe` file as untrusted.

Required baseline:

- command allowlist
- workspace isolation
- no write outside workspace
- generator output validation
- path traversal prevention
- generated executable allowed only inside workspace/cache
- network disabled during build/run unless explicitly approved

---

## 10. Compatibility model

CREXE does not guarantee universal compatibility by itself.

It shifts compatibility from distribution time to generation/build time.

A `.crexe` file is compatible if:

- a runtime exists
- the target toolchain exists
- the target prompt is valid
- generated code builds successfully

---

## 11. Future extensions

- signed `.crexe`
- schema registry
- marketplace metadata
- local LLM support
- visual design/edit mode
- enterprise policies
- sandboxed build containers
- reproducible build verification
- dependency lockfiles
- package signing

---

## 12. Conclusion

CREXE proposes a new software file category:

> a creative executable specification.

It is not source code in the traditional sense, not a binary, and not merely an installer.

It is a recipe for generating executable software locally.
