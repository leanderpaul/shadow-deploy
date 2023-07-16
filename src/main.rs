use cloud::AWSClient;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

mod cloud;
mod utils;

const CONFIG_FILE_PATH: &str = "/etc/shadow-deploy.conf";

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct Config {
    queue_name: String,
    cwd: String,
    commands: Vec<String>,
    region: Option<String>,
    interval: Option<u64>,
}

#[tokio::main]
async fn main() {
    let config = fs::read_to_string(CONFIG_FILE_PATH).unwrap_or_else(|_| utils::exit("Error reading config file"));
    let config = serde_yaml::from_str::<Config>(&config).unwrap_or_else(|_| utils::exit("Error parsing config file"));

    let interval = config.interval.unwrap_or(2000);
    let client = AWSClient::new(config.region.clone()).await;

    loop {
        let message = client.receive_message(&config.queue_name).await;
        handle_message(message, &config, &client).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
    }
}

async fn handle_message(message: Option<String>, config: &Config, client: &AWSClient) {
    if message.is_none() {
        return;
    }
    let message = message.unwrap();
    log!("Update deployment started");
    let mut secrets_cache: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for command in &config.commands {
        let command = command.replace("{{message}}", &message);
        log!("sh -c {}", command);

        let command = resolve_secrets(command, &mut secrets_cache, &client).await;
        let ouput = Command::new("sh").arg("-c").arg(command).current_dir(&config.cwd).output();
        if ouput.is_err() {
            log!("Error running command");
            log!("{:?}", ouput.err());
            return;
        }

        let ouput = ouput.unwrap();
        let msg_buf = match ouput.status.success() {
            true => ouput.stdout,
            false => ouput.stderr,
        };
        let msg = String::from_utf8(msg_buf).unwrap();
        let msg = msg.trim();
        if msg.len() > 0 {
            log!("{}", msg);
        }
        if !ouput.status.success() {
            log!("Failed to update deployment, aborting");
            return;
        }
    }
    log!("deployment updated successfully");
}

async fn resolve_secrets(command: String, secrets_cache: &mut HashMap<String, Vec<(String, String)>>, client: &AWSClient) -> String {
    let mut resolved_command = String::new();
    let words: Vec<&str> = command.split("{{").flat_map(|s| s.split("}}")).collect();
    for word in words {
        if !word.starts_with("secrets.") {
            resolved_command.push_str(word);
            continue;
        }
        let word = &word[8..word.len()];
        let query: Vec<&str> = word.split(".").collect();
        let name = query.get(0).unwrap().to_owned();
        let query = query.get(1).unwrap().to_owned();

        let secret = match secrets_cache.get(name) {
            Some(secret) => secret.clone(),
            None => {
                let secret = client.get_secret(name).await;
                secrets_cache.insert(name.to_string(), secret.clone());
                secret
            }
        };
        match secret.iter().find(|(k, _)| k == &query) {
            Some((_, value)) => resolved_command.push_str(value),
            None => resolved_command.push_str(""),
        }
    }
    return resolved_command;
}
