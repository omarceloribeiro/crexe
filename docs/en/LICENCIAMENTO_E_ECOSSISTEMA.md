# Licensing and organization of the CREXE ecosystem

Review dated 24 November 2026 of the licensing plan dated 14 October 2026 and the current implementation. The retained decision is **GNU GPL v3 for the engine**. This record distinguishes what already exists from proposed formal arrangements; it does not constitute a new license, trademark policy, or legal opinion.

## Decision and current status

Separating the open project from commercial products is consistent with CREXE's direction. There is no need to change the Core license to organize these two spaces.

| Item | Status in the repository |
| --- | --- |
| Engine/CLI | GNU GPL v3 text in [LICENSE](../../LICENSE), referenced through `license-file` in [Cargo.toml](../../Cargo.toml). |
| Future GPL versions | The plan proposes `GPL-3.0-or-later`; the current README does not add that permission. The variant must be explicitly settled before changing metadata and notices. |
| Templates and examples | No specific MIT/Apache grant implementing the proposed matrix exists yet. |
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

Using a GPL tool does not automatically make its output GPL. Code incorporated from the tool itself, templates, dependencies, and assets may impose specific conditions. Exporting sources therefore does not amount to granting unrestricted rights over all content. Reference: [GNU FAQ on program output](https://www.gnu.org/licenses/gpl-faq.html#GPLOutput).

Communication through a CLI, pipes, or IPC often indicates separate programs, but is not a legal guarantee: coupling and the data exchanged also matter. A proprietary integration must be assessed in its concrete circumstances. Reference: [GNU FAQ on aggregation](https://www.gnu.org/licenses/gpl-faq.html#MereAggregation).

The GPL permits commercial activity. Its source-provision mechanism is tied to distribution conditions; it does not contain the AGPL's network-interaction obligation. This does not remove the need to assess distribution of a commercial product's components. References: [GNU GPL v3, sections 2 and 6](https://www.gnu.org/licenses/gpl-3.0.html), [GNU FAQ on modified versions](https://www.gnu.org/licenses/gpl-faq.html#UnreleasedMods).

### Concrete outstanding item: exported templates

The plan calls for permissive templates and a license inventory, but that separation still needs to be implemented:

| Content | Current origin |
| --- | --- |
| `.csproj` project inserted into the app | [src/profiles.rs](../../src/profiles.rs), .NET project creation/preparation functions. |
| GTK profile Makefile | [src/profiles/native.rs](../../src/profiles/native.rs). |
| Exported build/run/test/publish scripts and README | [src/package.rs](../../src/package.rs). |
| Project and generation manifests | [src/project.rs](../../src/project.rs) and [src/history.rs](../../src/history.rs); no template license inventory or third-party notices yet. |

Before promising permissive use of templates, inventory those sections, confirm ownership, select the applicable grant, separate their files/notices, and include the required licenses in exports. The current package does not automatically create a validated license for the application. This does not justify concluding that all output is GPL; it prevents claiming that the plan's permissive release has already been applied.

Official recipes also need an explicit grant before they can be advertised as MIT. The [engine packager](../../scripts/package-engine.py) currently bundles examples and the engine license, without a separate MIT license for recipes.

## Format, identity, and trademark

It is appropriate to distinguish interoperability with `.crexe` from presentation as an official product. GPL v3 permits reservations of trademark rights and reasonable requirements to identify modifications, while disallowing incompatible additional restrictions on the code. Reference: [GNU GPL v3, section 7](https://www.gnu.org/licenses/gpl-3.0.html#section7).

A future policy should preserve truthful descriptive references, such as “supports .crexe files,” and technical protocol names. `CREXE-PROJECT.json`, `CrexeApp.csproj`, or an indication of the tool used should not automatically be confused with a product's primary branding.

Publishing a policy alone should not be presented as establishing a universal prohibition on names, colors, domains, or commercial uses. The extent and limits of protection depend on applicable rights. In Brazil, registration, territory, and protected goods/services are relevant. The status of the CREXE trademark was not verified in this review. Reference: [INPI — trademark frequently asked questions](https://www.gov.br/inpi/pt-br/acesso-a-informacao/perguntas-frequentes/marcas).

## Additional licenses and contributions

Apache 2.0 for future SDKs/protocols and CC BY 4.0 for documentation are options compatible with the proposed organization, provided they are applied to defined materials by parties holding the necessary rights. CC BY should not replace a software license. References: [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0.html), [Creative Commons — software and documentation](https://creativecommons.org/faq/#can-i-apply-a-creative-commons-license-to-software).

A CLA is not a general requirement for maintaining a GPL project. Alternative commercial licensing requires control of sufficient rights over all involved parts; a CLA or assignment can be an instrument for that purpose. This record does not establish an active CLA. Reference: [Open Source Initiative — contribution agreements](https://opensource.org/faq).

Before turning the plan into binding policies, settle the GPL variant, ownership, template/documentation grants, contribution rights, and trademark rules. Contractual and trademark formalization should receive legal review. Meanwhile, the public presentation describes the existing license and the capabilities actually implemented.
