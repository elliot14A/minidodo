use crate::common::*;
use reqwest::StatusCode;

#[tokio::test]
async fn health_is_public() {
    let c = client();
    let r = c.get(format!("{}/v1/health", BASE_URL)).send().await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.text().await.unwrap(), "OK");
}

#[tokio::test]
async fn missing_header_is_rejected() {
    let c = client();
    let r = c.get(format!("{}/v1/businesses/me", BASE_URL)).send().await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["code"], "AUTH_UNAUTHORIZED");
}

#[tokio::test]
async fn invalid_key_is_rejected() {
    let c = client();
    let r = c
        .get(format!("{}/v1/businesses/me", BASE_URL))
        .bearer_auth("dodo_test_not_real")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["code"], "AUTH_INVALID_KEY");
}

#[tokio::test]
async fn valid_key_resolves_business() {
    let c = client();
    let r = get(&c, "/v1/businesses/me").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.data()["id"], BUSINESS_ID);
    assert_eq!(r.data()["name"], "Acme Corp");
}
