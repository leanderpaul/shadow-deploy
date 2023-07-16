use crate::log;
use aws_sdk_sqs::config::Region;
use serde_json::{Map, Value};

pub struct AWSClient {
    sqs: aws_sdk_sqs::Client,
    secretsmanager: aws_sdk_secretsmanager::Client,
}

impl AWSClient {
    pub async fn new(region: Option<String>) -> Self {
        let region = region.unwrap_or("ap-south-1".to_string());
        let region = Region::new(region);
        let config = aws_config::from_env().region(region).load().await;
        Self {
            sqs: aws_sdk_sqs::Client::new(&config),
            secretsmanager: aws_sdk_secretsmanager::Client::new(&config),
        }
    }

    pub async fn receive_message(&self, queue_name: &str) -> Option<String> {
        let result = self.sqs.receive_message().queue_url(queue_name);
        match result.send().await {
            Ok(output) => {
                let message = output.messages().unwrap_or_default().first();
                if let Some(message) = message {
                    let receipt = message.receipt_handle().unwrap();
                    let _ = self.sqs.delete_message().queue_url(queue_name).receipt_handle(receipt).send().await;
                    return Some(message.body().unwrap().to_string());
                }
                return None;
            }
            Err(error) => {
                log!("Error: {:?}", error);
                return None;
            }
        };
    }

    pub async fn get_secret(&self, name: &str) -> Vec<(String, String)> {
        let result = self.secretsmanager.get_secret_value().secret_id(name);
        match result.send().await {
            Ok(output) => {
                let secret = output.secret_string().unwrap_or_default();
                let secret = serde_json::from_str::<Map<String, Value>>(&secret).unwrap_or_default();
                return secret.iter().map(|(k, v)| (k.to_owned(), v.to_string())).collect();
            }
            Err(error) => {
                log!("Error: {:?}", error);
                return Vec::new();
            }
        };
    }
}
