# Local and remote models in CREXE

> Technical guide · 24/11/2026 · State of v1 under development. Integrations with operating-system models are research; the current adapters are Ollama, OpenAI, and DeepSeek. External requirements may change.

Running generation on your own computer can reduce data transmission and make working offline possible after preparation. Using a remote service may provide models and computing capacity that the computer cannot accommodate. The choice depends on the task, data, and available resources.

Neither choice automatically makes generated code safe or correct. A local model also makes mistakes; a well-structured remote response still needs to be compiled and verified. The [security guide](guia-seguranca-e-confianca.md) describes the limits of current execution.

## Implemented options

| Adapter | Current use | Important conditions |
|---|---|---|
| Ollama | Local default, no API key | The service and model must be available; check the configured endpoint |
| OpenAI | Generation through a remote API | Requires local configuration, valid credentials, and service availability |
| DeepSeek | Generation through a remote API | Requires local configuration, valid credentials, and service availability |

There is no automatic fallback to a paid provider. The `.crexe` file does not choose the endpoint or credentials. Those decisions belong to the user in the [engine configuration](../CONFIGURACAO.md). An Ollama endpoint can point to another machine; the adapter's name alone does not prove that data remains on the device.

The configuration screen lets you choose the model, test the connection, and limit attempts. Listing models does not generate an application or guarantee that every listed model can satisfy CREXE's output contract. Keys saved through the interface are stored in the user's native credential vault; `.env` is loaded only when explicitly specified.

## Preparing a first generation

With the engine compiled, run these commands from the repository root on Windows:

```powershell
.\target\release\crexe.exe configure
.\target\release\crexe.exe doctor --check
.\target\release\crexe.exe inspect examples\plaintext\calculadora.crexe
```

Configure the provider and check the model before generating. `doctor --check` checks local SDKs without calling the provider; `inspect` shows how the recipe is interpreted and what it requires. These checks differ from the connection test in the configuration screen.

After reviewing the recipe and accepting execution with the user's permissions, use this command to generate and open it:

```powershell
.\target\release\crexe.exe examples\plaintext\calculadora.crexe
```

A failed generation does not necessarily mean the model lacks capability. Causes may include a missing SDK, an incompatible response contract, a context limit, or a compilation error. The engine allows build/test repairs within the local budget; authentication, balance, connection, or invalid-response failures end the attempt. Increasing that budget indiscriminately does not address the cause.

## What needs to work offline

“Works offline” must be tested phase by phase, with the environment prepared and the connection genuinely unavailable.

| Phase | What to check |
|---|---|
| Installation | Are the engine, inference service, weights, SDKs, and libraries already available? |
| Generation | Is the endpoint local? Can the model load and respond without external services? |
| Compilation | Are all necessary dependencies present, including project packages? |
| Opening from cache | Are the approved revision and its runtime resources still available? |
| Application use | Does the requested behavior depend on APIs, remote files, or authentication? |

CREXE does not install SDKs automatically. A build may need to restore dependencies, depending on the [selected profile](../PERFIS_V1.md). Reopening a valid revision from cache avoids another generation but does not remove the application's own requirements. The source ZIP supports recompilation without CREXE, provided the necessary tools and dependencies are present.

It is also necessary to distinguish “no internet during use” from “network access denied by the system.” V1 does not provide network containment for generated code.

## Models offered by the operating system

Public APIs for local models already exist on major platforms. The claim that internal models are always inaccessible to applications has become outdated. Availability depends on the device, system version, settings, and API conditions.

| Platform | Documented access | Limits to consider |
|---|---|---|
| Apple | Foundation Models provides access to the on-device model used by Apple Intelligence | Check eligibility, activation, and model availability before creating a session |
| Android | ML Kit GenAI accesses Gemini Nano through the AICore service | Support varies by device and API; usage limits and foreground execution restrictions apply |
| Windows | Windows AI APIs expose local inference, documented for Phi Silica and its evolution | Hardware, SDK, distribution channel, and access requirements vary; query model state |

Apple documents structured generation and availability checks. An integration needs to query availability at runtime and report when the model is not ready. The documentation itself lists code generation among the tasks not recommended for this model. [Apple: generation with Foundation Models](https://developer.apple.com/documentation/foundationmodels/generating-content-and-performing-tasks-with-foundation-models).

Google offers specialized APIs and prompt-based generation, but each device/model combination has its own conditions. The documentation describes quotas and restrictions on background execution. [Google: ML Kit GenAI](https://developers.google.com/ml-kit/genai/).

Microsoft's documentation already announces the transition from Phi Silica to Aion Instruct. Phi Silica requires access authorization through a Limited Access Feature; conditions change in the announced transition. Model names and requirements must be checked during an integration; they are not a stable basis for promising universal support. [Microsoft: Phi Silica and Windows AI APIs](https://learn.microsoft.com/windows/ai/apis/phi-silica).

These APIs **are not integrated into current CREXE**. Availability for summarization or text generation does not establish quality for complete projects. On mobile devices, local inference likewise does not imply permission to freely compile and execute native programs. That requires platform-specific investigation.

## Repetition, caching, and reproduction

Fixing the model, parameters, and seed helps reduce variation when the backend supports it, but does not guarantee identical outputs across versions, hardware, or implementations. PyTorch's documentation, for example, distinguishes control of randomness from reproduction across platforms and between CPU and GPU. This illustrates a technical limit; it is not a statement that CREXE uses PyTorch. [PyTorch: Reproducibility](https://docs.pytorch.org/docs/2.15/notes/randomness.html).

There are three different objectives: obtaining equivalent behavior when regenerating; rebuilding the same bytes from controlled source files and tools; and reusing an already approved artifact. CREXE's cache serves the third. It does not demonstrate the first two.

## How to evaluate a model

Compare models using a fixed collection of small tasks and independent criteria: requirements met, successful builds, functional tests, and actual interface use. Record versions, context, parameters, machine, memory, duration, and number of repairs. Include failures, truncated responses, and incremental changes as well as first generations.

Measure preparation, generation, and compilation separately. A large initial download or an already loaded model changes the experience. Reports from one machine provide localized evidence, not universal requirements or a controlled comparison.

A future integration with system models should preserve the local choice of provider, expose unavailability without silent substitution, and validate contracts, privacy, and execution limits. These criteria allow new backends to be investigated without presenting research as a delivered feature.
