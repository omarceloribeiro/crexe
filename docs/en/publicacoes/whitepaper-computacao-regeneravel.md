# Regenerable computing: a research agenda starting from CREXE

> White paper · vision and proposal · 24/11/2026. Explores long-term possibilities that v1 does not deliver. It is neither a normative specification, an approved standard, nor a product commitment.

## Abstract

Software can be preserved as a binary, source files, or a description of desired behavior. Regenerable computing investigates the relationship between these representations: keeping intent understandable, producing implementations for defined environments, and retaining evidence with which to assess changes.

CREXE offers a limited starting point: readable recipes, source generation, local compilation, revisions, and export. The long-term hypothesis is to extend this workflow to personal tools and some desktop utilities. Doing so would require solving problems of trust, maintenance, functional equivalence, and cost that the existence of a generator alone does not solve.

## An operational definition

In this document, regeneration means producing a new implementation from intent and controlled context, then evaluating the result against acceptance criteria. Recompilation means building existing source files again. Reusing a cache means opening an artifact that has already been preserved. These are different operations, with different costs and risks.

A regenerable environment would need to choose explicitly between them. Reopening a calculator would not justify continuous inference. A new need might motivate editing the recipe; a dependency update might require only recompilation; a demonstrated incompatibility might justify regeneration. This choice should be observable and reversible, without replacing a working version with an unvalidated hypothesis.

## Why preserve intent

Source files describe an implementation with considerable precision, but do not always make the reasons for its decisions apparent. A well-written recipe can record objectives, examples, constraints, and success criteria in a form that users and developers can discuss together.

Imagine an educational tool for proportions. The teacher wants invalid values to be explained, decimal numbers to follow local conventions, and the formula to appear before the result. These rules can accompany the project as revisable intent. Later, a class requests a percentage mode. The challenge then becomes demonstrating that the new version adds this mode without changing the previous rule.

Recipe quality matters. Natural language allows ambiguity and unstated assumptions. Concrete examples, counterexamples, and tests help make the request verifiable. This broadens participation by people without programming backgrounds, but does not remove the need for judgment, learning, or technical support.

## Proposed architecture for a regenerable environment

A future architecture could separate the record of intent, the production of candidates, and authorization to use them. The diagram describes a research proposal, not modules already available in CREXE.

```text
Versioned intent + examples + independent criteria
                          |
            target context and requirements
                          |
               explicit operation choice
               /          |           \
             reuse     recompile    regenerate
               \          |           /
                 identified candidate
                          |
            functional and security checks
                          |
               accept or reject decision
                          |
     controlled deployment + observation + rollback
                          |
             preserved history and evidence
```

Each transition would need to record who initiated it, which inputs were used, which checks took place, and what changed. A model could help formulate or repair the implementation, but should not be the sole authority declaring its own work acceptable.

## Layers and limits of application

The initial field consists of bounded tasks with observable, recoverable failures. The broader a tool's reach, the greater the effort required to demonstrate that regeneration preserves its responsibilities.

| Layer | Possible investigation | Necessary boundary |
|---|---|---|
| Personal tools | Calculators, viewers, and small organizers | Clear acceptance cases and test data |
| Session utilities | Interfaces for user tasks | Minimum permissions, recovery, and state migration |
| Shared components | Libraries and services used by multiple programs | Compatibility contracts and evaluation of consumers |
| Kernel, drivers, and boot chain | Outside this proposal's scope | Would require much stronger evidence and assurance processes |

Excluding critical components defines the research scope; it is not a claim of technical impossibility. Their failure can compromise availability, integrity, or security before ordinary recovery mechanisms become available. This proposal does not recommend automatic experimentation in those layers or changes to safety-critical systems.

Even an apparently simple utility can be risky. Renaming files in bulk affects real data; a calculation tool used for financial decisions can have significant consequences. Scope should account for use and permissions, not just code size.

## Binaries remain part of the system

A precompiled artifact can offer predictable startup, efficient distribution, and a version that many people can test in common. Those benefits remain relevant. A regenerable architecture can combine recipes, approved source files, and identified binaries, allowing the appropriate artifact to be chosen for each situation.

Preserving a known binary can also support recovery when the model, compiler, or a remote service becomes unavailable. A recipe alone does not reconstruct the entire environment that made it executable. Future compatibility depends on tools, dependencies, knowledge of APIs, and criteria for evaluating a new implementation.

## Continuity requires more than prompt history

A tool can accumulate files, preferences, and patterns of use. Regenerating it while preserving only the interface description leaves those commitments out. A mature system would need to treat data formats, migrations, accessibility, error behavior, and integration with other programs as explicit parts of the contract.

History should preserve rejected candidates and relevant causes without unnecessarily exposing private data. Version comparisons would need to show changes in behavior and permissions. Returning to the previous executable is insufficient if the new version has already transformed data incompatibly; application rollback and data recovery are related but distinct problems.

## Evidence, trust, and variability

Caching is not reproducibility. It preserves a result; it does not guarantee that another generation will produce the same source files or that another environment will produce the same binary. Serious research must separate behavioral equivalence, reproduction of source files, and reproduction of compiled bytes.

Signatures and hashes could identify artifact origin and integrity, but would not demonstrate correctness. Dependencies, runtime behavior, and limits of authority would also need evaluation. Tests written by the same generator deserve independent review to reduce the risk that a test confirms the implementation's mistake.

In current v1, the [security model](../SECURITY.md) is execution on the host with the user's permissions. The isolation needed to broaden use with unknown recipes remains an open problem. Any future proposal to restrict network, file, or process access would need to be enforced through real mechanisms and tested against attempts to bypass them.

## Economics of the complete cycle

A short recipe does not automatically make distribution cheaper. Evaluation must include acquiring models, inference processing, SDKs, dependencies, revision storage, human time, and subsequent operation. These costs may be shared across applications or repeated on many machines.

The outcome depends on the scenario. A highly customized tool may justify a slow first generation. Distributing the same stable utility to thousands of machines may favor a common binary. Local compilation does not produce a universal gain in performance, bandwidth, or energy use. Alternatives must be compared using the same tasks and acceptance conditions.

## Research program

Progress can begin with small experiments, expanding the scope only when the evidence justifies the next step.

| Question | Proposed experiment | Decision criterion |
|---|---|---|
| Does intent survive change? | Add requirements to known tools | Independent tests detect regressions in previous behavior |
| Is the result usable? | Observe real tasks on different hosts | Users complete the task with documented requirements |
| Does recovery work? | Interrupt stages and simulate defective candidates | A valid version and its data remain recoverable |
| Is the cost justified? | Compare generation, compilation, and conventional distribution | Complete measurement, including preparation and human effort |
| Does the mechanism respect boundaries? | Evaluate permissions and attempts at unauthorized access | Restrictions demonstrated in the declared environment |

Negative results also need to be published: ambiguous requests, failed migrations, and cases in which human intervention exceeds the expected benefit. Comparisons must separate tests using controlled responses from trials with real models, and first generation from reopening through the cache.

## A possible path

In the short term, useful work means making bounded tools more understandable, editable, and verifiable. Over a longer horizon, communities could maintain intent, examples, and tests alongside approved implementations. Parts of a desktop environment could then evolve through these contracts, preserving conventional alternatives where they are more appropriate.

This path depends on engineering, evaluation, and governance. The [foundations white paper](whitepaper-fundamentos.md) describes the implemented starting point. This vision formulates testable questions about where regeneration offers useful autonomy and where its costs and risks call for another approach.
