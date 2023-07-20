use std::collections::BTreeMap;
use std::process::Command;

use serde_json::Map;
use serde_json::Value;

use crate::cloud;
use crate::config;
use crate::utils;

pub async fn handle_event(event: String) {
  let event = parse_event(event);
  if event.is_none() {
    return;
  }
  let event = event.unwrap();
  let event_name = event.get("event.name").unwrap();
  let config = config::get();
  let event_config = config.events.iter().find(|e| e.name.eq(event_name));
  if event_config.is_none() {
    log!("Unknown Event '{}' received", event_name);
    return;
  }

  log!("Event '{}' received", event_name);
  let event_config = event_config.unwrap();
  let secrets = cloud::get_secrets(&event_config.secrets).await;

  for command in &event_config.commands {
    let command = resolve_variables(command, &event);
    log!("sh -c {}", command);

    let command = resolve_variables(&command, &secrets);
    let ouput = Command::new("sh").arg("-c").arg(command).current_dir(&event_config.cwd).output();
    if ouput.is_err() {
      log!("Unexpected Error running command");
      log!("{:?}", ouput.err());
      return;
    }

    let ouput = ouput.unwrap();
    let msg_buf = if ouput.status.success() { ouput.stdout } else { ouput.stderr };
    let msg = String::from_utf8(msg_buf).unwrap();
    let msg = msg.trim();
    if msg.len() > 0 {
      log!("{}", msg);
    }
    if !ouput.status.success() {
      log!("Failed to handle event, aborting");
      return;
    }
  }
}

fn parse_event(event: String) -> Option<BTreeMap<String, String>> {
  let object = serde_json::from_str::<Map<String, Value>>(&event);
  if object.is_err() {
    log!("Error parsing event - {:?}", object.err());
    return None;
  }
  let object = object.unwrap();
  let event_name = object.get("name");
  if event_name.is_none() {
    log!("Error parsing event - required field 'name' not found");
    return None;
  }
  let event_name = event_name.unwrap();
  if !event_name.is_string() {
    log!("Error parsing event - 'name' should be a string");
    return None;
  }

  let json = Value::Object(object);
  let map = utils::flatten_json(&json, "event", &mut BTreeMap::new());
  return Some(map);
}

fn resolve_variables(command: &str, payload: &BTreeMap<String, String>) -> String {
  let words = command.split("{{").flat_map(|s| s.split("}}")).collect::<Vec<&str>>();

  let mut command = String::new();
  for (i, word) in words.iter().enumerate() {
    let word = *word;
    let default = format!("{{{{{}}}}}", word);
    let resolved_word = if i % 2 == 0 {
      word
    } else {
      payload.get(word).unwrap_or_else(|| &default)
    };
    command.push_str(resolved_word);
  }

  return command;
}
