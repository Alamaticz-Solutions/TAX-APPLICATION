package system.offices
import rego.v1

# Row-level filter for Office entity type in the System schema


read_filter = filter if {

    # if tenant id is PDS Health id, then return empty filter

    input.subject.tenant_id == "180000"

    filter = {}

} else = filter if {

    # otherwise, return filter to match on the tenant_id value

    filter = { "tenant_id": input.subject.tenant_id }

}
