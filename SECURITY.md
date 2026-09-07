# Security Policy

> This file exists for **the path GitHub recognises** — a `SECURITY.md` at the
> root is what enables the *"Report a vulnerability"* button. It **does not
> duplicate** the norm; it points at the single source.
>
> **The source is [docs/security.md](docs/security.md), binding from the first
> commit.**

## Reporting a vulnerability

A vulnerability, an exposed secret or a suspected leak: report it to
**samirhv@me.com**, **immediately**. Please do not open a public issue for a
security problem.

You will get an acknowledgement, and a structural fix becomes an ADR in
[docs/decisions.md](docs/decisions.md).

See [docs/security.md §11](docs/security.md#11-incident-response) for the
normative incident-response text, and
[§5](docs/security.md#5-secrets-and-configuration) for why an exposed secret is
**rotated, not reverted** — removing the line from HEAD leaves it in the history
and in every clone.

## Supported versions

<!-- Replace with this project's real support window, or keep the line below if
     there is no released artefact yet. -->
The current `master` is the supported state.

## Context

franknote is a **public** repository, and has been since its first commit.

That makes this file the first point of contact for anyone reporting a problem
from outside the house, and it makes the pre-flight in
[docs/runbook.md §6](docs/runbook.md#6-pre-flight-before-making-a-repository-public)
a standing rule rather than a one-off gate: nothing that assumes privacy — a
secret, an internal hostname, a private IP range — ever goes in, because there is
no later moment at which it would be caught. A secret that reaches a commit here
is public from that push, and the response is to **rotate** it, never to delete
the line.
