use crate::common::*;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn create_get_list() {
    let c = client();
    let email = format!("buyer-{}@example.com", Uuid::new_v4());
    let created = post_json(
        &c,
        "/v1/customers/",
        json!({ "name": "Grace Hopper", "email": email }),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.data()["business_id"], BUSINESS_ID);
    let id = created.data()["id"].as_str().unwrap().to_string();

    let got = get(&c, &format!("/v1/customers/{}", id)).await;
    assert_eq!(got.status, StatusCode::OK);
    assert_eq!(got.data()["id"], id);
    assert_eq!(got.data()["email"], email);

    let listed = get(&c, "/v1/customers/?page=1&limit=1").await;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.data()["page"], 1);
    assert_eq!(listed.data()["limit"], 1);
    assert!(listed.data()["items"].as_array().unwrap().len() <= 1);
}

#[tokio::test]
async fn unknown_is_404() {
    let c = client();
    let r = get(&c, "/v1/customers/00000000-0000-0000-0000-0000000000ff").await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.code(), "DB_RECORD_NOT_FOUND");
}

#[tokio::test]
async fn missing_fields_is_400() {
    let c = client();
    let r = post_json(&c, "/v1/customers/", json!({})).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}
