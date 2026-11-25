# Licensing and organization of the CREXE ecosystem

Policy updated on **25 November 2026**. The CREXE engine uses **MIT**. Materials CREXE incorporates into output and official examples have an additional **MIT-0** grant. **Users choose the generated application's license, including a proprietary license.** [LICENSE](../../LICENSE) and [LICENSE-GENERATED](../../LICENSE-GENERATED) define the conditions and scope of these permissions.

## Current licenses

| Material | License and scope |
| --- | --- |
| Engine/CLI and its implementations | MIT in [LICENSE](../../LICENSE), identified in [Cargo.toml](../../Cargo.toml). Permits use, modification, distribution, sublicensing and sale, while preserving the notices MIT requires. |
| Output materials and official examples | MIT-0 within the scope of [LICENSE-GENERATED](../../LICENSE-GENERATED). Does not require attribution or retention of the notice for these materials. |
| Generated application | License chosen by the user, respecting the rights and terms of any third-party components. CREXE does not automatically assign a license to the whole application. |
| Third-party dependencies, assets, recipes and other materials | Keep their own licenses and conditions; they are not relicensed by CREXE's MIT or MIT-0. |
| Historical documents and supplied originals | Preserve their notices and context. Earlier licensing proposals do not define the engine's current policy. |

MIT allows the community to port the engine, develop forks, integrate its code into other products and distribute open or proprietary versions. It does not require publishing modifications, but requires preserving the copyright and permission notices in copies or substantial portions of the software. The license does not automatically transfer ownership to the maintainer.

Maintained documentation describes this policy. This document does not establish a separate CC BY or Apache license; consult the notices applicable to each material, particularly in the historical collection.

## Generated applications and MIT-0 materials

Users who generate an application with CREXE may use, modify, sell and distribute their program under their chosen license, including a closed/proprietary license. The license of CREXE-provided materials does not require publishing source code, paying royalties or crediting CREXE. The [MIT No Attribution (MIT-0)](https://spdx.org/licenses/MIT-0.html) grant permits sublicensing and selling the designated materials without requiring attribution or retention of their notice.

This grant covers CREXE-authored portions of the following materials, their copies and modifications, as defined in `LICENSE-GENERATED`. It does not grant rights over all user/model output or determine its copyright status. The Rust implementations that produce these materials use MIT; the MIT-0 grant is specific to the emitted materials and designated examples.

| Content | Current origin |
| --- | --- |
| `.csproj` project inserted into the app | [src/profiles.rs](../../src/profiles.rs), .NET project creation/preparation functions. |
| GTK profile Makefile | [src/profiles/native.rs](../../src/profiles/native.rs). |
| Exported build/run/test/publish scripts and README | [src/package.rs](../../src/package.rs). |
| Project and generation manifest structures and text | [src/project.rs](../../src/project.rs) and [src/history.rs](../../src/history.rs). The generation manifest remains in the local revision; it is not included in the source ZIP. |
| Instructions and contracts incorporated into output and generated recipes | [src/reader.rs](../../src/reader.rs), profiles, [src/orchestration.rs](../../src/orchestration.rs) and [src/generator.rs](../../src/generator.rs), only their emitted materials. |
| Official recipes and packaged example documents | The `examples/` directory and example text in [scripts/package-engine.py](../../scripts/package-engine.py). |

The project and its ZIP receive `CREXE-OUTPUT-LICENSE.txt`, a copy of `LICENSE-GENERATED`. The notice explains the grant for CREXE materials and may be removed under MIT-0. The exporter preserves a user-provided `LICENSE`; it does not assign a license to the whole application. Exporting an older revision also includes the current notice without regenerating the application or changing its cached revision.

Distributing the engine itself or incorporating parts of its implementation requires preserving the applicable MIT notices. Permission to remove the notice for MIT-0 materials does not extend to the engine or waive notices required by third parties. Engine packages include `LICENSE` and `LICENSE-GENERATED` to distinguish these scopes.

## Third-party rights

Third-party code, libraries, assets and recipes retain their own conditions, as do the terms of the model/provider used. Users choose the application's license while respecting those terms; CREXE cannot grant rights held by others. The exporter does not automatically audit those licenses.

For example, OpenTK and its dependencies in the .NET OpenGL profile, GTK/libepoxy, Apple frameworks and compiler runtimes retain their own licenses and distribution conditions. Building or exporting does not turn these components into CREXE MIT-0 code.

Native engine packages include `THIRD-PARTY-NOTICES.txt` and `third-party-licenses/`, with license texts, origins, versions and hashes. Collection uses Cargo metadata filtered for the host and conservatively includes build and development dependencies. Version-pinned supplementary texts in `scripts/third_party_licenses/` and their associated mapping preserve notices missing from upstream packages. This documentary inventory does not claim that every listed component is embedded in the binary or certify complete legal compliance.

## Contributions

Engine contributions are made under MIT. Contributions to the output materials and examples designated in `LICENSE-GENERATED` are also made under MIT-0, in addition to the repository license, unless explicitly agreed otherwise. This additional permission is necessary to preserve generated applications' licensing freedom; the engine's MIT license alone does not remove its notice requirement.

The license does not automatically transfer ownership of contributions to the maintainer. The project does not require a copyright assignment or CLA as a general condition of contribution. Contributors must hold the necessary rights to offer material on these terms, identify third-party content and preserve applicable notices. See the [contribution guide](CONTRIBUTING.md).

## History and earlier versions

The engine used GNU GPL v3 before this decision. The current policy prioritizes adoption, ports and integration of the implementation under MIT, while retaining MIT-0 for output materials. The change does not rewrite earlier commits, tags, packages or their licenses, and does not revoke permissions already granted. Consult the license accompanying each version; this document does not promise to retroactively relicense third-party contributions.

RFCs, original handoffs and files in `docs/archive` remain historical records. Earlier proposals about products or licenses do not replace `LICENSE`, `LICENSE-GENERATED` and this current policy. GPL references in the [v1 plan's assessment](../PLANO_V1.md) record the earlier state. The [editorial library](publicacoes/README.md) distinguishes originals from maintained publications.

## Format, identity and future products

Interoperability with `.crexe` and independent format implementations are welcome. Using the format, porting the engine or choosing an application's license does not authorize presenting a product as an official CREXE product. The software license does not transfer trademark rights; truthful descriptive references such as “supports .crexe files” should be distinguished from official branding. This document does not claim that the trademark is registered or establish a separate trademark policy.

`crexe.org` brings together the open project, downloads, documentation, RFCs and examples. Future commercial products and services may be organized in other repositories and websites without being presented as features available in this version. That organization does not change the licenses of the components used.
