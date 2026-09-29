
# Admin and CRM operations can manage quote statuses.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales and finance users can read quote status taxonomy.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "sales_rep", "finance_user"])
    res := {"allow": true, "filter": {}}
}
