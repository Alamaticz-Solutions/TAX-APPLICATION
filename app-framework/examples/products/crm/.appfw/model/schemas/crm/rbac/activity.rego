
# Admin and CRM operations can manage all activities.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales and support users can create work items.
access := res if {
    check_schema_type()
    input.action == "create"
    has_any_role(input.user, ["sales_manager", "sales_rep", "support_user"])
    res := {"allow": true, "filter": {}}
}

# Sales and support users can see open work.
access := res if {
    check_schema_type()
    input.action in ["read", "update"]
    has_any_role(input.user, ["sales_manager", "sales_rep", "support_user"])
    res := {"allow": true, "filter": {"is_closed": {"_eq": false}}}
}
