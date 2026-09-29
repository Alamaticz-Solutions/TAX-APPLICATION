# Modules
# In Rego, policies are defined inside modules. Modules consist of:
#     - Exactly one Package declaration.
#     - Zero or more Import statements.
#     - Zero or more Rule definitions.




sites := [{"name": "prod"}, {"name": "smoke1"}, {"name": "dev"}]

r if {
    some site in sites
    site.name == "prod"
}

# The rule r above asserts that there exists (at least) one document within sites where the name attribute equals "prod".
# The result: true



# Incremental Definitions
# A rule may be defined multiple times with the same name. When a rule is defined this way, we refer
# to the rule definition as incremental because each definition is additive. The document produced by
# incrementally defined rules is the union of the documents produced by each individual rule.
# For example, we can write a rule that abstracts over our servers and containers data as instances:

instances contains instance if {

    server := sites[_].servers[_]

    instance := {"address": server.hostname, "name": server.name}

}

​
instances contains instance if {

    container := containers[_]

    instance := {"address": container.ipaddress, "name": container.name}

}