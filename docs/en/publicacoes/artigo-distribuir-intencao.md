# Distributing intent: another way to share software

> Article • editorial perspective • 24/11/2026

An application reaches us as a ready-made answer. Someone chose its features, organized the interface, compiled the code, and prepared a distribution. That work makes it possible to install and use a tool without reconstructing every decision that produced it. For many situations, this remains an excellent way to deliver software.

There are, however, small and specific needs for which no ready-made answer is satisfactory. A calculator needs a different presentation. An activity calls for different rules. A tool solves almost everything, but lacks precisely the operation that matters to its user. In those cases, also sharing a description of what we want to build may open up another possibility.

CREXE explores that possibility. Its name stands for **Creative Executable**. A `.crexe` file expresses an application's intent and participates in a process that generates source files, compiles them, and runs the result on the user's computer. The proposal adds a way of sharing software alongside binaries, source code, and other formats that remain useful.

## A description that can be transformed

Consider a file named `calculadora.crexe` with the following content:

```text
Generate a simple calculator with the four arithmetic operations,
a clear button, division-by-zero handling, and a green background.
```

This text presents an understandable intent before any compilation. Its recipient can evaluate the request, change the colors, or explain a behavior more clearly. The description also makes visible some of the decisions that, in a standalone executable, would have to be discovered through its interface or documentation.

That does not mean the text determines every detail of the program. An implementation still has to be chosen, ambiguities resolved, and the result verified. A short description leaves many decisions open: screen organization, input handling, accessibility, and internal structure. Writing a better recipe is part of the creative work.

In the current implementation, the engine accepts plain text, Markdown, and a legacy YAML format. It uses local generation configuration and the available profiles to prepare the project. Compilation and the tests available in the profile precede approval of a revision. Concrete requirements are in the [project guide](../README.md) and [profile documentation](../PERFIS_V1.md).

## Editing the request is also design

After trying the calculator, someone might add: “keep the current behavior and enlarge the buttons to make it easier to use on a small screen.” Intent begins to record a need noticed during use. The file becomes more than a starting point: it becomes part of the conversation with the tool.

CREXE already maintains local history for each document. When intent changes, generation receives the previous request, approved source files, and current request. The guidance is to preserve unaffected behavior, but that guidance does not guarantee every change will be correct. The result must be checked, including features that should have remained the same.

A preparation failure does not replace the last approved revision. Reopening without changes can reuse the validated revision in the cache. This mechanism avoids repeated work under the expected conditions; it does not make generation deterministic or demonstrate that two machines will produce the same source files. Caching and reproducibility are related subjects with different requirements.

## Sharing a recipe or sharing a version

A recipe and an exported project serve distinct purposes. A recipe lets the recipient revisit the request and produce an implementation in the destination environment, provided suitable support and tools exist. Exported source files preserve a concrete realization of that request and can be studied, modified, and recompiled with their requirements.

Distributing intent therefore does not eliminate source distribution. In CREXE, export includes project files, a manifest, and scripts. The recipient can work on that material without depending on the engine to recompile it, according to the [project operations contract](../PROJETOS_V1.md). The result remains subject to dependencies and environmental conditions.

Users choose the application's license, including a proprietary license, and may sell it without an obligation to publish its source or credit CREXE. The engine remains GNU GPL v3; its materials incorporated into output and official recipes have an MIT-0 grant. Third-party dependencies, assets and recipes, as well as model/provider terms, retain their conditions. The [licensing record](../LICENCIAMENTO_E_ECOSSISTEMA.md) explains the scope of that freedom and the notice included in exports.

## Freedom needs criteria

Local generation does not make code trustworthy by origin. A program compiled on one's own machine can still contain errors or perform unwanted actions. In the current version, tools and applications run with the user's permissions; the working directory does not provide security isolation. Reading the recipe helps explain the request, but does not replace examining the implementation and deciding whether it should run.

There are also concrete costs: preparing SDKs, obtaining dependencies, waiting for generation and compilation, and reviewing results. Some tasks will continue to be better served by an application that has already been distributed and tested. Others may justify a customized tool. That choice should account for the complete work, without assuming that generation will always be faster, cheaper, or simpler.

CREXE's appeal lies in the possibility of bringing the people who use a tool closer to the decisions that shape it. A readable recipe can circulate, receive adjustments, and provide the basis for new experiments. The project remains under development. Its invitation is to explore this workflow with clear criteria: describe, generate, verify, use, and share what has been understood and approved.
