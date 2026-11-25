# Software as RNA: a metaphor for intent in motion

> Essay • editorial perspective • 24/11/2026

A software recipe occupies an interesting position. It can be stored as a document, shared as an idea, and used to produce a program. Its value does not end with reading: there is a transition between what it describes and what a machine will do. The image of RNA helps us think about that transition, provided we accept the comparison's limits.

This essay uses biology as a metaphor, not as a scientific explanation of CREXE. Software is not alive, a generation model is not a cell, and configuring a platform is not an epigenetic process. Nor do we assume an unchanging DNA reduced to a file of instructions. Biology has mechanisms and relationships that this image deliberately does not attempt to reproduce.

The point of comparison is more modest: a message can participate in producing something that performs a function. For messenger RNA, the reference is its role in protein production. For CREXE, it is the transformation of a readable recipe into source files and then into an executable application.

## From message to operation

We can organize the image as follows:

| Element of the metaphor | Approximate counterpart in CREXE |
| --- | --- |
| A message participating in production | A `.crexe` recipe containing the application request |
| Machinery that processes the message | Engine, generator, and compilation tools |
| Conditions in which the process occurs | System, architecture, profile, SDKs, and local configuration |
| A product capable of performing a function | The application that is produced and executed |

These correspondences help visualize a process without establishing literal equivalences. The engine coordinates stages that depend on different tools. The generator interprets a request, the compiler works on source files, and the application executes instructions. Gathering all of this under the image of machinery highlights the workflow, but should not obscure each part's responsibility.

A recipe can request a proportion calculator. The system needs to turn that description into inputs, operations, error messages, and an interface. The environment limits the possible choices. A profile requires specific tools; an unavailable capability needs to be identified. Intent does not create a missing SDK or make every platform compatible.

## The environment participates in the result

In CREXE's proposal, a request can circulate without carrying the executable someone has already produced. The recipient prepares their own implementation under the available conditions. This lets us think of the file as a portable message, while the concrete realization remains tied to a context.

That connection is not a promise of universal adaptation. The current implementation works with a catalog of known profiles and requirements. Support and validation have limits, described in the [profile matrix](../PERFIS_V1.md). A recipe may require resources that the environment does not offer or that the generator cannot implement satisfactorily.

Even with the same request, two generations can diverge. That variability is part of the problem to manage. The cache preserves an approved revision for reuse under the expected conditions; it does not demonstrate that a future generation will repeat that exact implementation. When preserving particular source files matters, exporting the project provides a different kind of record.

## Where the analogy reaches its limit

A natural-language recipe contains ambiguities and human choices. It can omit a case, contradict another requirement, or express a need poorly. The resulting program does not reveal an essence hidden in the file: it is one possible implementation, built from the request, context, and tools used.

Calling an edit a “mutation” therefore adds little unless we say what changed and why. In software work, we can formulate goals and review decisions explicitly. One person requests new controls, another proposes a correction, and someone decides to preserve a version. There is authorship, collaboration, and responsibility; we do not need to attribute will to the file or system.

The same caution applies to “evolution.” A sequence of versions does not improve automatically. It can accumulate errors, lose clarity, or abandon necessary behavior. A different result is better only in relation to criteria that someone can explain and verify. The metaphor should serve that discussion without turning novelty into proof of progress.

## Selection requires testing and observation

For the proportion calculator, a simple criterion would be to verify that A equal to 2, B equal to 4, and C equal to 3 produce X equal to 6. Other cases need to examine invalid inputs, undefined operations, and interaction with the interface. Asking the program to work is only the beginning of defining what will be accepted.

CREXE compiles and runs tests according to the profile before approving a revision. Those tests are part of the process but may be produced by the same model that wrote the source files. They therefore do not replace independent evaluation or establish all requested behavior. History preserves an approved revision when a later attempt fails; people must still decide whether the delivered version meets its intended use.

It would also be inappropriate to treat the engine as a protective membrane. The current version runs tools and applications on the host with the user's permissions. The temporary directory organizes the work without isolating network access, files, or processes. The [security documentation](../SECURITY.md) describes this condition directly. No metaphor should soften it.

## A useful image as long as it stays bounded

Thinking of software as RNA draws attention to the transition from description to operation and to the environment's participation. It can inspire clearer recipes, better records, and tools that make a transformation easier to review. It does not, by itself, explain compatibility, authorship, quality, or trust.

CREXE explores a way to keep intent available while working with a program. Its contribution will be measured by what this workflow allows people to do and verify. The biological image can open the conversation; accessible source files, explicit requirements, tests, and human decisions must sustain it.
