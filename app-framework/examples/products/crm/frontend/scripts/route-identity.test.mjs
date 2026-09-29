#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { Buffer } from "node:buffer";
import ts from "typescript";

const accountEntity = {
  schemaName: "crm",
  typeName: "Account",
  primaryKey: "id",
  captionField: "name",
  routeSegment: "accounts",
  fields: [
    { name: "id" },
    { name: "name" }
  ],
  scaffold: {
    list: { fields: ["name"] },
    detail: { fields: ["name"] }
  },
  operations: [
    {
      name: "query_accounts",
      kind: "query",
      graphqlName: "queryAccounts",
      returnsShape: "connection",
      selectionPreset: []
    },
    {
      name: "find_account",
      kind: "query",
      graphqlName: "findAccount",
      returnsShape: "record",
      selectionPreset: []
    }
  ]
};

test("record_locator is the route ref for normal rows", async () => {
  const routeIdentity = await importTypeScriptModule("src/scaffold/routeIdentity.ts");
  installLocalStorage({
    "crm-record-route-refs-v1": JSON.stringify({
      "crm.Account": {
        toInternal: { rec_existing: "acct-001" },
        toRoute: { "acct-001": "rec_existing" }
      }
    })
  });

  const routeRef = routeIdentity.recordRouteRef(accountEntity, {
    id: "acct-001",
    record_locator: " rl_crm_account_001 "
  });

  assert.equal(routeRef, "rl_crm_account_001");
  assert.notEqual(routeRef, "acct-001");
  assert.notEqual(routeRef, "rec_existing");
});

test("legacy rec_ refs resolve without creating new legacy mappings", async () => {
  const routeIdentity = await importTypeScriptModule("src/scaffold/routeIdentity.ts");
  const storage = installLocalStorage({
    "crm-record-route-refs-v1": JSON.stringify({
      "crm.Account": {
        toInternal: { rec_legacy: "acct-legacy" },
        toRoute: { "acct-legacy": "rec_legacy" }
      }
    })
  });

  assert.equal(routeIdentity.resolveRecordRouteRef(accountEntity, "rec_legacy"), "acct-legacy");
  assert.equal(routeIdentity.resolveRecordRouteRef(accountEntity, "acct-legacy"), "acct-legacy");
  assert.equal(routeIdentity.getOrCreateRecordRouteRef(accountEntity, " acct-legacy "), "acct-legacy");
  assert.deepEqual(storage.writesFor("crm-record-route-refs-v1"), []);
});

test("new record draft refs round-trip compact initial values", async () => {
  const routeIdentity = await importTypeScriptModule("src/scaffold/routeIdentity.ts");
  installLocalStorage();

  const draftRef = routeIdentity.createNewRecordDraftRef(accountEntity, {
    id: "",
    name: "Acme Launch",
    owner_id: null,
    active: false
  });

  assert.match(draftRef, /^draft_/);
  assert.deepEqual(routeIdentity.resolveNewRecordDraftRef(accountEntity, draftRef), {
    name: "Acme Launch",
    active: false
  });
  assert.equal(routeIdentity.resolveNewRecordDraftRef(accountEntity, "rec_legacy"), null);
});

test("list and detail requests preserve backend route locator identity", async () => {
  const { createAppfwClient } = await importTypeScriptModule("src/lib/appfwClient.ts");
  const requests = [];
  const locator = "rl_crm_account_detail";
  const client = createAppfwClient({
    baseUrl: "https://crm.test",
    fetchImpl: async (url, init) => {
      const body = JSON.parse(String(init.body));
      requests.push({ url, body });

      if (body.query.includes("findAccountByLocator")) {
        return jsonResponse({
          data: {
            findAccountByLocator: {
              id: "acct-detail",
              name: "Detail account",
              record_locator: locator
            }
          }
        });
      }

      return jsonResponse({
        data: {
          queryAccounts: {
            skip: 0,
            limit: 25,
            page_count: 1,
            page_index: 0,
            query_count: 1,
            items: [
              {
                id: "acct-list",
                name: "List account",
                record_locator: "rl_crm_account_list"
              }
            ]
          }
        }
      });
    }
  });

  const listResult = await client.queryEntityList(accountEntity, {
    selection: ["id", "record_locator", "name"]
  });
  const detailResult = await client.findEntityRecord(accountEntity, locator, ["id", "record_locator", "name"]);

  assert.equal(requests[0].url, "https://crm.test/crm");
  assert.match(requests[0].body.query, /\brecord_locator\b/);
  assert.deepEqual(requests[0].body.variables, { limit: 25 });
  assert.equal(listResult.data.rows[0].record_locator, "rl_crm_account_list");

  assert.match(requests[1].body.query, /findAccountByLocator/);
  assert.match(requests[1].body.query, /\$locator: String!/);
  assert.deepEqual(requests[1].body.variables, { locator });
  assert.doesNotMatch(requests[1].body.query, /\$id: String!/);
  assert.equal(detailResult.data.record?.record_locator, locator);
});

test("answer-envelope entity refs compose through generated addressing and view registry", async () => {
  const { composeEntityReferenceNavigation } = await importTypeScriptModule("src/scaffold/entityReferenceNavigation.ts");
  const { crmUiContract } = await importTypeScriptModule("src/generated/appfw-ui-contract.ts");

  const navigation = composeEntityReferenceNavigation(crmUiContract, {
    entity: { schemaName: "crm", typeName: "Account" },
    idKind: "record_locator",
    ids: ["rl_crm_account_001"],
    view_hint: "detail",
    query: {
      filter: {
        record_locator: { _in: ["rl_crm_account_001", "rl_crm_account_002"] }
      }
    }
  });

  assert.ok(navigation);
  assert.equal(navigation.idKind, "record_locator");
  assert.equal(navigation.recordLocator, "rl_crm_account_001");
  assert.equal(navigation.detailHref, "/data/accounts/rl_crm_account_001");
  assert.equal(navigation.listHref, "/data/accounts");
  assert.equal(
    navigation.filteredListHref,
    `/data/accounts?filter=${encodeURIComponent(JSON.stringify({
      record_locator: { _in: ["rl_crm_account_001", "rl_crm_account_002"] }
    }))}`
  );
  assert.equal(navigation.nativeView?.viewId, "entity:accounts:detail");
  assert.equal(navigation.resolveOperation?.graphqlName, "findAccountByLocator");
  assert.deepEqual(navigation.resolveOperation?.variables, { locator: "rl_crm_account_001" });
  assert.equal(navigation.exposesPrimaryKey, false);
});

test("answer-envelope flow refs select generated workflow views and timeline surfaces", async () => {
  const { composeEntityReferenceNavigation } = await importTypeScriptModule("src/scaffold/entityReferenceNavigation.ts");
  const { crmUiContract } = await importTypeScriptModule("src/generated/appfw-ui-contract.ts");

  const navigation = composeEntityReferenceNavigation(crmUiContract, {
    kind: "Account",
    record_locator: "rl_crm_account_001",
    view_hint: "flow-graph"
  });

  assert.ok(navigation);
  assert.equal(navigation.nativeView?.viewId, "workflow:accounts");
  assert.equal(navigation.nativeView?.route, "/accounts");
  assert.ok(navigation.nativeView?.supports.shapes.includes("flow_graph"));
  assert.equal(navigation.timelineWorkflow?.id, "audit");
  assert.ok(navigation.timelineWorkflow?.proofPoints.includes("read_only_timeline"));
});

test("primary keys are not accepted as answer-envelope navigation currency", async () => {
  const { composeEntityReferenceNavigation } = await importTypeScriptModule("src/scaffold/entityReferenceNavigation.ts");
  const { crmUiContract } = await importTypeScriptModule("src/generated/appfw-ui-contract.ts");

  const navigation = composeEntityReferenceNavigation(crmUiContract, {
    entity: { schemaName: "crm", typeName: "Account" },
    idKind: "primary_key",
    ids: ["acct-001"],
    view_hint: "detail"
  });

  assert.equal(navigation?.recordLocator, null);
  assert.equal(navigation?.detailHref, null);
  assert.equal(navigation?.resolveOperation, null);
  assert.equal(navigation?.exposesPrimaryKey, false);
});

async function importTypeScriptModule(relativePath) {
  const source = await readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2020,
      target: ts.ScriptTarget.ES2020,
      verbatimModuleSyntax: false
    }
  }).outputText;
  const cacheBust = `\n//# sourceURL=${relativePath}?test=${Date.now()}-${Math.random()}`;
  const encoded = Buffer.from(`${output}${cacheBust}`).toString("base64");
  return import(`data:text/javascript;base64,${encoded}`);
}

function installLocalStorage(initialValues = {}) {
  const values = new Map(Object.entries(initialValues));
  const writes = [];
  const storage = {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key) {
      return values.has(key) ? values.get(key) : null;
    },
    key(index) {
      return Array.from(values.keys())[index] ?? null;
    },
    removeItem(key) {
      values.delete(key);
    },
    setItem(key, value) {
      writes.push({ key, value: String(value) });
      values.set(key, String(value));
    },
    writesFor(key) {
      return writes.filter((write) => write.key === key);
    }
  };
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    writable: true,
    value: storage
  });
  return storage;
}

function jsonResponse(payload, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: status >= 200 && status < 300 ? "OK" : "Error",
    headers: {
      get(name) {
        if (name === "x-request-id") return "test-request";
        if (name === "x-correlation-id") return "test-correlation";
        return null;
      }
    },
    json: async () => payload
  };
}
