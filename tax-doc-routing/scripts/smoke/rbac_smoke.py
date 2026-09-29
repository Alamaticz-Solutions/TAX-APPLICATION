#!/usr/bin/env python3
"""Live RBAC smoke test for the tax_routing GraphQL API (docs/architecture/rbac-design.md).

Needs: the backend running (default http://127.0.0.1:8090, override TAX_API) and the Tax Postgres
container `tax-doc-routing-postgres` (override TAX_PG_CONTAINER). It seeds its own rows (every name
is prefixed `SMOKE-`), asserts the access matrix as different local identities, and removes its
rows afterwards. Identities use the local test-auth bearer form:
    Authorization: Bearer appfw-local:user=<name>;tenant=t1;roles=<ignored, see below>

Access is database-driven (docs/architecture/rbac-design.md): the backend resolves grants from
the app_user / user_role / role / role_permission / permission tables, keyed by (tenant_id,
user_name) from the token — the `roles=` part of the bearer token is no longer read by any policy.
This test therefore seeds its own app_user + user_role rows for each test identity (pointing at
the real TAX_STAFF / TAX_ADMIN role ids from seeds/01_role.yaml) before asserting anything, and
polls for up to WAIT_TIMEOUT seconds because the backend's grant cache (services/authz.rs) only
re-reads the database every ~30s.

Run:  python scripts/smoke/rbac_smoke.py
Exit code 1 if any assertion fails. "KNOWN GAP" lines are documented defects, not failures.
"""
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid

API = os.environ.get("TAX_API", "http://127.0.0.1:8090") + "/tax-routing"
PG = os.environ.get("TAX_PG_CONTAINER", "tax-doc-routing-postgres")

# Fixed ids from .appfw/model/schemas/tax_routing/seeds/01_role.yaml — referencing the real seeded
# roles, not re-declaring them, so this test exercises the actual RBAC data, not a stand-in for it.
TAX_STAFF_ROLE = "208a2102-a238-4e7c-b2d3-c279bf00f89e"
TAX_ADMIN_ROLE = "95b5f46a-de62-463e-b822-d437ac2b28c7"

STAFF_A, STAFF_B, ADMIN = "smoke_alex", "smoke_sarah", "smoke_dana"
NOBODY = "smoke_mike"  # deliberately never gets an app_user row: tests the "unknown user" case

WAIT_TIMEOUT = 40  # seconds to wait for the backend's authz cache to pick up seeded rows

failures = []


def sql(statement):
    out = subprocess.run(
        ["docker", "exec", PG, "psql", "-U", "postgres", "-d", "tax_routing", "-qAtc", statement],
        capture_output=True, text=True,
    )
    if out.returncode:
        sys.exit(f"psql failed: {out.stderr.strip()}")
    return out.stdout.strip()


def gql(query, who=None):
    headers = {"content-type": "application/json"}
    if who:
        headers["Authorization"] = f"Bearer appfw-local:user={who};tenant=t1;roles=n/a"
    req = urllib.request.Request(API, data=json.dumps({"query": query}).encode(), headers=headers)
    try:
        return json.load(urllib.request.urlopen(req, timeout=20))
    except urllib.error.HTTPError as e:
        return {"errors": [{"message": f"HTTP {e.code}"}]}


def items(entity, fields, who):
    res = gql("{ query%s(limit: 200) { items { %s } } }" % (entity, fields), who)
    if res.get("errors"):
        return None
    data = res["data"]["query" + entity]
    return data["items"] if data else []


def check(name, ok, detail=""):
    print(f"  {'PASS' if ok else 'FAIL'}  {name}{('  ' + detail) if detail and not ok else ''}")
    if not ok:
        failures.append(name)


def known_gap(name, detail):
    print(f"  KNOWN GAP  {name}: {detail}")


def input_fields(type_name):
    res = gql('{__type(name:"%s"){inputFields{name type{name kind ofType{name}}}}}' % type_name)
    return res["data"]["__type"]["inputFields"]


def build_input(type_name, overrides, ref_id):
    parts = []
    for f in input_fields(type_name):
        name = f["name"]
        if name in overrides:
            parts.append(f"{name}: {overrides[name]}")
            continue
        if name in ("version", "record_locator"):
            continue
        base = f["type"]["name"] or f["type"]["ofType"]["name"]
        if name == "id":
            value = f'"{uuid.uuid4()}"'
        elif name.endswith("_id"):
            value = f'"{ref_id}"'
        elif "email" in name:
            value = '"smoke@example.com"'
        else:
            value = {"String": '"x"', "Int": "1", "Float": "1", "Boolean": "true",
                     "DateTime": '"2026-05-12T10:00:00Z"', "RoutingRecordStatus": "CollectingInfo"}.get(base)
        if value:
            parts.append(f"{name}: {value}")
    return "{" + ", ".join(parts) + "}"


def cleanup():
    sql("delete from tax_routing.tax_documents where document_name like 'SMOKE-%'")
    sql("delete from tax_routing.exception_tasks where failure_reason like 'SMOKE-%'")
    sql("delete from tax_routing.tax_routing_records where client_full_name like 'SMOKE-%'")
    sql("delete from tax_routing.tax_payer_ref_data where full_name like 'SMOKE-%'")
    sql("delete from tax_routing.tax_entity_lists where entity_name like 'SMOKE-%'")
    sql("delete from tax_routing.user_roles where user_id in "
        "(select id from tax_routing.app_users where user_name like 'smoke_%')")
    sql("delete from tax_routing.app_users where user_name like 'smoke_%'")


def seed_identity(user_name, role_id):
    """Creates an app_user + user_role row for a test identity and returns its app_user id."""
    user_id = str(uuid.uuid4())
    sql(f"insert into tax_routing.app_users (id, tenant_id, user_name, display_name, is_active) "
        f"values ('{user_id}','t1','{user_name}','{user_name}',true)")
    sql(f"insert into tax_routing.user_roles (id, user_id, role_id, granted_at) "
        f"values (gen_random_uuid(),'{user_id}','{role_id}',now())")
    return user_id


def wait_for_grants(user_name, expect_user_id):
    """Polls until the backend's authz cache resolves `user_name` to exactly `expect_user_id`.

    The cache is keyed by (tenant_id, user_name), and this test reuses the same smoke_* user
    names across runs with a fresh random app_user id each time — checking only "does a read
    succeed" would pass against a *stale* cached identity from a previous run's cache entry
    (same name, old id, same permissions), and every ownership-scoped assertion after that would
    then compare against the wrong id. Waiting for the id itself to match rules that out.
    """
    deadline = time.time() + WAIT_TIMEOUT
    while time.time() < deadline:
        res = gql("{ myPermissions }", user_name)
        if not res.get("errors") and res["data"]["myPermissions"].get("user_id") == expect_user_id:
            return True
        time.sleep(2)
    return False


def main():
    cleanup()

    alex_id = seed_identity(STAFF_A, TAX_STAFF_ROLE)
    sarah_id = seed_identity(STAFF_B, TAX_STAFF_ROLE)
    dana_id = seed_identity(ADMIN, TAX_ADMIN_ROLE)
    # NOBODY gets no app_user row at all.

    print(f"Waiting up to {WAIT_TIMEOUT}s for the backend's authz cache to pick up the seeded identities...")
    for name, uid in [(STAFF_A, alex_id), (STAFF_B, sarah_id), (ADMIN, dana_id)]:
        if not wait_for_grants(name, uid):
            sys.exit(f"authz cache never resolved {name} to its freshly seeded app_user id; is the "
                      "backend running against the same tax_routing database this test seeds?")
    print("  cache refreshed")

    ref_id = str(uuid.uuid4())
    sql(f"insert into tax_routing.tax_payer_ref_data (id, full_name, is_active, read_write_password) values "
        f"('{ref_id}','SMOKE-Active',true,'S3cret-Pass!'),(gen_random_uuid(),'SMOKE-Inactive',false,null)")
    sql("insert into tax_routing.tax_entity_lists (id, entity_name, is_active) values "
        "(gen_random_uuid(),'SMOKE-Entity',true),(gen_random_uuid(),'SMOKE-Dormant',false)")
    # Every column TaxRoutingRecord requires NOT NULL (business spec 5.4's denormalised client
    # fields), so a real record always has them by the time it exists — set them here too, or
    # any later write (including cancel_record / reassign_record's "carry every field forward"
    # replace) fails validation on a smoke-seeded row the same way it would on stale real data.
    common_cols = ("client_first_name, client_last_name, office_location, pds_email, "
                   "personal_email, internal_folder, folder_name, read_write_password, "
                   "client_reference_id, created_at, updated_at")
    common_vals = f"'x','x','x','x@example.com','x@example.com','x','x','x','{ref_id}',now(),now()"
    sql("insert into tax_routing.tax_routing_records "
        f"(id, status, assigned_user_id, client_full_name, {common_cols}) values "
        f"(gen_random_uuid(),'collecting_info','{alex_id}','SMOKE-A1',{common_vals}),"
        f"(gen_random_uuid(),'collecting_info','{alex_id}','SMOKE-A2',{common_vals}),"
        f"(gen_random_uuid(),'processing','{sarah_id}','SMOKE-S1',{common_vals})")
    sql("insert into tax_routing.tax_documents (id, document_name, assigned_user_id) values "
        f"(gen_random_uuid(),'SMOKE-a.pdf','{alex_id}'),(gen_random_uuid(),'SMOKE-s.pdf','{sarah_id}')")
    sql("insert into tax_routing.exception_tasks (id, failure_reason, assigned_user_id) values "
        f"(gen_random_uuid(),'SMOKE-a','{alex_id}'),(gen_random_uuid(),'SMOKE-s','{sarah_id}')")

    def names(rows, key):
        return sorted(r[key] for r in rows if str(r[key]).startswith("SMOKE-")) if rows is not None else None

    print("Routing records: staff see only their own (BR-19), admin sees all (BR-20)")
    f = "client_full_name"
    check("staff A sees A1, A2 only", names(items("TaxRoutingRecords", f, STAFF_A), f) == ["SMOKE-A1", "SMOKE-A2"])
    check("staff B sees S1 only", names(items("TaxRoutingRecords", f, STAFF_B), f) == ["SMOKE-S1"])
    check("admin sees all three", names(items("TaxRoutingRecords", f, ADMIN), f) == ["SMOKE-A1", "SMOKE-A2", "SMOKE-S1"])
    check("an unknown identity (no app_user row) is denied", items("TaxRoutingRecords", f, NOBODY) is None)

    print("Documents follow their record's owner")
    f = "document_name"
    check("staff A sees only A's document", names(items("TaxDocuments", f, STAFF_A), f) == ["SMOKE-a.pdf"])
    check("staff B sees only B's document", names(items("TaxDocuments", f, STAFF_B), f) == ["SMOKE-s.pdf"])
    check("admin sees both", names(items("TaxDocuments", f, ADMIN), f) == ["SMOKE-a.pdf", "SMOKE-s.pdf"])

    print("Exception queue is shared (business spec 5.3)")
    f = "failure_reason"
    for label, who in [("staff A", STAFF_A), ("staff B", STAFF_B), ("admin", ADMIN)]:
        check(f"{label} sees the whole queue", names(items("ExceptionTasks", f, who), f) == ["SMOKE-a", "SMOKE-s"])

    print("Reference data: staff read active rows only, admin reads all")
    check("staff: taxpayers active only", names(items("TaxPayerRefData", "full_name", STAFF_A), "full_name") == ["SMOKE-Active"])
    check("admin: taxpayers all", names(items("TaxPayerRefData", "full_name", ADMIN), "full_name") == ["SMOKE-Active", "SMOKE-Inactive"])
    check("staff: entities active only", names(items("TaxEntityLists", "entity_name", STAFF_A), "entity_name") == ["SMOKE-Entity"])
    check("admin: entities all", names(items("TaxEntityLists", "entity_name", ADMIN), "entity_name") == ["SMOKE-Dormant", "SMOKE-Entity"])

    print("Writes")
    rec = build_input  # readability
    a_id, a_ver = sql("select id||'|'||version from tax_routing.tax_routing_records where client_full_name='SMOKE-A1'").split("|")
    s_id, s_ver = sql("select id||'|'||version from tax_routing.tax_routing_records where client_full_name='SMOKE-S1'").split("|")

    def update(rid, ver, owner_id, who):
        body = rec("InputTaxRoutingRecord", {"id": f'"{rid}"', "version": ver, "assigned_user_id": f'"{owner_id}"',
                                            "client_full_name": '"SMOKE-edited"'}, ref_id)
        return gql("mutation { updateTaxRoutingRecord(input:%s) { id } }" % body, who)

    check("staff updates own record", not update(a_id, a_ver, alex_id, STAFF_A).get("errors"))
    check("staff cannot update another's record", bool(update(s_id, s_ver, sarah_id, STAFF_A).get("errors")))
    check("admin updates any record", not update(s_id, s_ver, sarah_id, ADMIN).get("errors"))
    created = gql("mutation { createTaxRoutingRecord(input:%s) { id } }" % rec(
        "InputTaxRoutingRecord", {"assigned_user_id": f'"{alex_id}"', "client_full_name": '"SMOKE-new"'}, ref_id), STAFF_A)
    check("staff creates a record", not created.get("errors"))
    deleted = gql('mutation { deleteTaxRoutingRecord(input:{id:"%s"}) }' % a_id, STAFF_A)
    check("staff cannot delete a record", bool(deleted.get("errors")))
    ref_update = gql("mutation { updateTaxPayerRefData(input:%s) { id } }" % rec(
        "InputTaxPayerRefData", {"id": f'"{ref_id}"', "full_name": '"SMOKE-Active"'}, ref_id), STAFF_A)
    check("staff cannot write taxpayer reference data", bool(ref_update.get("errors")))

    print("Admin-only actions (custom methods, checked by permission code, not a role literal)")
    # cancelRecord / reassignRecord return a raw JSON scalar (serde_json::Value), which has no
    # GraphQL subfields — the query must not request a selection set on them.
    a2_id = sql("select id from tax_routing.tax_routing_records where client_full_name='SMOKE-A2'")
    cancel_by_staff = gql(
        'mutation { cancelRecord(recordId:"%s", resolutionStatus:"Cancelled", comments:"smoke test") }' % a2_id,
        STAFF_A,
    )
    check("staff cannot cancel a record", bool(cancel_by_staff.get("errors")))
    cancel_by_admin = gql(
        'mutation { cancelRecord(recordId:"%s", resolutionStatus:"Cancelled", comments:"smoke test") }' % a2_id,
        ADMIN,
    )
    check("admin can cancel a record", not cancel_by_admin.get("errors"))

    reassign_by_staff = gql(
        'mutation { reassignRecord(recordId:"%s", newAssignedUserId:"%s") }' % (a_id, sarah_id), STAFF_A
    )
    check("staff cannot reassign a record", bool(reassign_by_staff.get("errors")))
    reassign_by_admin = gql(
        'mutation { reassignRecord(recordId:"%s", newAssignedUserId:"%s") }' % (a_id, sarah_id), ADMIN
    )
    check("admin can reassign a record", not reassign_by_admin.get("errors"))

    print("myPermissions reflects the database, not a role literal")
    perms = gql("{ myPermissions }", STAFF_A)
    staff_codes = [] if perms.get("errors") else [g["code"] for g in perms["data"]["myPermissions"]["grants"]]
    check("staff holds routing_record.read", "routing_record.read" in staff_codes)
    check("staff does not hold routing_record.cancel", "routing_record.cancel" not in staff_codes)
    perms = gql("{ myPermissions }", ADMIN)
    admin_codes = [] if perms.get("errors") else [g["code"] for g in perms["data"]["myPermissions"]["grants"]]
    check("admin holds routing_record.cancel", "routing_record.cancel" in admin_codes)
    check("admin holds rbac.manage", "rbac.manage" in admin_codes)

    print("Audit trail: readable with the audit.read permission, not a role literal")
    audit = gql("{ queryTaxRoutingRecordsAudit(limit: 1) { items { audit_id } } }", ADMIN)
    check("admin (holding tax_routing_record_audit.read) can read the audit trail", not audit.get("errors"))
    audit_denied = gql("{ queryTaxRoutingRecordsAudit(limit: 1) { items { audit_id } } }", STAFF_A)
    check("staff (no audit permission) cannot read the audit trail", bool(audit_denied.get("errors")))

    print("Known gaps (documented, not failures)")
    leaked = items("TaxPayerRefData", "full_name read_write_password", STAFF_A) or []
    if any(r.get("read_write_password") for r in leaked if str(r["full_name"]).startswith("SMOKE-")):
        known_gap("plaintext client password readable by staff",
                  "read_write_password is returned by GraphQL; the framework has no field-level read hiding. "
                  "See docs/architecture/tax-document-routing-design.md, section 5.2.")
    else:
        check("client password is not readable by staff", True)

    cleanup()
    print("\nRBAC SMOKE %s" % ("FAILED: " + ", ".join(failures) if failures else "PASSED"))
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
