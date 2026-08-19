# Security Policy

This repository holds the XION treasury contract — the **Treasury** asset in the
[Core Protocol Contracts bug bounty program](https://github.com/burnt-labs/bug-bounty/blob/main/programs/contracts.md).
That program's terms govern scope, proof of concept requirements, severity
classification, and reward eligibility for findings in this contract — including
the fee grant scope rules that determine when a grant issuance finding is
eligible.

This policy supplements the
[organization-wide policy](https://github.com/burnt-labs/.github/blob/main/SECURITY.md),
which governs anything not addressed here or by the program.

## Reporting a Vulnerability

**Do not open a public GitHub issue for a security vulnerability.**

| Type of finding                  | How to report                                                       |
| -------------------------------- | ------------------------------------------------------------------- |
| Security vulnerability           | **Security → Report a vulnerability** on this repository, or email [security@burnt.com](mailto:security@burnt.com) |
| Non-sensitive or operational bug | Open a GitHub issue on this repository                              |

Prefer GitHub private vulnerability reporting where possible: the fix is
developed against the report, and you are credited on the published advisory
and in any CVE we request.

Include the type of vulnerability, affected version, steps to reproduce,
impact, how an attacker would exploit it, and any known mitigations.

We acknowledge receipt within **5 business days** and provide a triage decision
within **14 days**.

## Proof of Concept

**An end-to-end proof of concept is required.** Tests that mock contract state
or bypass CosmWasm message routing — including `cw-multi-test` environments —
do not demonstrate exploitability on their own. Run the proof of concept
against a locally running XION node configured with mainnet parameters, using
the governance-deployed contract bytecode, and execute the attack via standard
transaction broadcast. The
[program document](https://github.com/burnt-labs/bug-bounty/blob/main/programs/contracts.md)
states the full requirements.

## Responsible Disclosure

- Do not exploit a vulnerability beyond what is necessary to confirm it exists
- **Do not test against XION mainnet.** Testing that targets live production
  systems disqualifies the report
- Do not access, modify, or exfiltrate user data
- Do not disclose publicly before a fix is confirmed and deployed
