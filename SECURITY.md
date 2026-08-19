# Security Policy

This repository holds the XION treasury contract — the **Treasury** asset in the
[Core Protocol Contracts bug bounty program](https://github.com/burnt-labs/bug-bounty/blob/main/programs/contracts.md).
This policy is built from that program's terms.
[`burnt-labs/bug-bounty`](https://github.com/burnt-labs/bug-bounty) remains the
canonical source — where this file and the program documents differ, the
program documents govern.

## Reporting a Vulnerability

**Do not open a public GitHub issue for a security vulnerability.** Public
disclosure before a patch is available increases the harm to users.

| Type of finding                  | How to report                                                       |
| -------------------------------- | ------------------------------------------------------------------- |
| Security vulnerability           | **Security → Report a vulnerability** on this repository, or email [security@burnt.com](mailto:security@burnt.com) |
| Non-sensitive or operational bug | Open a GitHub issue on this repository                              |

Prefer GitHub private vulnerability reporting: the fix is developed against the
report, and you are credited on the published advisory and in any CVE we
request.

Include the type of vulnerability, affected version, steps to reproduce,
impact, how an attacker would exploit it, and any known mitigations.

We acknowledge receipt within **5 business days** and provide a triage decision
within **14 days**. Active exploitation, or confirmed attacker awareness of an
unpatched vulnerability, escalates the issue to Critical handling regardless of
its original classification.

## Scope

Scope applies to the contract as deployed on the current XION mainnet.
Findings affecting only deprecated deployments, or already remediated in the
currently deployed bytecode, are not eligible regardless of whether the fix
was publicly announced. Verify exploitability against the current deployed
contract version before submitting.

### Fee Grant Scope

Fee grant issuance findings are in scope where an **unprivileged caller can
extract value from the treasury's XION balance without effective bound**.

A grant is treated as bounded — and therefore out of scope — only when its
allowance and expiration **cannot be reset, refreshed, revoked and reissued,
or otherwise renewed by an unprivileged caller**. Where a configured spend cap
can be renewed, the cap does not bound total extraction in practice, and the
finding is in scope.

Findings against genuinely bounded grant operations are not eligible. That
design intentionally delegates authorization to the calling application layer,
and a bounded grant behaving as specified is not a vulnerability.

## Severity

| Severity     | Description                                                                                                                                                                                                                                                                                                    |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **CRITICAL** | Direct, permanent, irrecoverable theft or loss of funds held in or routed through the contract at meaningful scale. Complete bypass of account authentication where the proof of concept demonstrates actual movement of funds from a pre-existing victim account to an attacker-controlled address using only attacker-controlled keys. Permanent state corruption with no recovery path |
| **HIGH**     | Theft or freezing of funds affecting individual accounts. Authentication bypass with demonstrated exploitability against an existing account. Permanent disruption of core contract functionality                                                                                                                |
| **MEDIUM**   | Limited fund loss requiring specific preconditions. Attacks requiring privileged-party cooperation. Temporary disruption recoverable by governance                                                                                                                                                              |
| **LOW**      | Valid, reproducible code-level issue with no direct risk to funds, representing a meaningful hardening opportunity. Must include a specific code reference                                                                                                                                                       |

Only **High** and **Critical** findings are reward eligible. Collecting a
bounty requires completing a KYC process; we cannot pay reporters in
sanctioned jurisdictions. Where several reports describe the same underlying
issue, the first complete report with a working proof of concept is the one
considered. We assess reports as submitted; we do not reclassify a report to a
different severity on a reporter's behalf.

## Proof of Concept

**An end-to-end proof of concept is required.**

Tests that mock contract state or bypass CosmWasm message routing — including
`cw-multi-test` environments and harnesses that stub the bank, staking, or IBC
modules — do not demonstrate exploitability on their own.

The proof of concept should run against a **locally running XION node
configured with mainnet parameters**, using the governance-deployed contract
bytecode, the XION ante handler chain, and module configuration matching
mainnet. The attack should be executed via standard transaction broadcast
against that node.

## Permissioned Chain Policy

XION mainnet operates with `code_upload_access: Nobody`. New contracts require
governance approval to deploy.

**Any attack vector requiring an attacker to deploy a malicious contract on
mainnet is out of scope, regardless of technical validity.** A finding must be
exploitable using only contracts already deployed on mainnet.

## Privileged Actor Policy

Attacks requiring a contract admin, governance, or another privileged party to
take self-destructive or colluding action are classified at **Medium at
most**, regardless of downstream impact. The threat model assumes privileged
actors behave according to their role.

## Out of Scope

**Assets**

- The [`burnt-labs/account-contract`](https://github.com/burnt-labs/account-contract)
  repository — report account findings there
- The contracts in [`burnt-labs/contracts`](https://github.com/burnt-labs/contracts),
  and example and demo contracts
- Third-party contracts deployed on XION by external teams
- Chain node modules — see the
  [Blockchain / DLT program](https://github.com/burnt-labs/bug-bounty/blob/main/programs/blockchain.md)
- Applications and the client SDK — see the
  [Applications and SDKs program](https://github.com/burnt-labs/bug-bounty/blob/main/programs/applications.md)
- Upstream dependencies. Vulnerabilities in CosmWasm or the Cosmos SDK are not
  eligible here; only code originating in this repository is covered

**Vulnerability classes**

- Attacks requiring malicious contract deployment on mainnet
- Denial of service requiring sustained attacker resource expenditure
  proportional to the harm caused
- Fee grant operations that are bounded as defined above
- Governance attacks requiring a malicious proposal to pass
- Theoretical vulnerabilities without a working end-to-end proof of concept
- Attacks where the attacker's cost to execute exceeds the demonstrable harm to
  the protocol or its users
- Best practices, gas optimizations, missing events, and informational findings

## Responsible Disclosure

- Do not exploit a vulnerability beyond what is necessary to confirm it exists
- **Do not test against production systems.** This includes XION mainnet.
  Testing production disqualifies the report
- Use only a local environment or infrastructure you control. Testnets,
  staging, preview, and development deployments are not authorized by
  implication
- Do not access, modify, or exfiltrate user data
- Do not disrupt or degrade our networks, data, or services
- Do not disclose publicly before a fix is confirmed and deployed
- Allow us reasonable time to address the issue

## Safe Harbor

Burnt Labs will not pursue legal action against researchers who report
vulnerabilities in good faith under this policy, do not exploit beyond what is
necessary to confirm the finding, do not access or disclose user data, and do
not disrupt production systems.

**Naming this contract as an asset establishes eligibility, not permission to
test the production deployment.** Authorization to actively test extends only
to local environments and infrastructure you control. Reporting a
vulnerability you encountered incidentally is always welcome.
