

selections_json: Object {
  "name": String("queryUsers"), 
  "selection_set": Array [
    Object {
      "name": String("dateTime"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("requestDuration"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("skip"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("limit"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("pageIndex"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("pageCount"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("queryCount"), 
      "selection_set": Array []
    }, 
    Object {
      "name": String("items"), 
      "selection_set": Array [
        Object {
          "name": String("id"), 
          "selection_set": Array []
        }, 
        Object {
          "name": String("userName"), 
          "selection_set": Array []
        }, 
        Object {
          "name": String("email"), 
          "selection_set": Array []
        }, 
        Object {
          "name": String("mobile"), 
          "selection_set": Array []
        }
      ]
    }
  ]
}


audit_recs AS 
(
  select a.*
  from sys.users_audit a
  order by a.id
), 
root_cte AS 
(
  select {select_fields},
        json_agg(audit_recs) as audit_recs
  from {schema_name}.{table} {root_alias}
        left join audit_recs on audit_recs.record_id = t0.id
  {filter}
  group by {root_alias}.id
  {order_by}
)





  async fn get_upsert_data(&self, entity_type: Arc<EntityType>, input: Map<String, Value>, is_update: bool) -> SqlUpsertData {

    let mut fields_vec: Vec<String> = vec![];
    let mut values_vec: Vec<String> = vec![];
    let mut params_vec: Vec<Box<dyn ToSql + Sync>> = vec![];

    for attr in input {
      let prop = EntityTypes::get_prop(entity_type.clone(), &attr.0);
      if EntityTypes::is_native_prop(prop.clone()) {
        if is_update || !prop.is_key { // Database generates primary key value
          fields_vec.push(attr.0);
          values_vec.push(format!("${}", params_vec.len() + 1));
          match self.get_prop_input(prop.clone(), &attr.1) {
            Ok(prop_input) => match prop_input {
              Some(prop_input) => match prop_input {
                PropInput::Boolean(v) => params_vec.push(Box::new(v)),
                PropInput::String(v) => params_vec.push(Box::new(v)),
                PropInput::Number(v) => params_vec.push(Box::new(v)),
                PropInput::DateTime(v) => params_vec.push(Box::new(v.to_rfc3339())),
                PropInput::Timestamp(v) => params_vec.push(Box::new(v.to_rfc3339())),
                PropInput::Json(v) => params_vec.push(Box::new(v)),
                PropInput::Object(v) => params_vec.push(Box::new(v)),
                PropInput::ObjectArray(v) => params_vec.push(Box::new(v)),
                PropInput::JsonArray(v) => params_vec.push(Box::new(v)),
              },
              None => params_vec.push(Box::new("null")),
            },
            Err(e) => { eprintln!("{:?}", e); }
          }
          // params.push(.expect("could not get attribute value"));          
        }
      }
    }
  
    SqlUpsertData {
      fields_vec,
      values_vec,
      params_vec,
    }

  }
  
  fn get_prop_input(&self, prop: Arc<PropertyType>, value: &Value) -> Result<Option<PropInput>>
  {
    // println!("---------------------------------------------------");
    // println!("\n > Database: get_prop_input {:?} {:?} {:?}", prop.name, value, prop.data_type);
    Ok(

        match prop.data_type 
        {
          DataType::Boolean => { 
            match value {
              Value::Null => None,
              Value::Bool(v) => Some(PropInput::Boolean(v)),
              _ => todo!(),
            }
          },
          DataType::Uuid => { 
            match value {
              Value::Null => None,
              Value::String(v) => Some(PropInput::String(v.to_owned())),
              _ => todo!(),
            }
          },
          DataType::String => { 
            match value {
              Value::Null => None,
              // Escape single quote (e.g., code editor content)
              // Value::String(v) => format!("'{}'", v.replace("'", "''")), 
              Value::String(v) => Some(PropInput::String(v.to_owned())), 
              _ => todo!(),
            }
          },
          DataType::StringArray | DataType::UuidArray => { 
            match value {
              Value::Null => None,
              Value::Array(v) => {
                // let items = v.iter().map(|item| format!("'{}'", item.as_str().unwrap())).collect::<Vec<_>>();
                let items = v.iter().map(|item| format!("{}", item.as_str().unwrap())).collect::<Vec<_>>();
                Some(PropInput::String(format!("[{}]", items.join(","))))
              },
              _ => todo!(),
            }
          },

          DataType::Enum => { 
            match value {
              Value::Null => None,
              Value::String(v) => Some(PropInput::String(v.to_owned())),
              _ => todo!(),
            }
          },
          DataType::EnumArray => { 
            match value {
              Value::Null => None,
              Value::Array(v) => {
                // let items = v.iter().map(|item| format!("'{}'", item.as_str().unwrap())).collect::<Vec<_>>();
                let items = v.iter().map(|item| format!("{}", item.as_str().unwrap())).collect::<Vec<_>>();
                Some(PropInput::String(format!("[{}]", items.join(","))))
              },
              _ => todo!(),
            }
          },

          DataType::Date => { 
            match value {
              Value::Null => None,
              Value::String(v) => {
                let created_date_time: DateTime<Utc> = DateTime::parse_from_rfc3339(v).unwrap().with_timezone(&Utc);
                Some(PropInput::Timestamp(created_date_time))
              },
              _ => todo!(),
            }
          },
          DataType::DateTime => { 
            match value {
              Value::Null => None,
              Value::String(v) => {
                let created_date_time: DateTime<Utc> = DateTime::parse_from_rfc3339(v).unwrap().with_timezone(&Utc);
                Some(PropInput::Timestamp(created_date_time))
              },
              _ => todo!(),
            }
          },
          DataType::Timestamp => { 
            match value {
              Value::Null => None,
              Value::String(v) => {
                let created_date_time: DateTime<Utc> = DateTime::parse_from_rfc3339(v).unwrap().with_timezone(&Utc);
                Some(PropInput::Timestamp(created_date_time))
              },
              _ => todo!(),
            }
          },
          DataType::Time => { 
            match value {
              Value::Null => None,
              Value::String(v) => Some(PropInput::String(v.to_owned())),
              _ => todo!(),
            }
          },
          DataType::Int16 | DataType::Int32 | DataType::Int64 | DataType::Float32 | DataType::Float64 => { 
            match value {
              Value::Null => None,
              Value::Number(v) => Some(PropInput::Number(format!("{}", v))),
              _ => todo!(),
            }
          },


          DataType::Object => { 
            match value {
              Value::Null => None,
              Value::Object(v) => {
                let obj_entity_type = self.app_config.get_nested_entity_type(prop.clone());
                let obj_flds = self.get_obj_inputs(obj_entity_type.clone(), v);
                let res = format!("row({})::{}.{}", obj_flds.join(","), &obj_entity_type.schema_name, &obj_entity_type.snake_1);
                Some(PropInput::Object(res))
              },
              _ => todo!(),
            }
          },
          DataType::ObjectArray => { 
            match value {
              Value::Null => None,
              Value::Array(v) => {
                let obj_entity_type = self.app_config.get_nested_entity_type(prop.clone());
                let mut row_items: Vec<String> = Vec::new();
                for v_item in v {
                  match v_item {
                    Value::Object(v) => {
                      println!("ObjectArray item value {:?}", v);
                      if v.keys().len() > 0 {
                        let obj_flds = self.get_obj_inputs(obj_entity_type.clone(), v);
                        row_items.push(format!("row({})::{}.{}", obj_flds.join(","), &obj_entity_type.schema_name, &obj_entity_type.snake_1));
                      }
                    },
                    _ => { eprintln!("invalid ObjectArray item value {:?}", v_item); },
                  }
                }
                let res = format!("array[{}]::{}.{}[]", row_items.join(","), &obj_entity_type.schema_name, &obj_entity_type.snake_1);
                Some(PropInput::ObjectArray(res))
              },
              _ => todo!(),
            }
          },
          DataType::Json => { 
            match value {
              Value::Null => None,
              Value::Object(v) => {
                // Escape single quote (e.g., code editor content)
                let s = serde_json::to_string(v).expect("should be valid json").replace("'", "''"); 
                Some(PropInput::Json(format!("'{}'::jsonb", s)))
              },
              _ => todo!(),
            }
          },
          DataType::JsonArray => { 
            match value {
              Value::Null => None,
              Value::Array(v) => {
                let mut array_items: Vec<String> = Vec::new();
                for v_item in v {
                  // Escape single quote (e.g., code editor content)
                  array_items.push(format!("'{}'::jsonb", serde_json::to_string(v_item).expect("should be valid json").replace("'", "''")));
                }
                Some(PropInput::JsonArray(format!("array[{}]::jsonb[]", array_items.join(","))))
              },
              _ => todo!(),
            }
          },

          _ => {
            eprintln!("unsupported data type {:?} for property {:?}", prop.data_type, prop.name);
            None
          }
        }
      
    )

  }

  fn get_obj_inputs(&self, obj_entity_type: Arc<EntityType>, v: &Map<String, Value>) -> Vec<String>
  {
    let mut obj_flds: Vec<String> = Vec::new();
    for prop in &obj_entity_type.props 
    {
      let s = if let Some(p_val) = v.get(&prop.name) {
        match self.get_prop_input(Arc::new(prop.to_owned()), p_val) {
          Ok(prop_input) => prop_input,
          Err(e) => { eprintln!("{:?}", e); String::from("null") },
        }  
      } else { String::from("null") };
      obj_flds.push(s);
    }
    obj_flds
  }






// Function to serialize a serde_json::Map<String, Value> into a PostgreSQL composite type string
pub fn serialize_type(app_config: Arc<AppConfig>, entity_type: Arc<EntityType>, map: &serde_json::Map<String, Value>) -> Result<String, AppError> 
{
  let mut serialized_fields = Vec::new();
  for prop in &entity_type.props {
    let p = Arc::new(prop.clone());
    let value = map.get(&prop.name);
    let serialized_value = match value {
      Some(val) => serialize_value(app_config.clone(), p.clone(), val)?,
      None => 
        if prop.is_required {
          return Err(invalid_null(p.clone()));
        } else {
          "NULL".to_string()
        }
    };
    serialized_fields.push(serialized_value);
  }

  // Join the serialized fields with commas and wrap in parentheses
  //'{}'::jsonb
  // let composite_str = format!("({})", fields.join(","));
  let type_value = format!("({})::{}.{}", serialized_fields.join(","), entity_type.schema_name, entity_type.snake_1);
  Ok(type_value)
}

// Helper function to serialize individual values according to their DataType
pub fn serialize_value(app_config: Arc<AppConfig>, prop: Arc<PropertyType>, value: &Value) -> Result<String, AppError> {
  match prop.data_type {

    DataType::Boolean => match value {
      Value::Bool(v) => Ok(v.to_string()),
      _ => Err(error_from_str(prop.clone(), format!("Expected boolean, got {:?}", value))),
    },

    DataType::String => match value {
      Value::String(v) => Ok(v.to_string()),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::StringArray => match value {
      Value::Array(vec) => concat_str_array(prop.clone(), vec),
      _ => Err(error_from_str(prop.clone(), format!("Expected array, got {:?}", value))),
    },

    DataType::Uuid => match value {
      Value::String(v) => Ok(format!("\"{}\"", escape_postgres_string(v))),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::UuidArray => match value {
      Value::Array(vec) => concat_str_array(prop.clone(), vec),
      _ => Err(error_from_str(prop.clone(), format!("Expected array, got {:?}", value))),
    },

    DataType::Date => match value {
      Value::String(v) => Ok(v.to_string()),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::DateTime => match value {
      Value::String(v) => Ok(v.to_string()),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::Timestamp => match value {
      Value::String(v) => Ok(v.to_string()),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::Time => match value {
      Value::String(v) => Ok(v.to_string()),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => match value {
      Value::Number(n) => {
        if let Some(i) = n.as_i64() {
          Ok(i.to_string())
        } else {
          Err(error_from_str(prop.clone(), format!("Invalid integer value: {:?}", value)))
        }
      }
      _ => Err(error_from_str(prop.clone(), format!("Expected integer, got {:?}", value))),
    },
    DataType::Int8Array | DataType::Int16Array | DataType::Int32Array | DataType::Int64Array => match value {
      Value::Array(vec) => concat_int_array(prop.clone(), vec),
      _ => Err(error_from_str(prop.clone(), format!("Expected array, got {:?}", value))),
    },


    DataType::Float32 | DataType::Float64 => match value {
      Value::Number(n) => {
        if let Some(f) = n.as_f64() {
          Ok(f.to_string())
        } else {
          Err(error_from_str(prop.clone(), format!("Invalid float value: {:?}", value)))
        }
      }
      _ => Err(error_from_str(prop.clone(), format!("Expected float, got {:?}", value))),
    },


    DataType::Enum => match value {
      Value::String(v) => Ok(format!("\"{}\"", escape_postgres_string(v))),
      _ => Err(error_from_str(prop.clone(), format!("Expected string, got {:?}", value))),
    },
    DataType::EnumArray => match value {
      Value::Array(vec) => concat_str_array(prop.clone(), vec),
      _ => Err(error_from_str(prop.clone(), format!("Expected array, got {:?}", value))),
    },


    DataType::Object => match value {
      Value::Object(nested_obj) => match &prop.nested_entity_type {
        Some(net) => {
          let nested_type = app_config.get_entity_type(&net.schema_name, &net.type_name);
          serialize_type(app_config.clone(), nested_type.clone(), &nested_obj)
        },
        None => {
          return Err(error_from_str(prop.clone(), format!("Expected nested_entity_type")));
        },
      }
      _ => Err(error_from_str(prop.clone(), format!("Expected object for composite type, got {:?}", value))),
    },
    DataType::ObjectArray => match value {
      Value::Array(vec) => match &prop.nested_entity_type {
        Some(net) => {
          let nested_type = app_config.get_entity_type(&net.schema_name, &net.type_name);
          let mut serialized_elements = Vec::new();
          for elem in vec {
            match elem {
              Value::Object(nested_obj) => {
                // Serialize each composite element
                let composite_str = serialize_type(app_config.clone(), nested_type.clone(), nested_obj)?;
                // Escape the composite element as an array element
                let escaped = escape_postgres_array_element(&composite_str);
                // Composite elements need to be quoted in arrays
                serialized_elements.push(format!("\"{}\"", escaped));
              }
              _ => {
                return Err(error_from_str(prop.clone(), format!(
                    "Expected object in array of composite types, got {:?}",
                    elem
                )));
              }
            }
          }
          
          let array_str = format!("{{{}}}", serialized_elements.join(","));
          
          Ok(array_str)
        },
        None => {
          return Err(error_from_str(prop.clone(), format!("Expected nested_entity_type")));
        },
      }
      _ => Err(error_from_str(prop.clone(), format!("Expected array, got {:?}", value))),
    }
    


    DataType::Json => match value {
      Value::Object(v) => {
        // let json_str = serde_json::to_string(&v)?;
        // Ok(format!("\"{}\"", escape_postgres_string(&json_str)))
        // Serialize the serde_json::Value into a JSON string
        let json_str = serde_json::to_string(v)
          .map_err(|e| error_from_str(prop.clone(), e.to_string()))?;
        // Escape the JSON string for inclusion in SQL
        let escaped = escape_postgres_string(&json_str);
        // Wrap the escaped string in single quotes
        let sql_value = format!("'{}'", escaped);
        // app_gen postgres templates only use jsonb, type casting with this assumption
        Ok(format!("{}::jsonb", sql_value))
      },
      _ => Err(error_from_str(prop.clone(), format!("Expected json, got {:?}", value))),
    },

    DataType::JsonArray => match value {
      Value::Array(v) => {
        let mut serialized_elements = Vec::new();
        for elem in v {
          let json_str = serde_json::to_string(&elem)
            .map_err(|e| error_from_str(prop.clone(), e.to_string()))?;
          serialized_elements.push(format!("\"{}\"", escape_postgres_string(&json_str)));
        }
        // app_gen postgres templates only use jsonb, type casting with this assumption
        let array_str = format!("{{{}}}::jsonb[]", serialized_elements.join(","));
        Ok(array_str)
      },
      _ => Err(error_from_str(prop.clone(), format!("Expected array, got {:?}", value))),
    },

    DataType::NavToOne => Err(error_from_str(prop.clone(), format!("Invalid data type NavToOne"))),
    DataType::NavToMany => Err(error_from_str(prop.clone(), format!("Invalid data type NavToMany"))),

  }
}


// Function to escape strings for PostgreSQL composite type syntax
fn escape_postgres_string(s: &str) -> String {
  s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_postgres_array_element(s: &str) -> String {
  s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn needs_quoting(s: &str) -> bool {
  s.contains(&[' ', ',', '"', '\\', '{', '}'][..])
}




fn concat_str_array(prop: Arc<PropertyType>, v: &Vec<Value>) -> Result<String, AppError>
{
  let mut serialized_elements = Vec::new();
  for elem in v {
    match elem {
      Value::String(s) => {
        let escaped = escape_postgres_array_element(s);
        // Strings with special characters need to be quoted
        if needs_quoting(s) {
          serialized_elements.push(format!("\"{}\"", escaped));
        } else {
          serialized_elements.push(escaped);
        }
      }
      _ => {
        return Err(error_from_str(prop.clone(), format!("Expected string in array, got {:?}", elem)));
      },
    }
  }
  let array_str = format!("{{{}}}", serialized_elements.join(","));
  Ok(array_str)
}

fn concat_int_array(prop: Arc<PropertyType>, v: &Vec<Value>) -> Result<String, AppError>
{
  let mut serialized_elements = Vec::new();
  for elem in v {
    match elem {
      Value::Number(n) => {
        if let Some(i) = n.as_i64() {
          serialized_elements.push(i.to_string());
        } else {
          return Err(error_from_str(prop.clone(), format!("Invalid integer in array: {:?}", elem)));
        }
      }
      Value::Null => {
        serialized_elements.push("NULL".to_string());
      }
      _ => return Err(error_from_str(prop.clone(), format!("Expected integer in array, got {:?}", elem))),
    }
  }
  let array_str = format!("{{{}}}", serialized_elements.join(","));
  Ok(array_str)
}


fn escape_postgres_array_element(s: &str) -> String {
  s.replace('\\', "\\\\").replace('"', "\\\"")
}


fn needs_quoting(s: &str) -> bool {
  s.contains(&[' ', ',', '"', '\\', '{', '}'][..])
}