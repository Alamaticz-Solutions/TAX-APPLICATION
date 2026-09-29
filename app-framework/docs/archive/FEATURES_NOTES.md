# Behaviors

properties with is_required = true should always be selected when reading

Optimistic Concurrency 
Client that queries: the property should be automatically selected, but do not require caller to provide a value.
Table column should be non nullable
Data layer should set with a very fine grained value.
  - name: version
    data_type: Int64
    is_read_only: true
    is_concurrency_control: true


# Compromises

There is no intelligent sorting of the entity types when generating the types and tables in the tables.sql file. 
In {schema}/entity_types/{name}.yaml, you have to declare storage type objects in reverse order of depencency.
For example, below PropertyType depends on the first three:

- name: ForeignKey
  id: 9867d3d9-1623-4c37-8012-b2bb0132b30a
  is_storage_type: true
  ...

- name: NavByFkProperty
  id: 7cd8ad72-991b-4282-be84-6021326622a1
  is_storage_type: true
  ...

- name: NestedEntityType
  id: ec90395a-fda8-459e-82ce-900ca97f41b9
  is_storage_type: true
  ...

- name: PropertyType
  id: 0a576b5b-10c0-469d-b3b4-7242f6e406c9
  is_storage_type: true
  ...
  props:

  - name: foreign_key
    fragment: property-object
    nested_entity_type:
      type_name: ForeignKey

  - name: nav_by_fk_property
    fragment: property-object
    nested_entity_type:
      type_name: NavByFkProperty

  - name: nested_entity_type
    fragment: property-object
    nested_entity_type:
      type_name: NestedEntityType