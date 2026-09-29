
# Admin, CRM operations, and catalog managers can manage products.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops", "catalog_manager"])
    res := {"allow": true, "filter": {}}
}

# Sales and finance users can quote active products.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "sales_rep", "finance_user", "support_user"])
    res := {"allow": true, "filter": {"is_active": {"_eq": true}}}
}
