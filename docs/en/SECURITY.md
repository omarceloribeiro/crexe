# Execution limits

CREXE v1, currently under development, executes generated sources and build tools with the user's permissions, in a temporary directory on the host. This **does not provide security isolation**. A malicious program can access the user's files, network, and resources.

The engine's path validation, cache hashes, local credential destinations, response/log limits, and subprocess groups reduce specific failures. They do not make the host safe for untrusted code. The model is not a permissions authority.

The provider, endpoint, and credential variable are selected locally; `.env` is read only when explicitly requested. Known secrets are not inherited by subprocesses. A native program can still search for other user files/configuration; the engine does not promise to prevent that.

Windows Sandbox is prohibited at this stage. Docker/WSL are neither dependencies nor backends. Unimplemented isolation requirements must cause an error, never a false indication of protection.

Report problems with the version and a minimal example using synthetic data. Do not publish credentials, personal data, private caches, or environment dumps in issues. Rotate exposed credentials.
