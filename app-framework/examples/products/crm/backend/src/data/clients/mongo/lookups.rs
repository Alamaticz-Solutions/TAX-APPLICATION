use mongodb::bson::{doc, Document};
use tracing::debug;


fn create_lookups(
  parent: &serde_json::Value,
  from_collection: String,
  lookups: Vec<FieldLookup>
) -> Vec<Document>
{
  debug!(collection = %from_collection, "creating MongoDB lookups");
  let mut result: Vec<Document> = Vec::new();

  let from_collection_str = from_collection.as_str();

  let sel_set = parent.get("selection_set").unwrap();
  let selection_set = sel_set.as_array().unwrap().to_owned();
  for child in selection_set
  {

    let child_sel_set = child.get("selection_set").unwrap();
    let child_selection_set = child_sel_set.as_array().unwrap().to_owned();

    if child_selection_set.len() > 0
    {

      let child_name = child.get("name").unwrap().as_str().unwrap();
      debug!(selection = %child_name, "processing MongoDB lookup selection");

      let x = &lookups;
      let mut iterator = x.iter();

      // pub struct FieldLookup {
      //   pub from_lookup_field: String,
      //   pub from_collection: String,
      //   pub from_key_field: String,
      //   pub to_collection: String,
      //   pub to_id_field: String,
      // }

      if let Some(lookup) = iterator.find(|&l| {
        l.from_collection.eq(from_collection_str) && l.from_lookup_field.eq(child_name)
      }) {
        debug!(lookup_field = %lookup.from_lookup_field, "found MongoDB lookup");
        result.push(doc! {
          "$lookup": {
            "from": lookup.to_collection.to_string(),
            "localField": lookup.from_key_field.to_string(),
            "foreignField": lookup.to_id_field.to_string(),
            "as": lookup.from_lookup_field.to_string(),
          }
        });
        result.append(&mut create_lookups(&child, lookup.to_collection.to_string()).into());
      }
    }
  }
  result
}
