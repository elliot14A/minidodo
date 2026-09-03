use crate::common::*;
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn token_matrix() {
    let c = client();

    let ok: serde_json::Value = c
        .post(format!("{}/v1/charges", PSP_URL))
        .json(&json!({ "amount_cents": 100, "card_token": "tok_success", "idempotency_key": "m1" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ok["status"], "succeeded");
    assert!(ok["psp_ref"].is_string());

    let declined = c
        .post(format!("{}/v1/charges", PSP_URL))
        .json(&json!({ "amount_cents": 100, "card_token": "tok_card_declined", "idempotency_key": "m2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(declined.status(), StatusCode::BAD_REQUEST);
    let db: serde_json::Value = declined.json().await.unwrap();
    assert_eq!(db["code"], "card_declined");

    let insuff = c
        .post(format!("{}/v1/charges", PSP_URL))
        .json(&json!({ "amount_cents": 100, "card_token": "tok_insufficient_funds", "idempotency_key": "m3" }))
        .send()
        .await
        .unwrap();
    assert_eq!(insuff.status(), StatusCode::BAD_REQUEST);
    let ib: serde_json::Value = insuff.json().await.unwrap();
    assert_eq!(ib["code"], "insufficient_funds");

    let neterr = c
        .post(format!("{}/v1/charges", PSP_URL))
        .json(&json!({ "amount_cents": 100, "card_token": "tok_network_error", "idempotency_key": "m4" }))
        .send()
        .await
        .unwrap();
    assert_eq!(neterr.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
