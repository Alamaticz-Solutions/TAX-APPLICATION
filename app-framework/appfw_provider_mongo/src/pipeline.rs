use appfw_runtime::{
    query_ir::{RuntimeAggregateFunction, RuntimeSortDirection},
    RuntimeError, RuntimeFilterOp,
};
use bson::{doc, Bson, Document};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoAggregateGroup {
    pub alias: String,
    pub field_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoAggregateMetric {
    pub function: RuntimeAggregateFunction,
    pub alias: String,
    pub field_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MongoAggregateHavingPredicate {
    pub metric_alias: String,
    pub op: RuntimeFilterOp,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoAggregateSortSpec {
    pub alias: String,
    pub direction: RuntimeSortDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoRelationshipLookup {
    Direct {
        relationship_name: String,
        target_collection: String,
        local_field: String,
        foreign_field: String,
        flatten_to_one: bool,
    },
    ManyToMany {
        relationship_name: String,
        junction_collection: String,
        junction_alias: String,
        source_key_field: String,
        junction_local_key: String,
        junction_foreign_key: String,
        target_collection: String,
        target_key_field: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoSelectionProjectionNode {
    pub name: String,
    pub is_object_id_key: bool,
    pub children: Vec<MongoSelectionProjectionNode>,
}

pub fn combine_filter_documents(filter_doc: Document, access_doc: Document) -> Document {
    match (filter_doc.is_empty(), access_doc.is_empty()) {
        (true, true) => doc! {},
        (false, true) => filter_doc,
        (true, false) => access_doc,
        (false, false) => doc! { "$and": [filter_doc, access_doc] },
    }
}

pub fn relationship_lookup_stages(lookup: MongoRelationshipLookup) -> Vec<Document> {
    match lookup {
        MongoRelationshipLookup::Direct {
            relationship_name,
            target_collection,
            local_field,
            foreign_field,
            flatten_to_one,
        } => {
            let mut stages = vec![doc! {
                "$lookup": {
                    "from": target_collection,
                    "localField": local_field,
                    "foreignField": foreign_field,
                    "as": relationship_name.clone()
                }
            }];
            if flatten_to_one {
                stages.push(doc! {
                    "$set": {
                        relationship_name.clone(): {
                            "$ifNull": [
                                { "$first": format!("${relationship_name}") },
                                Bson::Null
                            ]
                        }
                    }
                });
            }
            stages
        }
        MongoRelationshipLookup::ManyToMany {
            relationship_name,
            junction_collection,
            junction_alias,
            source_key_field,
            junction_local_key,
            junction_foreign_key,
            target_collection,
            target_key_field,
        } => vec![
            doc! {
                "$lookup": {
                    "from": junction_collection,
                    "localField": source_key_field,
                    "foreignField": junction_local_key,
                    "as": junction_alias.clone()
                }
            },
            doc! {
                "$lookup": {
                    "from": target_collection,
                    "localField": format!("{junction_alias}.{junction_foreign_key}"),
                    "foreignField": target_key_field,
                    "as": relationship_name
                }
            },
            doc! { "$unset": junction_alias },
        ],
    }
}

pub fn selection_projection(nodes: &[MongoSelectionProjectionNode]) -> Document {
    let mut projection = doc! { "_id": 0 };
    for node in nodes {
        push_projection_node(&mut projection, node, None);
    }
    projection
}

pub fn query_pipeline(
    mut lookup_stages: Vec<Document>,
    filter_doc: Document,
    sort_doc: Document,
    skip: i32,
    limit: i32,
    projection_doc: Document,
) -> Vec<Document> {
    lookup_stages.push(doc! { "$match": filter_doc });
    let mut item_docs = Vec::new();
    if !sort_doc.is_empty() {
        item_docs.push(doc! { "$sort": sort_doc });
    }
    item_docs.push(doc! { "$skip": skip });
    item_docs.push(doc! { "$limit": limit });
    item_docs.push(doc! { "$project": projection_doc });

    lookup_stages.push(doc! {
        "$facet": {
            "items": item_docs,
            "query_info": [{ "$count": "count" }]
        }
    });
    lookup_stages
}

pub fn get_item_pipeline(
    mut lookup_stages: Vec<Document>,
    filter_doc: Document,
    projection_doc: Document,
) -> Vec<Document> {
    lookup_stages.push(doc! { "$match": filter_doc });
    lookup_stages.push(doc! { "$limit": 1 });
    lookup_stages.push(doc! { "$project": projection_doc });
    lookup_stages
}

pub fn aggregate_group_and_project(
    groups: &[MongoAggregateGroup],
    metrics: &[MongoAggregateMetric],
) -> Result<(Document, Document), RuntimeError> {
    let mut id_doc = Document::new();
    for group in groups {
        id_doc.insert(
            group.alias.clone(),
            Bson::String(format!("${}", group.field_name)),
        );
    }

    let mut group_doc = Document::new();
    group_doc.insert(
        "_id",
        if groups.is_empty() {
            Bson::Null
        } else {
            Bson::Document(id_doc)
        },
    );

    let mut project_doc = doc! { "_id": 0 };
    for group in groups {
        project_doc.insert(
            group.alias.clone(),
            Bson::String(format!("$_id.{}", group.alias)),
        );
    }

    for metric in metrics {
        match metric.function {
            RuntimeAggregateFunction::Count => {
                let value = metric
                    .field_name
                    .as_ref()
                    .map(|field_name| {
                        Bson::Document(doc! {
                            "$cond": [
                              { "$ne": [format!("${field_name}"), Bson::Null] },
                              1,
                              0
                            ]
                        })
                    })
                    .unwrap_or(Bson::Int32(1));
                group_doc.insert(metric.alias.clone(), doc! { "$sum": value });
                project_doc.insert(
                    metric.alias.clone(),
                    Bson::String(format!("${}", metric.alias)),
                );
            }
            RuntimeAggregateFunction::CountDistinct => {
                let temp_alias = distinct_temp_alias(&metric.alias);
                group_doc.insert(
                    temp_alias.clone(),
                    doc! { "$addToSet": format!("${}", metric_field(metric)?) },
                );
                project_doc.insert(
                    metric.alias.clone(),
                    doc! { "$size": format!("${temp_alias}") },
                );
            }
            RuntimeAggregateFunction::Sum
            | RuntimeAggregateFunction::Avg
            | RuntimeAggregateFunction::Min
            | RuntimeAggregateFunction::Max => {
                let op = match metric.function {
                    RuntimeAggregateFunction::Sum => "$sum",
                    RuntimeAggregateFunction::Avg => "$avg",
                    RuntimeAggregateFunction::Min => "$min",
                    RuntimeAggregateFunction::Max => "$max",
                    _ => unreachable!("metric branch narrowed above"),
                };
                group_doc.insert(
                    metric.alias.clone(),
                    doc! { op: format!("${}", metric_field(metric)?) },
                );
                project_doc.insert(
                    metric.alias.clone(),
                    Bson::String(format!("${}", metric.alias)),
                );
            }
        }
    }

    Ok((group_doc, project_doc))
}

pub fn aggregate_having(
    predicates: &[MongoAggregateHavingPredicate],
) -> Result<Document, RuntimeError> {
    let mut match_doc = Document::new();
    for predicate in predicates {
        let value =
            bson::to_bson(&predicate.value).map_err(|e| RuntimeError::DataAccess(e.to_string()))?;
        let criterion = match predicate.op {
            RuntimeFilterOp::Eq => value,
            RuntimeFilterOp::Ne => Bson::Document(doc! { "$ne": value }),
            RuntimeFilterOp::Lt => Bson::Document(doc! { "$lt": value }),
            RuntimeFilterOp::Lte => Bson::Document(doc! { "$lte": value }),
            RuntimeFilterOp::Gt => Bson::Document(doc! { "$gt": value }),
            RuntimeFilterOp::Gte => Bson::Document(doc! { "$gte": value }),
            RuntimeFilterOp::In | RuntimeFilterOp::NotIn => {
                let values = predicate.value.as_array().ok_or_else(|| {
                    RuntimeError::Validation(format!(
                        "having list for '{}' must be an array",
                        predicate.metric_alias
                    ))
                })?;
                let mut bson_values = Vec::with_capacity(values.len());
                for value in values {
                    bson_values.push(
                        bson::to_bson(value)
                            .map_err(|e| RuntimeError::DataAccess(e.to_string()))?,
                    );
                }
                let op = if matches!(predicate.op, RuntimeFilterOp::In) {
                    "$in"
                } else {
                    "$nin"
                };
                Bson::Document(doc! { op: bson_values })
            }
            _ => {
                return Err(RuntimeError::Validation(format!(
                    "unsupported aggregate having operator {:?}",
                    predicate.op
                )))
            }
        };
        match_doc.insert(predicate.metric_alias.clone(), criterion);
    }
    Ok(match_doc)
}

pub fn aggregate_sort(specs: &[MongoAggregateSortSpec]) -> Document {
    let mut sort_doc = Document::new();
    for spec in specs {
        sort_doc.insert(
            spec.alias.clone(),
            if spec.direction == RuntimeSortDirection::Desc {
                -1
            } else {
                1
            },
        );
    }
    sort_doc
}

pub fn aggregate_pipeline(
    filter_doc: Document,
    group_doc: Document,
    project_doc: Document,
    having_doc: Document,
    sort_doc: Document,
    skip: i32,
    limit: i32,
) -> Vec<Document> {
    let mut item_docs = Vec::new();
    if !sort_doc.is_empty() {
        item_docs.push(doc! { "$sort": sort_doc });
    }
    item_docs.push(doc! { "$skip": skip });
    item_docs.push(doc! { "$limit": limit });

    let mut pipeline = vec![
        doc! { "$match": filter_doc },
        doc! { "$group": group_doc },
        doc! { "$project": project_doc },
    ];
    if !having_doc.is_empty() {
        pipeline.push(doc! { "$match": having_doc });
    }
    pipeline.push(doc! {
        "$facet": {
            "items": item_docs,
            "query_info": [{ "$count": "count" }]
        }
    });
    pipeline
}

fn distinct_temp_alias(alias: &str) -> String {
    format!("__distinct_{alias}")
}

fn metric_field(metric: &MongoAggregateMetric) -> Result<&str, RuntimeError> {
    metric.field_name.as_deref().ok_or_else(|| {
        RuntimeError::Validation(format!(
            "{} metric '{}' requires a field",
            metric.function.as_str(),
            metric.alias
        ))
    })
}

fn push_projection_node(
    projection: &mut Document,
    node: &MongoSelectionProjectionNode,
    prefix: Option<&str>,
) {
    let field_name = crate::mongo_field_name(&node.name, node.is_object_id_key);
    let path = prefix
        .map(|prefix| format!("{prefix}.{field_name}"))
        .unwrap_or(field_name);
    if node.children.is_empty() {
        projection.insert(path, 1);
    } else {
        for child in &node.children {
            push_projection_node(projection, child, Some(&path));
        }
    }
}

#[cfg(test)]
mod tests {
    use appfw_runtime::query_ir::{RuntimeAggregateFunction, RuntimeSortDirection};
    use appfw_runtime::RuntimeFilterOp;
    use bson::Bson;
    use serde_json::json;

    use super::*;

    #[test]
    fn combines_filter_and_access_documents() {
        assert_eq!(
            combine_filter_documents(doc! {}, doc! { "tenant_id": "t1" }),
            doc! { "tenant_id": "t1" }
        );

        let combined =
            combine_filter_documents(doc! { "name": "Acme" }, doc! { "tenant_id": "t1" });
        assert!(combined.contains_key("$and"));
    }

    #[test]
    fn builds_query_facet_pipeline() {
        let pipeline = query_pipeline(
            vec![doc! { "$lookup": { "from": "crm.contacts" } }],
            doc! { "name": "Acme" },
            doc! { "name": 1 },
            10,
            25,
            doc! { "_id": 0, "name": 1 },
        );

        assert_eq!(pipeline.len(), 3);
        assert!(pipeline[0].contains_key("$lookup"));
        assert!(pipeline[1].contains_key("$match"));
        assert!(pipeline[2].contains_key("$facet"));
    }

    #[test]
    fn query_facet_pipeline_omits_empty_sort_stage() {
        let pipeline = query_pipeline(
            Vec::new(),
            doc! { "record_locator": "rl_abc" },
            doc! {},
            0,
            1,
            doc! { "_id": 0, "name": 1 },
        );

        let facet = pipeline
            .last()
            .and_then(|stage| stage.get_document("$facet").ok())
            .expect("facet stage");
        let item_stages = facet.get_array("items").expect("items facet");
        assert!(!item_stages
            .iter()
            .filter_map(Bson::as_document)
            .any(|stage| stage.contains_key("$sort")));
    }

    #[test]
    fn builds_relationship_lookup_stages_and_selection_projection() {
        let direct = relationship_lookup_stages(MongoRelationshipLookup::Direct {
            relationship_name: "contacts".to_string(),
            target_collection: "crm.contacts".to_string(),
            local_field: "_id".to_string(),
            foreign_field: "account_id".to_string(),
            flatten_to_one: true,
        });
        assert_eq!(direct.len(), 2);
        assert_eq!(
            direct[0]
                .get_document("$lookup")
                .expect("lookup")
                .get_str("from"),
            Ok("crm.contacts")
        );
        assert!(direct[1].contains_key("$set"));

        let many = relationship_lookup_stages(MongoRelationshipLookup::ManyToMany {
            relationship_name: "opportunities".to_string(),
            junction_collection: "crm.account_opportunities".to_string(),
            junction_alias: "__opportunities_junction".to_string(),
            source_key_field: "_id".to_string(),
            junction_local_key: "account_id".to_string(),
            junction_foreign_key: "opportunity_id".to_string(),
            target_collection: "crm.opportunities".to_string(),
            target_key_field: "_id".to_string(),
        });
        assert_eq!(many.len(), 3);
        assert!(many[2].contains_key("$unset"));

        let projection = selection_projection(&[MongoSelectionProjectionNode {
            name: "contacts".to_string(),
            is_object_id_key: false,
            children: vec![MongoSelectionProjectionNode {
                name: "id".to_string(),
                is_object_id_key: true,
                children: Vec::new(),
            }],
        }]);
        assert_eq!(projection.get("_id"), Some(&Bson::Int32(0)));
        assert_eq!(projection.get("contacts._id"), Some(&Bson::Int32(1)));
    }

    #[test]
    fn builds_aggregate_group_project_having_sort_and_facet() {
        let groups = vec![MongoAggregateGroup {
            alias: "industry".to_string(),
            field_name: "industry_id".to_string(),
        }];
        let metrics = vec![
            MongoAggregateMetric {
                function: RuntimeAggregateFunction::Count,
                alias: "count".to_string(),
                field_name: None,
            },
            MongoAggregateMetric {
                function: RuntimeAggregateFunction::Sum,
                alias: "total_age".to_string(),
                field_name: Some("age".to_string()),
            },
        ];

        let (group, project) = aggregate_group_and_project(&groups, &metrics).expect("aggregate");
        assert!(group.get_document("_id").unwrap().contains_key("industry"));
        assert!(group.get_document("count").unwrap().contains_key("$sum"));
        assert!(group
            .get_document("total_age")
            .unwrap()
            .contains_key("$sum"));
        assert_eq!(
            project.get_str("industry").expect("group project"),
            "$_id.industry"
        );

        let having = aggregate_having(&[MongoAggregateHavingPredicate {
            metric_alias: "total_age".to_string(),
            op: RuntimeFilterOp::Gte,
            value: json!(10),
        }])
        .expect("having");
        assert_eq!(
            having
                .get_document("total_age")
                .expect("having criterion")
                .get("$gte"),
            Some(&Bson::Int64(10))
        );

        let sort = aggregate_sort(&[MongoAggregateSortSpec {
            alias: "total_age".to_string(),
            direction: RuntimeSortDirection::Desc,
        }]);
        assert_eq!(sort.get_i32("total_age"), Ok(-1));

        let pipeline = aggregate_pipeline(doc! {}, group, project, having, sort, 0, 10);
        assert!(pipeline.iter().any(|stage| stage.contains_key("$facet")));
    }
}
