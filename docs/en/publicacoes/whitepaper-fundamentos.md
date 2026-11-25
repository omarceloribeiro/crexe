# CREXE: the foundations of a creative executable

> White paper · informational · 24/11/2026. Describes the foundations and the current state of v1 under development. It is neither a normative specification nor a standard approved by the IETF or another standards organization.

## Abstract

CREXE stands for **Creative Executable**. A `.crexe` file expresses an application's intent in readable text. An engine interprets the document, combines the request with local configuration, asks a model for source files, and coordinates compilation, verification, and execution. The recipe can circulate independently of the application it produces; the generated project can also be exported.

The proposal expands the options for distributing and customizing small tools. Its usefulness depends on concrete conditions: a compatible model, available development tools, suitable tests, and a willingness to run the result. The current implementation demonstrates this workflow but remains under development and provides no security isolation. This document distinguishes the mechanism available today from hypotheses that still require research.

## Intent as a shareable artifact

A person may need a calculator with particular rules, an interface for a recurring task, or a classroom exercise. Sharing the description lets someone else understand the objective and adapt it before generation. A simple request could be the complete contents of `calculadora.crexe`:

```text
Generate a simple calculator with the four arithmetic operations,
a clear button, division-by-zero handling, and a green background.
```

This description does not determine every implementation detail. Two generations may resolve the same request differently, including behavior that was never requested. A recipe is portable intent; equivalence between results must be demonstrated. When the objective is to share the exact source files of an approved version, the exported project is the appropriate artifact.

## Documents accepted by v1

The extension supports three documented ways to express a request. They meet different needs for structure without requiring every user to write an extensive specification.

| Form | Main content | Role in the experience |
|---|---|---|
| Plain text | The entire document expresses the intent | Start with a description; the name comes from the file |
| Markdown | Free-form body and a recognized header with `crexe: 1` | Organize the request and optionally specify a name and profile |
| Legacy YAML | Structured inputs, selectors, intent, and targets | Preserve recipes with explicit commands and conditions |

The [format contract](../FORMATOS_V1.md) is the reference for the fields the engine actually recognizes. Recognized structures that are invalid are not silently reinterpreted as free-form text. Historical RFCs record the proposal's evolution; their examples do not establish that a feature is available. This white paper adds no input formats or fields to the contract.

## Architecture and execution workflow

The engine is implemented in Rust. Its role is to coordinate verifiable steps and maintain the state needed for resuming work and making changes. The model proposes source files; the engine validates the response and executes the operations defined by the profile or legacy recipe, subject to local policy.

```text
.crexe file + local configuration
                 |
        reading and normalization
                 |
 environment, profile, and previous revision
                 |
       valid cache? ---- yes ----> open approved artifact
                 |
                 no
                 |
       local model or remote service
                 |
     validation of received source files
                 |
       compilation + defined tests
                 |
       failure? ---- yes ----> repair within limits
                 |
           approved revision
                 |
        cache + exportable project
                 |
           optional execution
```

The profile specifies tools, project files, and operations. There are profiles for Windows Forms, console applications, and native interfaces, as well as OpenGL options. New profiles remain experimental, as described in the [prerequisite matrix](../PERFIS_V1.md). Detecting the system and finding a compiler does not guarantee that every request can be fulfilled. The engine does not emulate another system or automatically install global SDKs.

## Local choices and remote generation

The user selects the provider, model, endpoint, and credentials in local configuration. Ollama provides the local-model workflow; OpenAI and DeepSeek are remote options. A local failure does not authorize an automatic switch to a paid service. For remote generation, the request and the source context needed for the work are sent to the selected provider.

The available adapters and their limits are described in [configuration](../CONFIGURACAO.md). Historical records of experiments with other transports do not imply current support for them. A model is not compatible simply because it appears in a list: it must respond according to the contract accepted by the engine.

Credentials can be stored in the system credential vault. Configuration defines limits on attempts, output, and time. These controls constrain operation but do not amount to a precise spending cap: the output budget does not fully account for every possible API cost.

## Continuity when editing a recipe

Editing and saving the text does not start generation. When the document is opened again, the engine can use the last approved revision as context: the previous request, text source files, and the new request. This makes it possible to request an adjustment without deliberately starting from scratch.

Continuity is guidance and a history structure, not a guarantee of semantic preservation. The model can justify recreating the application or changing the profile when profile selection is not fixed. A visual change can also introduce functional regressions. Tests should therefore accompany the rules that need to remain intact.

Failures and cancellations do not advance the index to an incomplete revision. `--rebuild` forces a new generation using history; `--fresh` requests a start without that context. History belongs to the document's canonical path: moving or copying the recipe creates a different local history.

## Cache, provenance, and reproducibility

The cache preserves approved revisions, a source inventory, and hashes. It lets the engine reuse an existing result and identify changes or inconsistencies in recorded files. That function differs from reproducing a generation.

Repeating a model call may produce different source files. Recompiling the same source may produce different bytes depending on the compiler, dependencies, and environment. Research into reproducibility would need to control those variables and specify which property it seeks to reproduce: behavior, source files, or the binary. A hash identifies bytes; it does not prove correctness, security, or functional equivalence.

## The project remains useful outside the engine

Every approved revision includes a source package. The `CREXE-PROJECT.json` manifest records files and operations; scripts support recompilation with the required tools, without another AI generation. The CLI also provides build, test, run, local publish, and export operations for existing projects.

These operations have distinct limits. Exporting edited source files does not automatically validate the changes. Publishing produces local files; it does not deploy them. In the .NET profile, published output depends on the runtime. SDKs, system libraries, and execution conditions remain relevant, as described in the [project contract](../PROJETOS_V1.md).

## Trust and security limits

In v1, compilation and execution take place on the host with the user's permissions. The temporary directory organizes the work but does not restrict access to files, the network, or other resources. Path validation, response limits, tool policy, and cancellation address specific failures; they do not make unknown code safe.

Tests must also be distinguished from independent confirmation. A self-test produced by the same model may repeat a mistaken interpretation of the request. Compilation and passing that test provide useful evidence, but are insufficient to establish that the application meets every requirement. The [execution limits](../SECURITY.md) should guide any evaluation of the project.

## How to evaluate the proposal

A useful evaluation compares the complete workflow with suitable alternatives, using tasks and acceptance outcomes defined in advance.

| Dimension | Evidence to record |
|---|---|
| Correctness | Independent test cases, failures, and regressions after editing |
| Experience | Time to first use, interventions required, and clarity of diagnostics |
| Cost | Inference, tool installation, dependencies, storage, and maintenance |
| Portability | Hosts and architectures actually tested, with their prerequisites |
| Continuity | Preservation of previous rules and recovery after interruptions |

There is no guaranteed reduction in bandwidth or processing. A small recipe may require large models, SDKs, and dependencies. First generation and reopening from cache must be measured separately. The [current distribution documentation](../../DISTRIBUICAO_V1.md) distinguishes development snapshots from a validated public release.

## Openness and next steps

The engine uses [MIT](../../../LICENSE), allowing ports and open or proprietary integrations while preserving its notices. Users choose the generated application's license, including a proprietary license, without an obligation to publish its source or credit CREXE for the designated output materials. The MIT-0 grant for those materials and official recipes is formalized in [LICENSE-GENERATED](../../../LICENSE-GENERATED); this whitepaper only describes it. Third-party dependencies, assets and recipes, as well as model/provider terms, retain their conditions.

Progress depends on evidence about quality, compatibility, security, and operation on real machines. The [vision for regenerable computing](whitepaper-computacao-regeneravel.md) extends the hypothesis to a longer horizon while preserving the distinction between demonstrated capabilities and proposed research.
