use crate::common::*;
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use serde_json::json;
use sha2::Sha256;
use uuid::Uuid;

#[tokio::test]
async fn sink_rejects_bad_signature() {
    let c = client();

    let missing = c
        .post(format!("{}/webhooks/sink", PSP_URL))
        .json(&json!({ "event_id": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let bad = c
        .post(format!("{}/webhooks/sink", PSP_URL))
        .header("X-Webhook-Signature", "sha256=deadbeef")
        .header("X-Webhook-Timestamp", "1700000000")
        .json(&json!({ "event_id": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sink_accepts_valid_signature() {
    let c = client();
    let body = json!({ "event_id": Uuid::new_v4(), "event_type": "invoice.paid" });
    let raw = serde_json::to_vec(&body).unwrap();
    let timestamp = 1_700_000_000i64;

    let mut mac = Hmac::<Sha256>::new_from_slice(WEBHOOK_SECRET.as_bytes()).unwrap();
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(&raw);
    let sig = hex::encode(mac.finalize().into_bytes());

    let r = c
        .post(format!("{}/webhooks/sink", PSP_URL))
        .header("X-Webhook-Signature", format!("sha256={}", sig))
        .header("X-Webhook-Timestamp", timestamp.to_string())
        .header("content-type", "application/json")
        .body(raw)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn delivered_on_paid() {
    let c = client();
    let pool = db().await;
    let id = create_invoice(&c, 6100).await;
    finalize(&c, &id).await;
    assert_eq!(
        pay(&c, &id, &format!("wh-paid-{}", id), "tok_success").await.status,
        StatusCode::ACCEPTED
    );
    assert!(wait_for_state(&c, &id, "paid", 30).await);
    assert!(
        wait_for_delivery(&pool, &id, "invoice.paid", "delivered", 30).await,
        "invoice.paid webhook should be delivered"
    );
}

#[tokio::test]
async fn delivered_on_payment_failed() {
    let c = client();
    let pool = db().await;
    let id = create_invoice(&c, 2100).await;
    finalize(&c, &id).await;
    assert_eq!(
        pay(&c, &id, &format!("wh-fail-{}", id), "tok_card_declined").await.status,
        StatusCode::ACCEPTED
    );
    assert!(wait_for_state(&c, &id, "open", 30).await);
    assert!(
        wait_for_delivery(&pool, &id, "invoice.payment_failed", "delivered", 30).await,
        "invoice.payment_failed webhook should be delivered"
    );
}
