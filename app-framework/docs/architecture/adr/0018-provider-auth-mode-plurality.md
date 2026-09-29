# ADR 0018: Provider Authentication-Mode Plurality And Connection-Level Certification

## Status

Accepted (2026-08-04). Proposed 2026-07-28. Forcing function: MS SQL Server
ODBC unification with partner NTLM on Linux via FreeTDS — see
[spec](../../specs/mssql-odbc-ntlm-authentication.md). Implemented via the
shared `appfw_mssql_auth` registry and ODBC-backed `appfw_provider_mssql`.

## Context

A data provider is increasingly reached through more than one authentication
mechanism depending on where the target instance lives and who controls it.
MS SQL Server alone must support SQL logins (`sql_password` on Microsoft ODBC
Driver 18), Microsoft Entra ID (for Azure/Arc-eligible instances, today gated
to `FabricSqlAnalytics`), and on-prem AD NTLM (`ntlm` via FreeTDS ODBC for
domain-joined, vendor-controlled instances such as Epic-hosted Clarity that
cannot be Arc-enabled). These are not a migration sequence where one mechanism
eventually replaces the others — the estate is permanently heterogeneous, so
multiple mechanisms are co-equal and durable.

Separately, the certification model (ADR 0002) represents provider support as
granular `ProviderContractArea` entries with evidence tiers. Authentication
mechanism is **not** a query-semantics contract area — it is a connection-
level concern, the same category as TLS mode, which already lives outside the
semantic-parity matrix and is proven by dedicated unit/integration tests in
each provider's own connection code.

## Decision

1. **A single provider may carry multiple permanent, co-equal, structurally
   distinct authentication mechanisms**, selected per environment by an
   `auth_mode` on the data source. The mechanisms share one downstream path
   (session, pooling, execution, audit, migrations); only credential
   acquisition differs. No mechanism is privileged as "the default that the
   others bridge toward."

2. **Connection-level auth modes are certified by dedicated connection/auth
   live tests, not by a second full provider-parity suite and not by new
   `ProviderContractArea` columns or provider identities.** Mirrors the
   TLS-mode precedent: prove handshake, credential acquisition, and
   reconnect/lifecycle behavior hermetically (e.g. `tds-mock` +
   `ntlm_e2e` with FreeTDS client and `ntlm-auth` oracle; golden anchor from
   real Windows SQL). Linux SQL Server cannot accept remote NTLM for Windows
   logins — certification uses the protocol-fidelity mock plus scheduled live
   smoke (`scripts/ci/mssql-ntlm-live-smoke.sh`). Existing semantic
   live-certification for the provider
   (today's CRM-fixture `sql_password` / SA-password mssql
   `provider-parity.json` pass) remains the query-semantics proof and is
   neither replaced nor duplicated under the new auth mode. Record in the
   evidence matrix what was auth-proven vs. semantically live-certified.

## Consequences

- Adding an auth mechanism is a bounded change to a provider's `auth_mode`
  enum, credential acquisition, and connection wiring — not a change to the
  semantic-parity matrix, and not a new provider.
- Evidence stays honest: a new auth mode does not claim `live-certified`
  semantic parity it did not re-exercise; connection proof and semantic
  proof are separate axes.
- Mechanisms that cannot be self-certified or that graduate a capability
  (e.g. NTLM on ODBC) remain Change Class D: human approval and independent
  review are still required regardless of automated certification output.
- Credential *delivery* that is owned outside the framework (e.g. platform
  secret injection for domain passwords) is an assumed precondition, not a
  framework-generated artifact; the framework certifies the connection given
  valid credentials, not the platform plumbing.
