# Security Policy

## Supported versions

Prism is pre-1.0. Security fixes are applied to the latest `main` and to the most
recent tagged release. Older builds are not maintained; please reproduce issues on
the latest commit before reporting.

| Version | Supported |
| ------- | --------- |
| `main` (latest) | ✅ |
| Latest tagged release | ✅ |
| Older releases | ❌ |

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub's [private security advisory][advisory] flow on this
repository (Security → Advisories → *Report a vulnerability*). If advisories are
unavailable to you, contact the maintainers through the repository's GitHub profile
to arrange a private channel.

Please include:

- affected version or commit, and platform (OS / architecture);
- a minimal reproduction (command, workspace layout, or steps in the desktop app);
- the impact you believe is possible, and any preconditions;
- relevant logs with secrets removed.

## What to expect

This is a small, volunteer-maintained project, so no formal SLA is offered. In
practice:

- acknowledgement of a valid report within a few days;
- an assessment and a remediation plan once the issue is reproduced;
- coordinated disclosure: a fix is prepared before details are published, and you
  are credited unless you prefer otherwise.

## Security model

Prism is a local-first engineering workbench. Understanding this model helps
separate a real vulnerability from expected behaviour.

- **No telemetry, no background networking.** By default Prism performs no outbound
  requests. Deterministic analysis works fully offline and without API keys.
- **Read-only by default.** Analysis reads your project; it does not modify it.
  Snapshot state lives in the OS application-data directory, never inside the project.
- **Explicit, bounded network actions.** HTTP observation (API debugging) and deploy
  are the only networked features. They run only on explicit user intent, are bounded
  in scope, and are gated by an SSRF guard / allowlist.
- **Secret redaction across the chain.** Secret detection results are redacted before
  they reach reports, context packs, or logs.
- **MCP server is read-only.** `prism mcp` speaks newline-delimited JSON-RPC over stdio,
  only reads persisted analysis results from the local SQLite store, never re-runs
  analysis, never reads project files, and never writes data. Protocol output goes to
  stdout; logs go to stderr.
- **Untrusted input.** Repository contents, git metadata, log files, OpenAPI and
  manifest files are treated as untrusted input. Parsing failures are surfaced as
  `Unknown`/`partial` rather than being silently normalized.

### In scope

- Sandbox escapes or unintended file writes outside the app-data directory.
- SSRF / allowlist bypass in API observation or deploy.
- Secret leakage into reports, context packs, logs, SBOM output, or CLI stdout.
- Command injection through discovered verification/manifest entries.
- Path traversal when reading project files, snapshots, or artifacts.
- Denial of service caused by crafted repositories (e.g. unbounded recursion or
  memory growth that bypasses the documented resource limits).

### Out of scope

- Findings produced by Prism about *your* codebase (those are analysis results, not
  vulnerabilities in Prism).
- Running Prism against a repository you do not trust, with a configuration you wrote
  to execute arbitrary commands (verify checks and deploy steps are user-controlled by
  design; review them before running).
- Vulnerabilities in third-party dependencies that have no exploitable path in Prism —
  still welcome, but please note the upstream advisory.
- Missing hardening that does not cross a trust boundary.

[advisory]: https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability
