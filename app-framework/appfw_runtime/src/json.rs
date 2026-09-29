pub type JsonObj = crate::RuntimeJsonObj;

pub fn t_to_json_obj<T>(value: T) -> JsonObj
where
    T: Send + Sync + async_graphql::InputType + serde::Serialize + std::fmt::Debug,
{
    let value_json = serde_json::to_value(&value).expect("could not serialize value to JSON");
    value_json
        .as_object()
        .expect("serialized value should be a JSON object")
        .to_owned()
}

pub fn json_obj_to_t<T>(json_obj: JsonObj) -> T
where
    T: Send + Sync + async_graphql::OutputType + for<'de> serde::Deserialize<'de> + std::fmt::Debug,
{
    serde_json::from_value::<T>(json_obj.into()).expect("could not deserialize JSON object")
}
