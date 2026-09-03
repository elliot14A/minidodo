use super::constants::CUSTOMER_ID;
use super::http::{get, patch_json, post_json};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::time::Duration;

pub async fn create_invoice(c: &Client, unit_cents: i64) -> String {
    let r = post_json(
        c,
        "/v1/invoices/",
        json!({
            "customer_id": CUSTOMER_ID,
            "due_date": "2026-12-01",
            "line_items": [{ "description": "test", "quantity": 1, "unit_amount_cents": unit_cents }]
        }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "create invoice: {:?}", r.body);
    r.data()["id"].as_str().expect("invoice id").to_string()
}

pub async fn finalize(c: &Client, invoice_id: &str) {
    let r = patch_json(c, &format!("/v1/invoices/{}", invoice_id), json!({ "state": "open" })).await;
    assert_eq!(r.status, StatusCode::OK, "finalize: {:?}", r.body);
}

pub async fn invoice_state(c: &Client, invoice_id: &str) -> String {
    let r = get(c, &format!("/v1/invoices/{}", invoice_id)).await;
    r.data()["state"].as_str().unwrap_or("").to_string()
}

pub async fn wait_for_state(c: &Client, invoice_id: &str, want: &str, timeout_secs: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if invoice_state(c, invoice_id).await == want {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
