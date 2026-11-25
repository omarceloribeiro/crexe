# Beyond execution: software as creative material

> Article · editorial perspective · 24/11/2026. A reflection on personal tools, based on CREXE's current experience and explicitly identified future possibilities.

A teacher receives a proportion calculator. It works, but the next lesson calls for something different: showing the calculation steps, accepting decimal numbers, and explaining why a particular value cannot be zero. The need is small. The distance between using the tool and changing it may be large.

Spreadsheets, programming languages, and visual platforms already offer ways to make this kind of adaptation. Each alternative has its advantages. CREXE's proposal is to add another entry point: a readable description of the program a person wants to create.

## Intent can also be a file

A `.crexe` file can begin with a sentence. In the [proportion calculator example](../../../examples/markdown/regra-de-tres.crexe), the request describes fields A, B, and C, the relationship A/B = C/X, and validation of numbers and division by zero.

The document is small enough to read before any generation. Its recipient can understand the objective and propose a change. The recipe does not hide its purpose behind an already finished interface; it puts the request first.

An edit could add:

```text
Show the formula used and the calculation steps.
Explain invalid inputs in Portuguese.
Keep fields A, B, and C and the result X.
```

This is still a specification to be interpreted. It is not proof that the result will be correct. The question becomes: how can this request be turned into an implementation that can be verified?

## The creative workflow already has concrete parts

In the version under development, CREXE provides an intent editor. On Windows, the current file association includes an edit option in the context menu; the `crexe edit` command opens the editor in a compatible graphical session. Saving modifies the document. Generation happens when the file is opened again for execution.

The configured model receives the request and, when usable history exists, the context of the approved revision. The engine prepares the project, compiles it, and runs the tests specified by the profile. Only after these stages does the new revision become the approved baseline. A change can fail; preserving the previous revision allows investigation without making every attempt a permanent update.

This workflow brings editing and experimentation closer together. It does not remove SDKs, dependencies, compilation errors, or technology decisions. It organizes those tasks around the user's intent.

## Creating also means deciding how to test

An interface opening successfully does not demonstrate that its mathematics is correct. For the teacher's calculator, evaluation could include simple inputs with known results, decimal numbers, empty fields, and invalid denominators. Presentation criteria would follow: is the explanation understandable to the class? Does the text fit in the window? Does keyboard navigation work?

Some of these criteria can be automated. Others require use and human judgment. The greater the consequences of an error, the stricter the review needs to be. A demonstration exercise and a system used for financial decisions should not receive the same treatment simply because both began as text.

Better specification helps but does not replace validation. A culture of software through intent needs to teach both practices together.

## Sharing the recipe and sharing the version

There are two different deliverables. A recipe lets someone else interpret, adapt, and generate an implementation in their environment. An exported project preserves the source files of the selected revision, with the instructions and tools needed to recompile them.

This distinction avoids a misleading promise: two people with the same `.crexe` do not necessarily receive identical programs. Models, configuration, and environment may vary. If the goal is to teach the intent, a recipe is a good unit of discussion. If the goal is to review an exact implementation, approved source files provide the more concrete reference.

The [exported project contract](../PROJETOS_V1.md) describes the available operations. Dependencies, assets, and templates remain subject to their terms of use; exporting does not automatically transfer unrestricted rights over all content.

## A creative environment is a broader vision

The handoff imagines design modes, visual customization, educational explanations, and comparisons between request and result. These are directions for a future creative environment. They are not all features of the current engine.

A useful evolution would have to make choices understandable: what changed, what was preserved, which model was used, what the tests demonstrated, and how to recover a previous version. The user would need to be able to stop the process, examine the source files, and recognize what has not yet been validated.

This experience would have value even without producing any application for any system. Starting with small, well-bounded tools makes it possible to learn where generation helps and where a ready-made component, spreadsheet, or conventional project remains the better option.

## Software as working material

Creative Executable's appeal lies in this possibility of participation. Receiving a tool can also mean receiving a readable way to discuss its behavior. An improvement can begin as an edit to intent, go through generation and testing, and end as a project that others can examine.

Creativity does not need to hide the engineering. It becomes stronger when the request, implementation, and evidence remain close together. That meeting is what CREXE seeks to explore: reducing the distance between needing a tool and being able to change it responsibly.

## Continue reading

- [Foundations of intent-based distribution](whitepaper-fundamentos.md).
- [What if we could also distribute intent?](artigo-distribuir-intencao.md).
- [Security and trust](guia-seguranca-e-confianca.md).
- [Licensing and ecosystem](../LICENCIAMENTO_E_ECOSSISTEMA.md).
