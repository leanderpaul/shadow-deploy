use std::collections::BTreeMap;

use async_once::AsyncOnce;
use aws_sdk_sqs::types::MessageSystemAttributeName::MessageGroupId;
use aws_sdk_sqs::types::QueueAttributeName;
use serde_json::Value;

use crate::config;
use crate::utils;

struct AWSClient {
  sqs: aws_sdk_sqs::Client,
  secretsmanager: aws_sdk_secretsmanager::Client,
}

async fn get_client() -> &'static AWSClient {
  lazy_static! {
    static ref CLIENT: AsyncOnce<AWSClient> = AsyncOnce::new(async {
      let config = aws_config::load_from_env().await;
      let sqs = aws_sdk_sqs::Client::new(&config);
      let secretsmanager = aws_sdk_secretsmanager::Client::new(&config);
      AWSClient { sqs, secretsmanager }
    });
  }

  return CLIENT.get().await;
}

pub async fn get_event(queue_name: &str) -> Option<String> {
  let aws = get_client().await;
  let message_group_attribute = QueueAttributeName::from("MessageGroupId");
  let result = aws.sqs.receive_message().queue_url(queue_name).attribute_names(message_group_attribute);
  match result.send().await {
    Ok(output) => {
      let message = output.messages().unwrap_or_default().first();
      if message.is_none() {
        return None;
      }
      let message = message.unwrap();

      let group_id = config::get().group.clone();
      if group_id.is_some() {
        let group_id = group_id.unwrap();
        let message_group_id = message.attributes().unwrap().get(&MessageGroupId).unwrap();
        if !group_id.eq(message_group_id) {
          return None;
        }
      }

      let receipt = message.receipt_handle().unwrap();
      let _ = aws.sqs.delete_message().queue_url(queue_name).receipt_handle(receipt).send().await;
      return Some(message.body().unwrap().to_string());
    }
    Err(error) => {
      log!("Error: {:?}", error);
      return None;
    }
  };
}

pub async fn get_secrets(names: &Vec<String>) -> BTreeMap<String, String> {
  let aws = get_client().await;
  let mut secrets: BTreeMap<String, String> = BTreeMap::new();

  for name in names.iter() {
    let result = aws.secretsmanager.get_secret_value().secret_id(name);
    match result.send().await {
      Ok(output) => {
        let secret = output.secret_string().unwrap_or_default();
        let secret = serde_json::from_str::<Value>(&secret).unwrap_or_default();
        let prefix = format!("secrets.{}", name);
        utils::flatten_json(&secret, &prefix, &mut secrets);
      }
      Err(error) => {
        log!("Error: {:?}", error);
      }
    };
  }

  return secrets;
}
