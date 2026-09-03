use crate::common::*;
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn total_is_server_computed() {
    let c = client();
    let r = post_json(
        &c,
        "/v1/invoices/",
        json!({
            "customer_id": CUSTOMER_ID,
            "due_date": "2026-12-01",
            "line_items": [
                { "description": "seat", "quantity": 3, "unit_amount_cents": 1500 },
                { "description": "setup", "quantity": 1, "unit_amount_cents": 1000 }
            ]
        }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.data()["total_cents"], 5500);
    assert_eq!(r.data()["state"], "draft");
}

#[tokio::test]
async fn get_includes_line_items() {
    let c = client();
    let id = create_invoice(&c, 4200).await;
    let r = get(&c, &format!("/v1/invoices/{}", id)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.data()["id"], id);
    assert_eq!(r.data()["total_cents"], 4200);
    assert_eq!(r.data()["line_items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn for_foreign_customer_is_404() {
    let c = client();
    let r = post_json(
        &c,
        "/v1/invoices/",
        json!({
            "customer_id": "00000000-0000-0000-0000-0000000000ff",
            "due_date": "2026-12-01",
            "line_items": [{ "description": "x", "quantity": 1, "unit_amount_cents": 100 }]
        }),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn state_machine_valid_and_invalid_transitions() {
    let c = client();
    let id = create_invoice(&c, 5000).await;

    let open = patch_json(&c, &format!("/v1/invoices/{}", id), json!({ "state": "open" })).await;
    assert_eq!(open.status, StatusCode::OK);
    assert_eq!(open.data()["state"], "open");

    let void = patch_json(&c, &format!("/v1/invoices/{}", id), json!({ "state": "void" })).await;
    assert_eq!(void.status, StatusCode::OK);
    assert_eq!(void.data()["state"], "void");

    let leave_terminal =
        patch_json(&c, &format!("/v1/invoices/{}", id), json!({ "state": "open" })).await;
    assert_eq!(leave_terminal.status, StatusCode::CONFLICT);
    assert_eq!(leave_terminal.code(), "INVOICE_INVALID_STATE_TRANSITION");
}

#[tokio::test]
async fn state_machine_rejects_paid_as_patch_target() {
    let c = client();
    let id = create_invoice(&c, 5000).await;
    finalize(&c, &id).await;
    let r = patch_json(&c, &format!("/v1/invoices/{}", id), json!({ "state": "paid" })).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_filtered_by_state() {
    let c = client();
    let id = create_invoice(&c, 5000).await;
    finalize(&c, &id).await;
    let r = get(&c, "/v1/invoices/?state=open").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.data()["items"].is_array());
}
