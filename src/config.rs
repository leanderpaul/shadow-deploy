use std::fs;

use crate::utils;

const CONFIG_FILE_PATH: &str = "/etc/shadow-deploy.conf";

pub struct Event {
  pub name: String,
  pub cwd: String,
  pub secrets: Vec<String>,
  pub commands: Vec<String>,
}

pub struct Config {
  pub queue_name: String,
  pub group: Option<String>,
  pub interval: u64,
  pub events: Vec<Event>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct YamlEvent {
  name: String,
  description: Option<String>,
  cwd: Option<String>,
  commands: Vec<String>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct YamlConfig {
  queue_name: String,
  group: Option<String>,
  cwd: Option<String>,
  interval: Option<u64>,
  events: Option<Vec<YamlEvent>>,
}

pub fn get() -> &'static Config {
  lazy_static! {
    static ref CONFIG: Config = {
      let config = fs::read_to_string(CONFIG_FILE_PATH).unwrap_or_else(|_| utils::exit("Error reading the config file"));
      let config = serde_yaml::from_str::<YamlConfig>(&config).unwrap_or_else(|_| utils::exit("Error parsing the config file"));

      let queue_name = config.queue_name;
      let group = config.group;
      let interval = config.interval.unwrap_or(2000);
      let events = match config.events {
        Some(events) => events.iter().map(|e| parse_yaml_event(&config.cwd, &e)).collect(),
        None => vec![],
      };

      Config { queue_name, group, interval, events }
    };
  }

  return &CONFIG;
}

fn parse_yaml_event(base_cwd: &Option<String>, yaml_event: &YamlEvent) -> Event {
  let name = yaml_event.name.clone();
  let cwd = yaml_event.cwd.clone().unwrap_or(base_cwd.clone().unwrap_or("/home/ubuntu".to_string()));
  let commands = yaml_event.commands.clone();
  let secrets = get_required_secrets(&commands);
  return Event { name, cwd, commands, secrets };
}

fn get_required_secrets(commands: &Vec<String>) -> Vec<String> {
  let mut secrets: Vec<String> = vec![];
  for command in commands {
    if !command.contains("{{secrets.") {
      continue;
    }

    let words = command.split(".").collect::<Vec<&str>>();
    for (index, word) in words.iter().enumerate() {
      if word.ends_with("{{secrets") {
        if let Some(secret) = words.get(index + 1) {
          secrets.push(secret.to_string());
        }
      }
    }
  }

  return secrets;
}
