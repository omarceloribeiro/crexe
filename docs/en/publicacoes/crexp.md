# CREXP — Creative Executable Package

**Status: minimal preview on `feature/v1`; the complete architecture remains on the roadmap.** CREXP is a ZIP of specifications: multiple Markdown documents describe the software you want to generate. The initial implementation turns these documents into textual intent and reuses the engine's generation, build, test, and repair workflow.

This is the maintained guide to the preview, and the English counterpart of the Portuguese guide. It is **not a full translation of the handoff**. The [complete original Portuguese handoff](../../formatos/originais/CREXP-HANDOFF-COMPLETO.md) is available unchanged. It contains future proposals, pseudocode, and earlier licensing discussions; the [current policy](../LICENCIAMENTO_E_ECOSSISTEMA.md) is MIT for the engine and MIT-0 for designated output materials.

## Three files, three uses

| Format | What you share | On opening |
| --- | --- | --- |
| Textual `.crexe` | A simple recipe | Generates or reuses the accepted cached revision |
| `.crexp` | A ZIP of Markdown SDD documents | Consolidates specifications and enters the generation workflow |
| `.crexb` | Recipe and an accepted executable snapshot | Opens a compatible payload without AI or recompilation |

SDD means specification-driven development. Requirements, plans, and acceptance tests can live in separate documents. A ZIP recognized as CREXP or CREXB may also use the universal `.crexe` extension; its content determines the workflow. Renaming a generic ZIP does not make it a recipe.

## Preview scope

The minimal implementation accepts UTF-8 Markdown documents with portable ASCII internal filenames and an optional `crexp.yaml` manifest. Text can be in Portuguese, English, or another language; the external package filename can use Unicode. Documents are consolidated deterministically by priority, role, and path, without an extra LLM call to interpret the package. Build and repair attempts continue to follow the engine's normal settings, with a locally configured profile.

A package without a manifest must have a recognizable SDD structure, such as `spec.md`, `plan.md`, and `tasks.md`. A random Markdown file in an arbitrary ZIP is insufficient. With a manifest, declare which documents form the intent. The preview does not semantically resolve conflicting requirements, implement agents, or copy assets or seed code into the generated project.

## Assemble a package

To start with a ready example, [download calculadora.crexp](../../../examples/crexp/calculadora.crexp). The package contains `requirements.md`, `tests.md`, and `crexp.yaml`, with Portuguese text. It is a generation specification, not a compiled program. Its [sources and example instructions](../../../examples/crexp/README.md) are also available.

Place the files directly at the ZIP root, without an enclosing folder:

```text
calculator.crexp
├── crexp.yaml
├── spec.md
├── plan.md
└── acceptance.md
```

A minimal manifest uses:

```yaml
crexp: 1
package:
  name: Calculator
  id: org.example.calculator
  version: 1.0.0
documents:
  - path: spec.md
    role: requirements
    priority: 100
  - path: plan.md
    role: plan
    priority: 20
  - path: acceptance.md
    role: acceptance
    priority: 90
```

`package.id`, `package.version`, `role`, and `priority` are optional. The manifest must list every Markdown document exactly once. They must exist and be accepted Markdown files. These fields are the preview subset; the handoff's extensive manifest must not be treated as an already supported configuration. `role` and `priority` organize input; they do not prove that contradictory requirements have been resolved.

In `spec.md`, describe the four arithmetic operations and interface. In `acceptance.md`, specify, for example, that `6 × 7` must return `42` and division by zero must display an error. These criteria become part of the intent; prose alone does not become a formal test suite executed by the engine.

## Inspect and generate

With the engine built, from the checkout root:

```powershell
.\target\release\crexe.exe inspect calculator.crexp
.\target\release\crexe.exe exec calculator.crexp
```

On macOS/Linux, use `./target/release/crexe`. For an engine distribution package, use the executable in its extracted directory. `inspect` checks the input without calling the model. Running the package may call your configured provider and requires the profile's SDKs. The [configuration instructions](../CONFIGURACAO.md) and [profiles](../PERFIS_V1.md) still apply.

The consolidated recipe used for generation and its inventory live at `packages/recipes/<hash>/` under the engine's installation root, respecting `CREXE_HOME`. Inspection does not write those files. Materialization rejects tampered content instead of overwriting it. Deterministic document ordering does not mean the model will always produce identical code.

The hash identifies materialized content; revision history uses the original `.crexp` path. Editing the package at that path can evolve the latest accepted revision. A failed new attempt does not replace the accepted baseline used for CREXB export.

On Windows, `install` and `associate` include `.crexp` and `.crexb` in double-click opening. There is no global `.zip` association, and the engine's text editor does not edit ZIP packages. On other systems, use the CLI in this preview.

## Content and privacy

**CREXP documents are the recipe.** When generating with a remote provider, the consolidated intent from those documents is sent to the configured model. Do not include secrets or data you do not want to send. A local model keeps this step in the local environment, subject to its configuration and dependencies.

This differs from [Generate with CREXE](../GENERATE_WITH_CREXE.md): that workflow uses only an ordinary document's extension to plan a reader, and does not send the document's name, path, contents, or metadata to the model. CREXP and CREXB are engine inputs, not ordinary documents for the reader generator.

## ZIP validation

The engine validates archive structure and applies limits before interpreting documents. Paths escaping the root, ambiguous names, symbolic links, duplicate entries, and files outside the supported subset are reasons for rejection. Invalid archives must not be reinterpreted as text to try to continue.

The CREXP preview limits are 8 MiB for the ZIP, 1 MiB expanded, 128 entries, 256 KiB per file, a depth of 12 levels, and a maximum compression ratio of 200. ZIP64, encrypted entries, and multi-volume archives are not accepted. These are implementation limits, not the larger limits proposed in the handoff.

These checks protect package reading. They do not isolate the compiler or generated application: execution still takes place on the host with the user's permissions. Read the [security limits](../SECURITY.md).

## From the preview to the complete proposal

The [formats roadmap](roadmap-formatos.md) explicitly tracks the remaining work: richer document classification, precedence and conflicts, Recipe Compiler, inventory, resolved recipes, lock files, supporting resources, agents, resolvers, publishing, and trust policies.

The complete goal is to preserve software intent independently of language, framework, or technological generation. The preview demonstrates the first step: multiple SDD documents in one file that the engine can read and use for generation.

## Downloads and references

- [Complete CREXP handoff — original Portuguese Markdown](../../formatos/originais/CREXP-HANDOFF-COMPLETO.md).
- [Original file SHA-256 checksums](../../formatos/originais/checksums.sha256).
- [CREXB: distribute the accepted result](crexb.md).
- [Complete roadmap for both formats](roadmap-formatos.md).

The site's language selector offers this complete maintained guide in English and Portuguese. The complete handoff remains in its original Portuguese.
