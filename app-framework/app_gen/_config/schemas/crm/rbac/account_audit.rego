# Audit timelines are append-only system records. Admin and CRM operations can inspect them.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}
