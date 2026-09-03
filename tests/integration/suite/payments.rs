use crate::common::*;
use reqwest::StatusCode;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn requires_idempotency_key() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    finalize(&c, &id).await;
    let r = c
        .post(format!("{}/v1/invoices/{}/pay", BASE_URL, id))
        .bearer_auth(API_KEY)
        .json(&json!({ "card_token": "tok_success" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_REQUIRED_HEADER_MISSING");
}

#[tokio::test]
async fn is_non_blocking_and_settles() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    finalize(&c, &id).await;

    let key = format!("happy-{}", id);
    let r = pay(&c, &id, &key, "tok_success").await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    assert_eq!(r.data()["status"], "processing");
    assert_eq!(r.data()["invoice_id"], id);

    assert!(wait_for_state(&c, &id, "paid", 30).await, "should settle to paid");
}

#[tokio::test]
async fn idempotent_replay_returns_stored_result() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    finalize(&c, &id).await;
    let key = format!("replay-{}", id);

    let first = pay(&c, &id, &key, "tok_success").await;
    assert_eq!(first.status, StatusCode::ACCEPTED);
    let attempt_id = first.data()["attempt_id"].as_str().unwrap().to_string();

    assert!(wait_for_state(&c, &id, "paid", 30).await);

    let replay = pay(&c, &id, &key, "tok_success").await;
    assert_eq!(replay.status, StatusCode::OK);
    assert_eq!(replay.data()["status"], "paid");
    assert_eq!(replay.data()["attempt_id"], attempt_id);
    assert!(replay.data()["psp_ref"].is_string());

    let pool = db().await;
    assert_eq!(count_attempts(&pool, &id).await, 1, "replay must not create a new attempt");
}

#[tokio::test]
async fn same_key_different_body_is_422() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    finalize(&c, &id).await;
    let key = format!("conflict-{}", id);

    let first = pay(&c, &id, &key, "tok_success").await;
    assert_eq!(first.status, StatusCode::ACCEPTED);

    let conflict = pay(&c, &id, &key, "tok_card_declined").await;
    assert_eq!(conflict.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(conflict.code(), "PAYMENT_IDEMPOTENCY_KEY_CONFLICT");
}

#[tokio::test]
async fn already_paid_is_409() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    finalize(&c, &id).await;
    assert_eq!(pay(&c, &id, &format!("a-{}", id), "tok_success").await.status, StatusCode::ACCEPTED);
    assert!(wait_for_state(&c, &id, "paid", 30).await);

    let again = pay(&c, &id, &format!("b-{}", id), "tok_success").await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    assert_eq!(again.code(), "PAYMENT_INVOICE_NOT_PAYABLE");
}

#[tokio::test]
async fn unknown_invoice_is_404() {
    let c = client();
    let r = pay(&c, "00000000-0000-0000-0000-0000000000ff", "ghost", "tok_success").await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn declined_card_returns_invoice_to_open() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    finalize(&c, &id).await;
    assert_eq!(
        pay(&c, &id, &format!("dec-{}", id), "tok_card_declined").await.status,
        StatusCode::ACCEPTED
    );
    assert!(wait_for_state(&c, &id, "open", 30).await, "declined should return to open");

    let pool = db().await;
    assert_eq!(latest_attempt_status(&pool, &id).await.as_deref(), Some("failed"));
}

#[tokio::test]
async fn network_error_does_not_corrupt_state() {
    let c = client();
    let id = create_invoice(&c, 3300).await;
    finalize(&c, &id).await;
    assert_eq!(
        pay(&c, &id, &format!("net-{}", id), "tok_network_error").await.status,
        StatusCode::ACCEPTED
    );
    assert!(wait_for_state(&c, &id, "open", 30).await, "network error should return to open");
}

#[tokio::test]
async fn timeout_does_not_block_api_and_eventually_settles() {
    let c = client();
    let id = create_invoice(&c, 5500).await;
    finalize(&c, &id).await;

    let start = std::time::Instant::now();
    let r = pay(&c, &id, &format!("to-{}", id), "tok_timeout").await;
    let elapsed = start.elapsed();
    assert_eq!(r.status, StatusCode::ACCEPTED);
    assert!(
        elapsed.as_secs() < 5,
        "pay endpoint blocked on the slow PSP: took {:?}",
        elapsed
    );

    assert!(
        wait_for_state(&c, &id, "paid", 45).await,
        "slow success should settle within the client timeout window"
    );
}

#[tokio::test]
async fn concurrent_pays_charge_at_most_once() {
    let c = client();
    let id = create_invoice(&c, 9900).await;
    finalize(&c, &id).await;

    let n = 10;
    let mut handles = Vec::new();
    for i in 0..n {
        let c = c.clone();
        let id = id.clone();
        handles.push(tokio::spawn(async move {
            pay_status(&c, &id, &format!("conc-{}-{}", id, i), "tok_success").await
        }));
    }

    let mut counts: HashMap<u16, usize> = HashMap::new();
    for h in handles {
        let status = h.await.unwrap();
        *counts.entry(status.as_u16()).or_default() += 1;
    }

    assert_eq!(counts.get(&202).copied().unwrap_or(0), 1, "exactly one request wins the claim");
    assert_eq!(counts.get(&409).copied().unwrap_or(0), n - 1, "the rest are rejected with 409");

    assert!(wait_for_state(&c, &id, "paid", 30).await);

    let pool = db().await;
    assert_eq!(count_succeeded_attempts(&pool, &id).await, 1, "no double charge");
    assert_eq!(count_attempts(&pool, &id).await, 1, "only the winner created an attempt");
}
