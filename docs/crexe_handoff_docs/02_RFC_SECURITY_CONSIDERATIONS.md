# CREXE Security Considerations

**Status:** Draft 0.1  
**Applies to:** CREXE runtimes, `.crexe` authors, enterprise deployments

---

## 1. Abstract

CREXE introduces a powerful model: executable software generated from human-readable specifications and prompts.

This creates a new attack surface:

- specification-level attacks
- prompt-level malware
- generated code attacks
- build command abuse
- workspace escape
- cache poisoning
- malicious target selection

This document defines baseline security requirements for CREXE runtimes.

---

## 2. Fundamental principle

A `.crexe` file MUST be treated as untrusted.

The runtime MUST NOT trust:

- the YAML specification
- the prompt
- the generated code
- build commands
- run commands
- target selectors
- model output

---

## 3. Threat model

Potential adversaries may attempt to:

1. Generate malicious code.
2. Execute arbitrary commands.
3. Write outside the workspace.
4. Read sensitive local files.
5. Exfiltrate data.
6. Modify the runtime.
7. Modify the original `.crexe`.
8. Poison cache.
9. Hide payloads in generated files.
10. Abuse shell commands.

---

## 4. Workspace isolation

The runtime MUST:

- write only inside the selected workspace
- reject absolute generated paths
- reject `../` traversal
- reject path prefixes outside the workspace
- avoid following symlinks outside the workspace where possible

Generated files MUST be normalized before writing.

---

## 5. Command allowlist

The runtime MUST restrict build commands to approved tools.

Example:

```yaml
policies:
  commandAllowlist:
    windows: ["cmd", "cl", "link"]
    macos: ["sh", "clang"]
    linux: ["sh", "gcc", "pkg-config"]
```

Run commands are special:

- external commands remain restricted
- generated executables inside workspace/cache MAY be allowed
- generated executables outside workspace MUST be rejected

---

## 6. Network policy

Default policy SHOULD be:

```yaml
allowNetwork: false
```

Exception:

- v1 online generator requires network during generation
- build/run SHOULD remain offline by default

Future runtimes SHOULD separate:

```yaml
network:
  generate: true
  build: false
  run: false
```

---

## 7. Generator output validation

The generator MUST output only:

```json
{ "files": [] }
```

The runtime MUST reject:

- markdown wrappers in strict mode
- hidden scripts outside declared files
- base64 executable payloads if policy forbids binaries
- absolute paths
- path traversal
- empty file list
- oversized output

---

## 8. Code review mode

A safe runtime SHOULD support:

```cmd
crexe inspect app.crexe
crexe generate app.crexe --no-build
crexe diff app.crexe
crexe build app.crexe
```

Enterprise mode SHOULD require code review before build/run.

---

## 9. Signed CREXE

Future CREXE versions SHOULD support digital signatures.

A signature may cover:

- entire `.crexe`
- `prompt_core`
- target definitions
- generator config
- security policies

A marketplace or enterprise registry may require signed CREXE files.

---

## 10. Prompt-level malware

A malicious `.crexe` might instruct the LLM to generate:

- backdoors
- persistence mechanisms
- credential stealers
- destructive commands
- hidden network calls

Mitigations:

- strict output contract
- generated code review
- static scanning
- sandboxed compilation
- no network during run
- no filesystem access outside app workspace

---

## 11. Regenerable malware risk

CREXE can generate code adapted to the local OS.

This is powerful but dangerous.

A permissive runtime could become:

> an AI-powered malware distribution format.

Therefore CREXE MUST be:

- restrictive
- auditable
- sandboxed
- deterministic where possible
- explicit in permissions

---

## 12. Auto-modification

The runtime SHOULD NOT allow a generated program to modify:

- the original `.crexe`
- the CREXE runtime
- global CREXE registry
- associated file handlers

unless explicitly enabled by a high-trust policy.

---

## 13. Minimal safe runtime checklist

A runtime is minimally safe if it implements:

- workspace isolation
- command allowlist
- generated executable workspace check
- path traversal rejection
- output JSON validation
- cache fingerprint
- explicit network boundaries

---

## 14. Conclusion

Security is not an optional feature of CREXE.

It is foundational.

CREXE should be creative, regenerable, and powerful — but not autonomous, unrestricted, or opaque.
