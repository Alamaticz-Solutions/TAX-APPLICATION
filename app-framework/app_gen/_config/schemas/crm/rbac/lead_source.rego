
# Admin and CRM operations can manage lead sources.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Marketing and sales users can read source taxonomy.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["marketing_manager", "marketing_user", "sales_manager", "sales_rep"])
    res := {"allow": true, "filter": {}}
}
