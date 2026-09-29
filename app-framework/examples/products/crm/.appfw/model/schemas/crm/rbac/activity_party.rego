# Admin and CRM operations can manage all activity parties.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales and support users can inspect activity attendance context.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "sales_rep", "support_user"])
    res := {"allow": true, "filter": {}}
}
