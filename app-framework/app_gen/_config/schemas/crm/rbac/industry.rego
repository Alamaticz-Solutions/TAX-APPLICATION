
# Admin and CRM operations can manage industries.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# CRM business users can read industry classifications.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "sales_rep", "marketing_user", "finance_user"])
    res := {"allow": true, "filter": {}}
}
