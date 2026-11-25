# Roadmap — CREXP and CREXB

**Edition: 25 November 2026.** Minimal previews are part of development on `feature/v1`; they do not mean that the entire handoff architecture has shipped or that a stable public release already exists. This roadmap does not assign dates to later stages.

CREXE lets you describe. CREXP lets you organize specifications. CREXB lets you distribute and open an accepted result. These roles complement one another: the recipe preserves intent, the package organizes that intent, and the bundle also preserves an implementation.

## What fits in the initial delivery

| Area | Minimal preview | Explicit limit |
| --- | --- | --- |
| CREXP | Multiple Markdown files in a ZIP, optional minimal manifest, inspection, and deterministic consolidation | No assets, agents, or semantic conflict resolution |
| CREXP generation | Reuses the normal pipeline, SDKs, and retry budget | SDD is sent to the configured provider; this is not the reader generator's privacy contract |
| CREXB | Creates, inspects, verifies, caches, and opens an accepted snapshot | One target per bundle, a profile subset, and a previously available runtime |
| Recipe in CREXB | Exact original recorded at generation, explicit extraction for regeneration | Older caches without a snapshot need explicit regeneration before export |
| Evidence | Actual pipeline build and self-tests | Does not claim human acceptance or visual application validation |
| Integrity | Validated ZIP, controlled paths, and hashes before launch | Hashes do not authenticate a publisher; required signatures are rejected |
| Storage | Cached payload and separate mutable data | Does not sandbox the application |
| Publication | PT/EN guides and complete downloadable originals | Original handoffs are in Portuguese; English guides are not full handoff translations |

The maintained [CREXP](crexp.md) and [CREXB](crexb.md) guides describe commands, limits, and prerequisites. An accepted snapshot is a technical pipeline state, not a trust seal.

## Complete CREXP

| Stage | Planned work | Completion criterion |
| --- | --- | --- |
| Documents and manifest | Rich roles, inventory, assets, seed code, contracts, policies, and versioned capabilities | Explicit supported files; unknown capabilities are not silently ignored |
| Recipe Compiler | Document graph, precedence, conflicts, and `recipe.resolved.yaml` | Traceable decisions and conflicts; relevant ambiguities block or require explicit resolution |
| Reproducibility | Normalized hashes, generation plan, lock file, resolver and toolchain fingerprints | Distinguish interpretation, generation, build, and acceptance; do not assume bitwise identity |
| Agentic generation | Local/remote agents, tools, build/test/repair loops, and budgets | Verified time, cost, network, and permission limits; package instructions cannot elevate privileges |
| Resolvers and targets | Adaptation to additional environments and platforms | Tested support matrix with real examples; hypothetical systems are not advertised as supported |
| Publishing | Folder publishing and future cloud destinations configured externally | Explicit operations, credentials outside packages, verifiable artifacts and reports |
| Observability and trust | Reports, local traces, policies, signing, and interoperability | Clarity about data leaving the computer, verifiable origins, and compatibility tests |

## Complete CREXB

| Stage | Planned work | Completion criterion |
| --- | --- | --- |
| Export experience | Export interface, source/test/log options, and acceptance policies | Selected generation exported with actual evidence; failed attempts do not masquerade as accepted builds |
| Recipe and regeneration | Embedded CREXP, inventory, plan, guided inspection, and generation for another host | Preserved recipe; explicit regeneration with visible costs and prerequisites |
| Runtimes and compatibility | Self-contained payloads where feasible, verified dependencies, and guided repair | Respect dependency redistribution terms; never silently execute an incompatible target |
| Distribution trust | Signatures, publishers, SBOM, notices, scanning, and provenance | Distinguish integrity from authenticity; actually enforce signature policies |
| Export privacy | Secret scanning, evidence selection, and controlled redaction | No silent disclosure of private data or loss of recipe-to-build linkage |
| Multiple targets | Several payloads and selection of the most appropriate one | Demonstrated compatibility on each host; extraction and validation limited to authorized content |
| Versions and updates | Indexes, channels, rollback, migrations, cache collection, and possible deltas | Cache cleanup does not erase user data; updates do not promote incomplete content |
| Concurrency and scale | Locks, transactions, retention policies, and running-bundle management | Simultaneous launches and intermediate failures do not corrupt cache or state |

## What remains research

Deep operating system integration, historical platforms, virtual mounting, partial/streaming execution, and bitwise reproduction across different toolchains remain research hypotheses. A Gallery, commercial services, and cloud publishing may be planned without being available products in this version. This site does not include a commercial landing page.

There is no promise that a generated program is safe merely because it is packaged, that hashes prove authorship, or that every computer is compatible. Programs run on the host with the user's permissions. Models, SDKs, and dependencies still matter.

## Complete, unabridged documents

- [CREXP — complete original Portuguese handoff](../../formatos/originais/CREXP-HANDOFF-COMPLETO.md).
- [CREXB — complete original Portuguese handoff](../../formatos/originais/CREXB-HANDOFF-COMPLETO.md).
- [CREXB — original manifest v1 JSON Schema](../../formatos/originais/crexb-manifest-v1.schema.json).
- [CREXB — original illustrative manifest](../../formatos/originais/crexb-manifest.example.json).
- [Original SHA-256 checksums](../../formatos/originais/checksums.sha256).

All four files were preserved byte for byte. The schema and manifest illustrate the proposal; the example uses placeholders and **is not an executable application**. The proposed v1 manifest allows possibilities beyond the preview subset. The handoffs retain earlier licensing discussions; the [current policy](../LICENCIAMENTO_E_ECOSSISTEMA.md) is MIT for the engine and MIT-0 for designated output materials. This roadmap and the other English guides are full counterparts of the maintained Portuguese guides, not full translations of the original Portuguese handoffs. Use the site's language selector to switch guide editions.
