# Admin and CRM operations can manage business units.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Managers can inspect active business units.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "support_user"])
    res := {"allow": true, "filter": {"is_active": {"_eq": true}}}
}
