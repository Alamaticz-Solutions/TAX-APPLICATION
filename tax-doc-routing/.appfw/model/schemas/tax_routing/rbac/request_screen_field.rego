# Database-driven RBAC (docs/architecture/rbac-design.md). Access is decided entirely by the
# caller's resolved permission grants — no role name is written here. This file names only the
# resource it governs ("request_screen_field").
# bodies-only: see tax_routing_record.rego for the header/helpers the generator supplies.
# Catalogue table (docs/architecture/dynamic-form-engine-design.md): Tax Staff read it to
# render the New Request screens; only Tax Admin may edit it.

access := res if {
	check_schema_type()
	grants := [g |
		some g in input.user.grants
		g.resource == "request_screen_field"
		g.action == input.action
	]
	count(grants) > 0
	res := access_for_scope(best_scope(grants), grants, input.user)
}

best_scope(grants) := "all" if {
	some g in grants
	g.scope == "all"
}

best_scope(grants) := "own" if {
	not has_scope(grants, "all")
	some g in grants
	g.scope == "own"
}

best_scope(grants) := "active_only" if {
	not has_scope(grants, "all")
	not has_scope(grants, "own")
	some g in grants
	g.scope == "active_only"
}

has_scope(grants, scope) if {
	some g in grants
	g.scope == scope
}

access_for_scope("all", _, _) := {"allow": true, "filter": {}}

access_for_scope("own", grants, user) := {"allow": true, "filter": {own_owner_field(grants): {"_eq": user.id}}}

access_for_scope("active_only", _, _) := {"allow": true, "filter": {"is_active": {"_eq": true}}}

own_owner_field(grants) := field if {
	some g in grants
	g.scope == "own"
	field := g.owner_field
}
