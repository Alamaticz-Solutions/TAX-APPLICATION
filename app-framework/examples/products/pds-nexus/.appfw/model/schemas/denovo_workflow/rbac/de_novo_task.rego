# Local synthetic pilot: every allowed read is scoped to the primary fixture tenant.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["admin", "nexus_ops", "nexus_reader"])
    res := {"allow": true, "filter": {"tenant": {"_eq": "tenant_a"}}}
}
