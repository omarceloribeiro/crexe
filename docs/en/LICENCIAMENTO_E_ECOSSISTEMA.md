# Licensing and organization of the CREXE ecosystem

Review dated 25 November 2026. The engine remains under **GNU GPL v3**. **Users choose the license of the application they generate, including a proprietary license.** CREXE-provided output materials and official recipes receive the **MIT-0** grant in [LICENSE-GENERATED](../../LICENSE-GENERATED). This document explains that separation and future proposals; it does not replace the license texts or constitute legal advice.

## Decision and current status

Separating the open project from commercial products is consistent with CREXE's direction. There is no need to change the Core license to organize these two spaces.

| Item | Status in the repository |
| --- | --- |
| Engine/CLI | GNU GPL v3 text in [LICENSE](../../LICENSE), referenced through `license-file` in [Cargo.toml](../../Cargo.toml). |
| Future GPL versions | The plan proposes `GPL-3.0-or-later`; the current README does not add that permission. The variant must be explicitly settled before changing metadata and notices. |
| Output materials and official examples | Explicit MIT-0 grant in [LICENSE-GENERATED](../../LICENSE-GENERATED), with a defined scope. No license is automatically assigned to the application. |
| Documentation and site code | This review does not apply CC BY, MIT, or Apache to existing files. |
| Protocols, external SDKs, `.crexp`, and certification | Future directions, not functionality or policies delivered in this version. |
| Trademark and ownership | The plan presents Bestows Solutions Ltda. as the proposed owner. This review does not establish an assignment of rights or trademark registration. |

“GPL v3” and “GPL v3 or later” are not equivalent identifiers. The option to use later versions depends on the notice applicable to the program; the application example at the end of the license text does not replace that notice. Therefore, `LICENSE`, `Cargo.toml`, and the current README wording are preserved. Reference: [GNU GPL v3, section 14](https://www.gnu.org/licenses/gpl-3.0.html#section14).

## Website structure

| Space | Content and role |
| --- | --- |
| `crexe.org` | Open project: introduction, engine/CLI, downloads, implemented formats, RFCs, documentation, examples, contribution, and security. Governance and formal policies will be added when defined. |
| `crexe.bestows.solutions` | Planned destination for Bestows commercial products and services. Studio, Education, Cloud, Gallery, Enterprise, support, and certification remain proposals until separately announced and delivered. |

The public landing page stays focused on understanding, trying, and downloading the project. It should not advertise future products as available, imply an independent foundation, or present policies in preparation as active agreements. The domain split is editorial and institutional: by itself, it does not determine the license of each implementation.

There is no need to create all planned repositories before their components exist. When multiple licenses are involved, boundaries between files, dependencies, and exported content must be clear, whether in directories within the same repository or in separate repositories.

## Engine, generated programs, and separate products

Users who generate an application with CREXE may use, modify, sell and distribute their program under their chosen license, including a closed/proprietary license. CREXE does not require source publication, royalties or attribution to CREXE. Using a GPL tool does not automatically make its output GPL. Reference: [GNU GPL v3, section 2](https://www.gnu.org/licenses/gpl-3.0.html#section2).

To give the engine's own templates that same freedom, its materials incorporated into output are offered under [MIT No Attribution (MIT-0)](https://spdx.org/licenses/MIT-0.html), within the scope of [LICENSE-GENERATED](../../LICENSE-GENERATED). This license permits sublicensing and selling without requiring attribution or retention of the notice. The grant also covers those materials in already generated applications; it does not change the engine's license.

Third-party code, libraries, assets and recipes retain their own terms, as do the terms of the model/provider used. Users choose the application's license while respecting those terms. CREXE neither claims ownership of output nor determines its copyright status, and cannot grant rights held by others. The exporter does not automatically audit those licenses.

Communication through a CLI, pipes, or IPC often indicates separate programs, but is not a legal guarantee: coupling and the data exchanged also matter. A proprietary integration must be assessed in its concrete circumstances. Reference: [GNU FAQ on aggregation](https://www.gnu.org/licenses/gpl-faq.html#MereAggregation).

The GPL permits commercial activity. Its source-provision mechanism is tied to distribution conditions; it does not contain the AGPL's network-interaction obligation. This does not remove the need to assess distribution of a commercial product's components. References: [GNU GPL v3, sections 2 and 6](https://www.gnu.org/licenses/gpl-3.0.html), [GNU FAQ on modified versions](https://www.gnu.org/licenses/gpl-faq.html#UnreleasedMods).

### Output materials licensed under MIT-0

The grant covers CREXE-authored portions of the following materials, their copies and modifications. The Rust implementations that produce them remain GPLv3:

| Content | Current origin |
| --- | --- |
| `.csproj` project inserted into the app | [src/profiles.rs](../../src/profiles.rs), .NET project creation/preparation functions. |
| GTK profile Makefile | [src/profiles/native.rs](../../src/profiles/native.rs). |
| Exported build/run/test/publish scripts and README | [src/package.rs](../../src/package.rs). |
| Project and generation manifest structures and text | [src/project.rs](../../src/project.rs) and [src/history.rs](../../src/history.rs). The generation manifest remains in the local revision; it is not included in the source ZIP. |
| Instructions and contracts incorporated into output and generated recipes | [src/reader.rs](../../src/reader.rs), profiles, [src/orchestration.rs](../../src/orchestration.rs) and [src/generator.rs](../../src/generator.rs), only their emitted materials. |
| Official recipes and packaged example documents | The `examples/` directory and example text in [scripts/package-engine.py](../../scripts/package-engine.py). |

The project and its ZIP receive `CREXE-OUTPUT-LICENSE.txt`, a copy of `LICENSE-GENERATED`. The notice identifies the grant for CREXE materials and may be removed under MIT-0. The exporter preserves a user-provided `LICENSE`; it does not assign a license to the whole application. Exporting an older revision also includes the current notice without regenerating the application or changing its cached revision.

Engine packages include both `LICENSE` (the engine's GPLv3) and `LICENSE-GENERATED` (MIT-0 for the designated materials and official examples). This grant does not automatically cover all documentation, conformance tests or independent format implementations.

Dependencies remain outside this grant. For example, OpenTK and its dependencies in the .NET OpenGL profile, GTK/libepoxy, Apple frameworks and compiler runtimes retain their own licenses and distribution conditions. Building or exporting does not turn these components into CREXE MIT-0 code.

## Format, identity, and trademark

It is appropriate to distinguish interoperability with `.crexe` from presentation as an official product. GPL v3 permits reservations of trademark rights and reasonable requirements to identify modifications, while disallowing incompatible additional restrictions on the code. Reference: [GNU GPL v3, section 7](https://www.gnu.org/licenses/gpl-3.0.html#section7).

A future policy should preserve truthful descriptive references, such as “supports .crexe files,” and technical protocol names. `CREXE-PROJECT.json`, `CrexeApp.csproj`, or an indication of the tool used should not automatically be confused with a product's primary branding.

Publishing a policy alone should not be presented as establishing a universal prohibition on names, colors, domains, or commercial uses. The extent and limits of protection depend on applicable rights. In Brazil, registration, territory, and protected goods/services are relevant. The status of the CREXE trademark was not verified in this review. Reference: [INPI — trademark frequently asked questions](https://www.gov.br/inpi/pt-br/acesso-a-informacao/perguntas-frequentes/marcas).

## Additional licenses and contributions

Apache 2.0 for future SDKs/protocols and CC BY 4.0 for documentation are options compatible with the proposed organization, provided they are applied to defined materials by parties holding the necessary rights. CC BY should not replace a software license. References: [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0.html), [Creative Commons — software and documentation](https://creativecommons.org/faq/#can-i-apply-a-creative-commons-license-to-software).

A CLA is not a general requirement for maintaining a GPL project. Alternative commercial licensing requires control of sufficient rights over all involved parts; a CLA or assignment can be an instrument for that purpose. This record does not establish an active CLA. Reference: [Open Source Initiative — contribution agreements](https://opensource.org/faq).

The MIT-0 scope for output materials and examples is explicit in `LICENSE-GENERATED`; new contributions to those materials must respect that grant. Future decisions about the GPL variant, corporate ownership, documentation, contribution rights and trademark rules remain separate. Contractual and trademark formalization should receive legal review. The public presentation describes the existing licenses and capabilities actually implemented.
