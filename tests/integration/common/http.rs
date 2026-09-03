use super::constants::{API_KEY, BASE_URL};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

pub fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("build reqwest client")
}

pub struct Resp {
    pub status: StatusCode,
    pub body: Value,
}

impl Resp {
    pub fn code(&self) -> &str {
        self.body.get("code").and_then(Value::as_str).unwrap_or("")
    }
    pub fn data(&self) -> &Value {
        &self.body["data"]
    }
}

async fn parse(resp: reqwest::Response) -> Resp {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let body = serde_json::from_str(&text).unwrap_or(Value::Null);
    Resp { status, body }
}

pub async fn get(c: &Client, path: &str) -> Resp {
    parse(
        c.get(format!("{}{}", BASE_URL, path))
            .bearer_auth(API_KEY)
            .send()
            .await
            .expect("GET request"),
    )
    .await
}

pub async fn post_json(c: &Client, path: &str, body: Value) -> Resp {
    parse(
        c.post(format!("{}{}", BASE_URL, path))
            .bearer_auth(API_KEY)
            .json(&body)
            .send()
            .await
            .expect("POST request"),
    )
    .await
}

pub async fn patch_json(c: &Client, path: &str, body: Value) -> Resp {
    parse(
        c.patch(format!("{}{}", BASE_URL, path))
            .bearer_auth(API_KEY)
            .json(&body)
            .send()
            .await
            .expect("PATCH request"),
    )
    .await
}

pub async fn pay(c: &Client, invoice_id: &str, idem_key: &str, token: &str) -> Resp {
    parse(
        c.post(format!("{}/v1/invoices/{}/pay", BASE_URL, invoice_id))
            .bearer_auth(API_KEY)
            .header("Idempotency-Key", idem_key)
            .json(&json!({ "card_token": token }))
            .send()
            .await
            .expect("pay request"),
    )
    .await
}

pub async fn pay_status(c: &Client, invoice_id: &str, idem_key: &str, token: &str) -> StatusCode {
    c.post(format!("{}/v1/invoices/{}/pay", BASE_URL, invoice_id))
        .bearer_auth(API_KEY)
        .header("Idempotency-Key", idem_key)
        .json(&json!({ "card_token": token }))
        .send()
        .await
        .expect("pay request")
        .status()
}
