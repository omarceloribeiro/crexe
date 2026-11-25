# CREXB — Creative Executable Bundle

**Generated once. Opened whenever needed.** CREXB combines a recipe and an accepted application version in a controlled ZIP. On a compatible computer, the engine validates the bundle, prepares its cache, and opens the program without calling AI or recompiling.

**Status: minimal preview on `feature/v1`.** This guide describes the delivered subset and is the English counterpart of the Portuguese guide, **not a full translation of the handoff**. The [complete original handoff](../../formatos/originais/CREXB-HANDOFF-COMPLETO.md) remains available in Portuguese, unchanged. The complete proposal includes future features and earlier licensing discussions. The [current policy](../LICENCIAMENTO_E_ECOSSISTEMA.md) uses MIT for the engine and MIT-0 for designated output materials.

## Share the accepted version

A recipe enables another generation. A bundle also preserves a specific result. You can deliver the same version to a class, a team, or another compatible computer without requiring another generation.

The exporter uses the latest **accepted** revision with the original recipe snapshot recorded by the engine. A failed attempt does not replace that result. Older revisions created before snapshot recording require an explicit regeneration before their first CREXB export. The exporter does not assume that the recipe currently on disk matches the cached binary.

For CREXP, editing the package changes the materialization hash while preserving its revision lineage through the original file path. If a new attempt fails, export still uses the latest accepted revision and the exact recipe recorded with that generation, rather than the new content that failed.

Accepted means the revision passed the pipeline's implemented criteria. It does not mean a complete audit, security certification, or proof that all requested behavior was fulfilled. Evidence must describe the tests actually run, without inventing interface or acceptance validation.

The preview manifest records `applicationStart: not-verified` and `acceptance: not-required`; build and self-tests are not human approval of the application. A bundle includes the original recipe, a resolved representation, sources, a reference to self-tests in the sources, evidence, and payload. `tests/README.md` identifies `--crexe-self-test` tests within `source/`, without inventing an additional suite. Opening a bundle does not repeat those tests.

## Preview scope

The minimal bundle has one platform and architecture per file, a manifest, preserved recipe, payload, launch metadata, and hashes. The preview supports `cpp-win32`, `objc-cocoa`, `c-gtk`, `dotnet-console`, and `dotnet-winforms`, using an apphost for .NET 8 profiles. OpenGL and custom YAML recipe commands are outside this first subset.

The receiving computer needs the engine and the application's runtime dependencies. A framework-dependent .NET app requires a compatible .NET runtime; a native program may require system libraries. Opening the bundle does not require a build SDK or an AI provider. This does not automatically make the payload universal or self-contained.

## Export, verify, and open

After generating and accepting a revision, from the checkout root:

```powershell
.\target\release\crexe.exe bundle create calculator.crexe --target current --output calculator.crexb
.\target\release\crexe.exe bundle inspect calculator.crexb
.\target\release\crexe.exe bundle verify calculator.crexb
.\target\release\crexe.exe bundle run calculator.crexb
```

On macOS/Linux, use `./target/release/crexe`. For an engine distribution, use the executable in its extracted directory. `bundle create` accepts `--cache-dir` to locate the revision; opening uses the engine's default root, respecting `CREXE_HOME`. Creation uses an already accepted revision; it does not silently generate an application to fill a missing cache.

The execution flow also recognizes `.crexb` and bundles renamed to `.crexe` by their contents. The manifest at `META-INF/crexe-bundle.json` distinguishes CREXB from a CREXP SDD package. A ZIP without a recognized marker, or with conflicting markers, is rejected.

A file named `.crexb` without a bundle manifest is rejected; it never falls back to generation. On opening, the enclosed recipe remains inert: its bytes and hash are checked without consolidating the SDD documents again.

On Windows, `install` and `associate` also register `.crexp` and `.crexb` for double-click opening. The text editor does not edit these ZIPs, and there is no global `.zip` association. To only verify a bundle, use `bundle verify` or the normal execution flow with `--no-run`; that mode neither hydrates nor executes the payload.

## Cache and application data

Hydration verifies files and prepares the payload at `bundles/objects/sha256/<ZIP-hash>` under the configured cache. A valid cache can be reused without another generation. Mutable data stays separate, at `apps/<id-hash>/{data,config,state}` under the engine's installation root; concrete paths depend on configuration and platform. The process receives these directories through `CREXE_*` variables and uses the data directory as its working directory.

In this preview, bundle identity derives from the original path of the recipe file used to produce it, not from `package.id`. Moving or renaming that recipe before producing another bundle may create a different data identity. Stable public identity and data migration belong to the format's evolution.

An application should not treat its binary directory as data storage. Assets should be located relative to the executable's base, without assuming that the working directory is the payload directory. This separates distributed files from data created during use.

Incomplete or modified bundle caches, and caches containing extra files, are blocked. The preview does not silently delete them: explicitly removing only that cache object permits a new hydration while preserving separate application data. The exporter also rejects revisions containing files not listed in their inventory.

The bundle lock remains held while the application runs. A second simultaneous opening of the same bundle is rejected until the first process exits; multiple-instance management belongs to a future iteration.

## Compatibility and explicit regeneration

A Windows payload is not run as if it were Linux. The first version requires a compatible target; it does not emulate platforms or install runtimes automatically. Incompatibility blocks opening and explains why.

To recover the included recipe, use explicit extraction:

```powershell
.\target\release\crexe.exe bundle extract-recipe calculator.crexb --output recovered.crexe
.\target\release\crexe.exe inspect recovered.crexe
.\target\release\crexe.exe exec recovered.crexe
```

Choose the output extension according to the recipe type reported by inspection (`.crexe` or `.crexp`). The last line starts a normal new generation: it may call the model, use the network, and requires SDKs. The original bundle is unchanged. Guided regeneration, multiple targets, and advanced runtime selection remain on the roadmap.

## Integrity and trust

Hashes detect accidental changes and inconsistencies in declared content. **They do not authenticate the author:** someone changing all files can also recompute the hashes. The preview does not verify signatures and rejects bundles that require an unsupported signature.

The initial subset requires portable ASCII internal paths, at most 1,024 entries, individual files up to 128 MiB, and a ZIP up to 256 MiB. Launch accepts the profile entrypoint without custom arguments. Bundles containing signatures are also rejected in this preview, instead of being opened with a false indication of verification.

CREXB contains executable software. Including the recipe does not make it safe, and the current engine does not sandbox the application. Run only content you trust. ZIP and path checks do not replace the [security limits](../SECURITY.md).

## Distribution and privacy

Sharing a bundle also shares its recipe. Review included documents and files before distributing it. Secret scanning, automatic log redaction, SBOM, signing, and publisher verification are future proposals.

You choose the application's license, including a proprietary one, subject to the licenses of included recipes, dependencies, and components. The engine's MIT license and output materials' MIT-0 grant do not replace those third-party conditions.

## Complete proposal downloads

- [Complete CREXB handoff — original Portuguese Markdown](../../formatos/originais/CREXB-HANDOFF-COMPLETO.md).
- [Original manifest v1 JSON Schema](../../formatos/originais/crexb-manifest-v1.schema.json).
- [Original example manifest](../../formatos/originais/crexb-manifest.example.json).
- [SHA-256 checksums for all four originals](../../formatos/originais/checksums.sha256).

**The example manifest uses placeholders and illustrative references. It is not an executable CREXB application and does not contain a validated build.** The schema is a draft of the complete proposal, not a promise that all combinations it accepts work in the preview. The [roadmap](roadmap-formatos.md) identifies the remaining stages. The site's language selector offers the complete maintained guide in English and Portuguese; the complete handoff remains in its original Portuguese.
