# CREXE Threat Model: What Would a CREXE Virus Look Like?

## 1. Framing

This document is conceptual and defensive.

It does not describe how to build malware.

It describes the risks created by generative executable specifications so they can be mitigated.

---

## 2. Traditional malware

A traditional virus typically spreads as:

- binary
- script
- macro
- exploit payload

It executes directly or through an interpreter.

---

## 3. CREXE malware

A CREXE-based malicious artifact would not necessarily distribute a malicious binary.

It could distribute:

- a malicious specification
- a malicious prompt
- malicious build instructions
- malicious generated code

The danger moves upward:

> from binary-level malware to prompt/specification-level malware.

---

## 4. Prompt-level malware

A malicious CREXE might appear to generate a useful app but secretly instruct the model to generate:

- backdoors
- persistence
- file scanning
- credential harvesting
- network calls
- destructive operations

Mitigation:

- strict output contract
- generated code review
- static scanning
- network disabled at run
- command allowlists
- signed recipes

---

## 5. Could it lose control?

A CREXE artifact could become dangerous if a runtime allowed:

- self-modification
- recursive regeneration
- network propagation
- automatic publishing
- unrestricted filesystem access
- arbitrary command execution

A secure runtime should prevent this.

CREXE should be regenerable, but not autonomously self-replicating.

---

## 6. The runtime defines the universe

A CREXE virus is only dangerous to the degree the runtime allows it to act.

A strict runtime is a containment boundary.

A permissive runtime becomes the attack surface.

---

## 7. Defensive requirements

To avoid becoming an AI-powered malware format, CREXE needs:

- zero trust specification handling
- workspace isolation
- no writes outside workspace
- no hidden command execution
- safe generated executable policy
- signed recipes
- inspect mode
- network off by default
- enterprise lockdown mode

---

## 8. Conclusion

CREXE expands software creativity.

It also expands the importance of runtime governance.

Security cannot be added later.

It must be part of the file format culture from the beginning.
