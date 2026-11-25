# Security and trust in CREXE

> Technical guide · 24/11/2026 · Describes CREXE v1 under development and separates existing controls from security proposals. The operational reference is the [project security policy](../SECURITY.md).

A `.crexe` file describes a program; the engine turns that description into source files, compiles them, and runs tests before opening the result. Each transition introduces a trust decision. A readable description can produce incorrect code. An authorized compiler can execute project steps. A passing test can leave important behavior uncovered.

In the current implementation, tools and generated programs run **on the host, with the user's permissions**. The temporary directory organizes files but does not provide isolation. There is no general containment of files, network access, or subprocesses. This condition should guide the decision to run any recipe, even when the model is local.

## What exists today

The engine's controls address specific failures. Understanding their limits avoids turning a partial protection into a security promise.

| Current control | What it provides | What it does not provide |
|---|---|---|
| Path validation and file limits | Restricts files materialized by the engine | Control over files later accessed by native code |
| Local allowlist of tools | Checks executables started by the engine, using their resolved paths | Containment of subprocesses started by those tools |
| Credentials in the user's vault | Avoids storing the key directly in TOML | A general barrier preventing an application from accessing user data |
| Removal of known credential variables | Reduces accidental inheritance of secrets by child processes | A guarantee that no other secrets are accessible on the host |
| Response, log, and time limits | Contains certain resource failures and supports cancellation | General memory, disk, or network quotas for all executed code |
| Cache revisions and hashes | Identifies preserved material and allows an approved revision to be reopened | Proof of functional correctness, authorship, or absence of malicious behavior |

Provider, endpoint, and credentials are local choices. A recipe cannot expand the local tool allowlist or call budget. Nevertheless, authorizing a compiler does not make every project safe. The complete rules are in [local configuration](../CONFIGURACAO.md).

Legacy YAML can express `allowNetwork`, but `false` is rejected because the corresponding isolation is not implemented. There is currently no effective “network off by default” policy for the generated program. The rejection makes that missing capability explicit.

## Preparation and inspection

On Windows, from the repository root and with the engine already compiled, these commands help you understand the environment:

```powershell
.\target\release\crexe.exe doctor
.\target\release\crexe.exe doctor --check
.\target\release\crexe.exe inspect examples\plaintext\calculadora.crexe
```

`doctor` displays requirements and paths without executing the SDKs. The `--check` variant runs local tool probes, subject to policy and timeouts; it does not call the provider, install dependencies, or compile an application. `inspect` resolves intent and requirements without executing the SDKs. Finding a tool does not prove its provenance or a recipe's safety.

The `--no-run` argument serves another purpose: during a new generation, it retains compilation and the defined tests, including code produced by the model, but suppresses normal execution of the application. For OpenGL profiles, the graphical check is also omitted. Reusing a cached revision does not recompile or repeat the test suite. Therefore, `--no-run` is not a safe mode for inspecting unknown code. Before generating, understand the recipe's origin, review its intent, and confirm that the environment is suitable for running the result with your permissions.

In the [current profiles](../PERFIS_V1.md), the self-test is generated alongside the program. It helps find errors but can repeat the same mistaken interpretation. For important decisions, include independent tests with expected outcomes defined by someone who understands the problem.

## Separating trust boundaries

The workflow can be read as follows:

```text
Received intent
    → interpretation by the model
    → produced files
    → build tools
    → tests and application
    → user files, network, and other resources
```

A failure at any transition can escape previous checks. Responding in the expected JSON proves the shape of the response, not the safety of the source files. Likewise, instructing the model to ignore dangerous requests does not create a permission boundary. OWASP treats prompt injection as a risk requiring validation, least privilege, and boundary testing, in addition to instructions given to the model. [OWASP: Prompt Injection](https://genai.owasp.org/llmrisk/llm01-prompt-injection/).

This separation also applies to privacy: local inference may avoid sending intent to a remote service, but the resulting application remains able to access the network if the system permits it. Model locality and program containment are distinct properties.

## Proposed architecture for future development

The measures below are **proposals**, not options that can already be enabled in v1.

| Proposed layer | Verifiable responsibility |
|---|---|
| Structural contract | Version fields, types, and limits; reject unsupported capabilities before execution |
| Generator boundary | Treat the response as data; separate any tool access through authorized interfaces |
| Build isolation | Contain compilers, projects, and descendants; control reads, writes, and dependencies |
| Application isolation | Enforce file, network, process, and interface permissions during execution |
| Declared capabilities | Show requested access and verify whether the backend can enforce it |
| Provenance and signatures | Bind identity, intent, source files, dependencies, and the artifacts actually verified |
| Review and recovery | Allow inspection before execution, preserve evidence, and return to the previous revision |

A network policy must define the phase it applies to: model download, remote generation, dependency restoration, and execution are different operations. A backend unable to enforce the selected policy must fail explicitly. Writing a capability into a document does not implement it.

Signatures require a precise scope: signing intent is not equivalent to signing every binary that different models may produce. A signature identifies the origin of the signed material; it does not demonstrate that the material is harmless. Key rotation, revocation, and the relationship between revisions also require design work.

## Evidence needed

Development should begin with a threat model: which resources need protection, who supplies each input, and which components may be compromised. Every guarantee then needs independent trials. A minimum plan includes:

1. Verify that access outside granted permissions fails, including in descendant processes and build tools.
2. Observe files and connections from outside the application under test; its own declaration of success is insufficient.
3. Exercise cancellation, backend unavailability, resource limits, and recovery without promoting incomplete revisions.
4. Separate integrity, behavior, and isolation tests; publish the environment and limitations of each result.

The current policy does not include Windows Sandbox, Docker, or WSL as isolation backends. Future research requires its own implementation, review, and evidence before changing that condition.

To report a problem, use synthetic data, the engine version, and a minimal example. Do not publish keys, private caches, or environment dumps. The [security policy](../SECURITY.md) remains the reference for current limits and for reporting failures.
