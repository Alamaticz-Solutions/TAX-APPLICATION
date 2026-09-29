use std::sync::Arc;
use std::time::Instant;

use appfw_provider_postgres::{
    cte_definition_sql as provider_cte_definition_sql,
    cte_json_agg_expr as provider_cte_json_agg_expr,
    cte_junction_source_join as provider_cte_junction_source_join,
    cte_junction_target_join as provider_cte_junction_target_join,
    cte_navigation_join as provider_cte_navigation_join, order_by as provider_order_by,
    postgres_physical_table_name as provider_physical_table_name, ManyToManyConfig,
    PostgresCteDefinition, PostgresCteJunctionJoin, PostgresCteNavigationJoin, PostgresSortField,
    SqlParam,
};
use appfw_runtime::{extension::UserAuth, query_ir::parse_sort_specs, AccessAction, PolicyAccess};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::{
    config::app_config::AppConfig,
    routes::app_error::{AppError, MetadataError},
    schemas::system::{DataType, EntityType},
};

use super::filter;

/// Enhanced CTE structure with performance metadata
#[derive(Clone, Debug)]
pub struct CTE {
    pub name: String,
    pub def: String,
    pub performance_metadata: Option<CTEPerformanceMetadata>,
}

/// Performance metadata for CTE queries
#[derive(Clone, Debug)]
pub struct CTEPerformanceMetadata {
    pub has_many_to_many: bool,
    pub many_to_many_count: i32,
    pub estimated_complexity: CTEComplexity,
    pub generation_time_ms: u128,
}

#[derive(Clone, Debug)]
pub enum CTEComplexity {
    Low,    // No relationships or simple NavToOne
    Medium, // NavToMany or single ManyToMany
    High,   // Multiple ManyToMany relationships
}

// let root_alias = "t0".to_string();
impl CTE {
    pub fn new(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        selection_parent: serde_json::Value,
        filter: Option<Value>,
        sort: Option<Value>,
        user: &UserAuth,
        params: &mut Vec<SqlParam>,
    ) -> Result<CTE, AppError> {
        let start_time = Instant::now();
        let root_alias = "t0".to_string();
        let config = ManyToManyConfig::default();
        let mut visited_entities = std::collections::HashSet::new();

        let cte = Self::create(
            app_config.clone(),
            entity_type.clone(),
            selection_parent.to_owned(),
            &root_alias,
            filter,
            sort,
            user,
            params,
            &config,
            &mut visited_entities,
            0,
        )?;

        // Add performance metadata
        if config.enable_performance_monitoring {
            let generation_time = start_time.elapsed().as_millis();
            cte.trace_performance_metadata(&entity_type.pascal_1, generation_time);
            if generation_time > 100 {
                warn!(
                    entity = %entity_type.pascal_1,
                    duration_ms = generation_time,
                    "slow PostgreSQL CTE generation"
                );
            }
        }

        Ok(cte)
    }

    fn trace_performance_metadata(&self, entity: &str, total_generation_time_ms: u128) {
        if let Some(metadata) = self.performance_metadata.as_ref() {
            info!(
                entity,
                cte = %self.name,
                has_many_to_many = metadata.has_many_to_many,
                many_to_many_count = metadata.many_to_many_count,
                estimated_complexity = ?metadata.estimated_complexity,
                cte_generation_time_ms = metadata.generation_time_ms,
                total_generation_time_ms,
                "PostgreSQL CTE generation metrics"
            );
        }
    }

    #[tracing::instrument(
        skip(app_config, entity_type, selection_parent, filter, sort, user, params, config, visited_entities),
        fields(entity = %entity_type.pascal_1, alias = %alias, depth = recursion_depth)
    )]
    fn create(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        selection_parent: serde_json::Value,
        alias: &String,
        filter: Option<Value>,
        sort: Option<Value>,
        user: &UserAuth,
        params: &mut Vec<SqlParam>,
        config: &ManyToManyConfig,
        visited_entities: &mut std::collections::HashSet<String>,
        recursion_depth: u32,
    ) -> Result<CTE, AppError> {
        let start_time = Instant::now();

        // Check for circular references and recursion depth
        let entity_key = format!("{}.{}", entity_type.schema_name, entity_type.pascal_1);

        if recursion_depth > 10 {
            error!(
                "Maximum recursion depth exceeded (10) for entity: {}",
                entity_key
            );
            return Err(AppError::InternalServerError(
                anyhow::anyhow!(
                    "Maximum recursion depth exceeded for entity: {}",
                    entity_key
                )
                .into(),
            ));
        }

        if visited_entities.contains(&entity_key) {
            warn!(
                "Circular reference detected for entity: {} at depth {}. Skipping to prevent infinite recursion.",
                entity_key,
                recursion_depth
            );
            // Return a minimal CTE to break the cycle
            return Ok(CTE {
                name: format!("{}_circular", alias),
                def: format!("SELECT NULL as id WHERE FALSE"), // Empty result set
                performance_metadata: None,
            });
        }

        visited_entities.insert(entity_key.clone());

        debug!(
            "Creating CTE for entity: {}, alias: {}, depth: {}",
            entity_type.pascal_1, alias, recursion_depth
        );

        let access =
            app_config.evaluate_user_access(entity_type.clone(), AccessAction::Read, user)?;

        if !access.allow {
            return Err(AppError::AccessDenied);
        }

        let cte_name = format!("{alias}_cte");
        let mut table_cols_vec: Vec<String> = vec![];
        let mut ref_cte_defs: Vec<String> = vec![];
        let mut ref_cte_selects: Vec<String> = vec![];
        let mut ref_cte_joins: Vec<String> = vec![];

        // Performance tracking
        let mut many_to_many_count = 0;
        let mut has_many_to_many = false;

        let selection_set = selection_parent
            .get("selection_set")
            .and_then(|selection_set| selection_set.as_array())
            .ok_or_else(|| {
                MetadataError::InvalidSelection("selection_set must be an array".to_string())
            })?;

        for selection in selection_set {
            // println!("\n > create_cte: selection {:?}", selection);
            let sel_name = selection
                .get("name")
                .and_then(|name| name.as_str())
                .ok_or_else(|| {
                    MetadataError::InvalidSelection("selection name must be a string".to_string())
                })?
                .to_string();
            let prop = AppConfig::get_prop(entity_type.clone(), &sel_name)?;
            match prop.data_type {
                DataType::NavToOne | DataType::NavToMany => {
                    let nav = prop.nav_by_fk_property.clone().ok_or_else(|| {
                        MetadataError::MissingNavigation {
                            entity_type: entity_type.pascal_1.clone(),
                            property_name: prop.name.clone(),
                        }
                    })?;
                    let ref_entity_type =
                        app_config.get_nav_entity_type(entity_type.clone(), prop.clone())?;

                    // Result will be None if EntityType (table-level) access is denied
                    let ref_cte = Self::create(
                        app_config.clone(),
                        ref_entity_type.clone(),
                        selection.to_owned(),
                        &format!(
                            "{}_{}_{}",
                            ref_entity_type.snake_n, prop.name, entity_type.snake_n
                        ),
                        None,
                        None,
                        user,
                        params,
                        config,
                        visited_entities,
                        recursion_depth + 1,
                    )?;

                    ref_cte_defs.push(ref_cte.def);
                    ref_cte_selects.push(provider_cte_json_agg_expr(&ref_cte.name, &prop.name));

                    if nav.type_name == entity_type.pascal_1
                        && nav.schema_name == entity_type.schema_name
                    {
                        // Current entity_type (alias) has the foreign_key
                        // This means we're navigating FROM the entity that has the FK TO the referenced entity
                        // Join: current_table.fk_field = referenced_table.pk_field
                        let ref_pk_name =
                            app_config.get_primary_key_name(ref_entity_type.clone())?;
                        ref_cte_joins.push(provider_cte_navigation_join(
                            &PostgresCteNavigationJoin {
                                cte_name: ref_cte.name.clone(),
                                left_alias: alias.clone(),
                                left_key: nav.prop_name.clone(),
                                right_alias: ref_cte.name.clone(),
                                right_key: ref_pk_name,
                            },
                        ));
                    } else {
                        // Related entity_type has the foreign_key to current entity_type (alias)
                        // This means we're navigating FROM the referenced entity TO the entity that has the FK
                        // Join: related_table.fk_field = current_table.pk_field
                        let curr_pk_name = app_config.get_primary_key_name(entity_type.clone())?;
                        ref_cte_joins.push(provider_cte_navigation_join(
                            &PostgresCteNavigationJoin {
                                cte_name: ref_cte.name.clone(),
                                left_alias: ref_cte.name.clone(),
                                left_key: nav.prop_name.clone(),
                                right_alias: alias.clone(),
                                right_key: curr_pk_name,
                            },
                        ));
                    }
                }
                DataType::ManyToMany => {
                    if let Some(many_to_many) = &prop.many_to_many_property {
                        has_many_to_many = true;
                        many_to_many_count += 1;

                        debug!(
                            "Processing ManyToMany relationship: {} -> {}",
                            entity_type.pascal_1, many_to_many.target_type
                        );

                        let target_entity_type = app_config.get_entity_type_by_name(
                            &many_to_many.target_schema,
                            &many_to_many.target_type,
                        )?;

                        // Create CTE for the target entity with pagination support and circular reference detection
                        let target_cte = Self::create(
                            app_config.clone(),
                            target_entity_type.clone(),
                            selection.to_owned(),
                            &format!(
                                "{}_{}_{}",
                                target_entity_type.snake_n, prop.name, entity_type.snake_n
                            ),
                            None,
                            None,
                            user,
                            params,
                            config,
                            visited_entities,
                            recursion_depth + 1,
                        )?;

                        ref_cte_defs.push(target_cte.def);

                        // JSON aggregation with pagination support - using COALESCE and FILTER to handle NULL values
                        let json_agg_expr =
                            provider_cte_json_agg_expr(&target_cte.name, &prop.name);

                        ref_cte_selects.push(json_agg_expr);

                        // Create the junction table join with performance hints
                        let junction_schema = many_to_many
                            .junction_schema
                            .as_ref()
                            .unwrap_or(&entity_type.schema_name);
                        let junction_table =
                            provider_physical_table_name(&many_to_many.junction_table);
                        let local_key = &many_to_many.local_key;
                        let foreign_key = &many_to_many.foreign_key;

                        let curr_pk_name = app_config.get_primary_key_name(entity_type.clone())?;
                        let target_pk_name =
                            app_config.get_primary_key_name(target_entity_type.clone())?;
                        let junction_join = PostgresCteJunctionJoin {
                            junction_schema: junction_schema.to_string(),
                            junction_table: junction_table.clone(),
                            relationship_name: prop.name.clone(),
                            source_alias: alias.clone(),
                            source_pk: curr_pk_name.clone(),
                            local_key: local_key.clone(),
                            target_cte: target_cte.name.clone(),
                            target_pk: target_pk_name.clone(),
                            foreign_key: foreign_key.clone(),
                        };

                        // Optimized junction table joins with proper indexing hints
                        ref_cte_joins.push(provider_cte_junction_source_join(&junction_join));

                        // Second join: junction table -> target entity with performance optimization
                        ref_cte_joins.push(provider_cte_junction_target_join(&junction_join));

                        // Log performance warning for complex ManyToMany relationships
                        if config.enable_performance_monitoring && many_to_many_count > 3 {
                            warn!(
                  "Entity {} has {} ManyToMany relationships, consider query optimization",
                  entity_type.pascal_1,
                  many_to_many_count
                );
                        }
                    }
                }
                _ => {
                    table_cols_vec.push(format!("{}.{}", alias, prop.name.to_owned()));
                }
            }
        }

        // Always include the primary key column. Nested CTE joins and JSON aggregation need a
        // stable key column even when the caller did not request it.
        let pk_name = app_config.get_primary_key_name(entity_type.clone())?;
        let pk_col = format!("{}.{}", alias, pk_name);
        if !table_cols_vec.iter().any(|c| c == &pk_col) {
            table_cols_vec.insert(0, pk_col);
        }

        let select_fields = table_cols_vec.join(",");

        let filter_str = Self::create_cte_filter(
            app_config.clone(),
            entity_type.clone(),
            alias,
            filter,
            access,
            params,
        )?;
        // println!("\n >    filter_str: {:?}", filter_str);

        let sort_str = provider_order_by(
            alias,
            &pk_name,
            &postgres_sort_fields(entity_type.clone(), sort)?,
        );
        // println!("\n >    sort: {:?}", sort);

        debug!(sort_clause = %sort_str, "compiled CTE sort clause");

        let ref_cte_select_count = ref_cte_selects.len();
        let limit_clause =
            if config.enable_pagination && config.max_related_entities > 0 && alias != "t0" {
                format!("LIMIT {}", config.max_related_entities)
            } else {
                String::new()
            };
        let cte_def = provider_cte_definition_sql(&PostgresCteDefinition {
            ref_cte_defs,
            cte_name: cte_name.clone(),
            select_fields,
            ref_cte_selects,
            schema_name: entity_type.schema_name.clone(),
            table: entity_type.snake_n.clone(),
            alias: alias.to_string(),
            ref_cte_joins,
            filter_clause: filter_str,
            pk_name: pk_name.clone(),
            sort_clause: sort_str,
            limit_clause,
            performance_many_to_many_count: if has_many_to_many
                && config.enable_performance_monitoring
            {
                Some(many_to_many_count)
            } else {
                None
            },
        });

        // Calculate complexity
        let complexity = if many_to_many_count > 2 {
            CTEComplexity::High
        } else if many_to_many_count > 0 || ref_cte_select_count > 0 {
            CTEComplexity::Medium
        } else {
            CTEComplexity::Low
        };

        // Create performance metadata
        let performance_metadata = if config.enable_performance_monitoring {
            Some(CTEPerformanceMetadata {
                has_many_to_many,
                many_to_many_count,
                estimated_complexity: complexity.clone(),
                generation_time_ms: start_time.elapsed().as_millis(),
            })
        } else {
            None
        };

        // Log performance information
        if config.enable_performance_monitoring {
            let generation_time = start_time.elapsed().as_millis();
            info!(
                "CTE generated for {}: complexity={:?}, many_to_many_count={}, time={}ms",
                entity_type.pascal_1, complexity, many_to_many_count, generation_time
            );
        }

        // Clean up visited entities for this branch
        visited_entities.remove(&entity_key);

        Ok(CTE {
            name: cte_name,
            def: cte_def,
            performance_metadata,
        })
    }

    #[tracing::instrument(
    skip(app_config, entity_type, filter, access, params),
    fields(entity = %entity_type.pascal_1, alias = %alias, has_access_filter = access.filter.is_some())
  )]
    fn create_cte_filter(
        app_config: Arc<AppConfig>,
        entity_type: Arc<EntityType>,
        alias: &String,
        filter: Option<Value>,
        access: PolicyAccess,
        params: &mut Vec<SqlParam>,
    ) -> Result<String, AppError> {
        let base_filter_str = filter::try_create_filter(
            app_config.clone(),
            entity_type.clone(),
            filter,
            alias,
            params,
        )?;
        let access_filter_str = filter::try_create_filter(
            app_config.clone(),
            entity_type.clone(),
            access.filter,
            alias,
            params,
        )?;

        let filters = [base_filter_str, access_filter_str]
            .into_iter()
            .flatten()
            .filter(|filter| !filter.trim().is_empty())
            .collect::<Vec<_>>();

        let res = if filters.is_empty() {
            String::from("")
        } else {
            format!("WHERE {}", filters.join(" AND "))
        };

        Ok(res)
    }
}

fn postgres_sort_fields(
    entity_type: Arc<EntityType>,
    sort: Option<Value>,
) -> Result<Vec<PostgresSortField>, AppError> {
    parse_sort_specs(sort)
        .map_err(AppError::from)?
        .into_iter()
        .map(|spec| {
            let prop = AppConfig::get_prop(entity_type.clone(), &spec.field)?;
            Ok(PostgresSortField {
                name: prop.name.clone(),
                direction: spec.direction,
            })
        })
        .collect()
}
