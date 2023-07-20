use std::collections::BTreeMap;
use std::process;

use serde_json::Value;

pub fn exit(msg: &str) -> ! {
  log!("{}", msg);
  process::exit(1);
}

pub fn flatten_json(json: &Value, prefix: &str, flattened: &mut BTreeMap<String, String>) -> BTreeMap<String, String> {
  match json {
    Value::Object(obj) => {
      for (key, value) in obj {
        let new_prefix = if prefix.is_empty() {
          key.to_owned()
        } else {
          format!("{}.{}", prefix, key)
        };
        flatten_json(value, &new_prefix, flattened);
      }
    }
    Value::Array(arr) => {
      for (index, value) in arr.iter().enumerate() {
        let new_prefix = format!("{}[{}]", prefix, index);
        flatten_json(value, &new_prefix, flattened);
      }
    }
    Value::String(val) => {
      flattened.insert(prefix.to_owned(), val.to_owned());
    }
    _ => {
      flattened.insert(prefix.to_owned(), json.to_string());
    }
  }

  return flattened.clone();
}
