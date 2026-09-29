// CRM Neo4j graph read-model seed.
//
// Idempotent: every node/relationship is MERGE-d, so re-running (the neo4j-seed
// init container runs once per `up`, and may re-run on restart) is safe.
//
// `tenant_id` matches the JWT `company` claim that becomes UserAuth.tenant_id:
//   180000  -> pdsh_admin           (primary island)
//   999999  -> cc_tenant_user       (isolation island; admin must not see it)
//
// Nodes are keyed by `record_locator` (the framework's public locator). Each
// statement is independent (cypher-shell runs them separately), so relationship
// statements re-MATCH their endpoints.

// ---------------------------------------------------------------------------
// Tenant 180000 — accounts + contacts
// ---------------------------------------------------------------------------
MERGE (a:Account {record_locator: 'rl_acct_acme'})
  SET a.tenant_id = '180000', a.name = 'Acme Dental Group';
MERGE (a:Account {record_locator: 'rl_acct_medi'})
  SET a.tenant_id = '180000', a.name = 'MediSystems Inc';
MERGE (a:Account {record_locator: 'rl_acct_techbridge'})
  SET a.tenant_id = '180000', a.name = 'TechBridge Solutions';

MERGE (c:Contact {record_locator: 'rl_ctc_dana'})
  SET c.tenant_id = '180000', c.name = 'Dana Reed';
MERGE (c:Contact {record_locator: 'rl_ctc_sam'})
  SET c.tenant_id = '180000', c.name = 'Sam Lopez';

// Relationships (re-MATCH endpoints; MERGE the edge for idempotency).
MATCH (a:Account {record_locator: 'rl_acct_acme'}), (b:Account {record_locator: 'rl_acct_medi'})
  MERGE (a)-[r:PARENT_OF {tenant_id: '180000'}]->(b);
MATCH (a:Account {record_locator: 'rl_acct_acme'}), (b:Account {record_locator: 'rl_acct_techbridge'})
  MERGE (a)-[r:REFERRED {tenant_id: '180000'}]->(b);
MATCH (c:Contact {record_locator: 'rl_ctc_dana'}), (a:Account {record_locator: 'rl_acct_acme'})
  MERGE (c)-[r:WORKS_AT {tenant_id: '180000'}]->(a);
MATCH (c:Contact {record_locator: 'rl_ctc_sam'}), (a:Account {record_locator: 'rl_acct_medi'})
  MERGE (c)-[r:WORKS_AT {tenant_id: '180000'}]->(a);
MATCH (d:Contact {record_locator: 'rl_ctc_dana'}), (s:Contact {record_locator: 'rl_ctc_sam'})
  MERGE (d)-[r:KNOWS {tenant_id: '180000'}]->(s);

// ---------------------------------------------------------------------------
// Tenant 999999 — isolation island (admin tenant 180000 must NOT see this)
// ---------------------------------------------------------------------------
MERGE (a:Account {record_locator: 'rl_acct_cc'})
  SET a.tenant_id = '999999', a.name = 'CC Customer Org';
MERGE (c:Contact {record_locator: 'rl_ctc_cc'})
  SET c.tenant_id = '999999', c.name = 'Pat CC';
MATCH (c:Contact {record_locator: 'rl_ctc_cc'}), (a:Account {record_locator: 'rl_acct_cc'})
  MERGE (c)-[r:WORKS_AT {tenant_id: '999999'}]->(a);
