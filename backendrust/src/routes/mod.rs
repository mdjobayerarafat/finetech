//! All HTTP route handlers.

use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::ctx::RequestId;
use crate::state::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(healthz))
        .route("/", web::get().to(root))
        .service(
            web::scope("/api/v1")
                .configure(auth::configure)
                .configure(accounts::configure)
                .configure(budgets::configure)
                .configure(payees::configure)
                .configure(subscriptions::configure)
                .configure(cards::configure)
                .configure(wealth::configure)
                .configure(agent::configure)
                .configure(statements::configure)
                .configure(actions::configure)
                .configure(admin::configure)
                .configure(features::configure)
                .configure(smart_transfers::configure)
                .configure(util::configure)
                .route("/docs", web::get().to(|| async {
                    HttpResponse::Ok().json(json!({
                        "message": "BankPilot API docs are available in the FastAPI version at /api/v1/docs.",
                    }))
                })),
        );
}

async fn healthz(_state: web::Data<AppState>, _rid: RequestId) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "ok": true, "service": "bankpilot", "version": env!("CARGO_PKG_VERSION") }))
}

async fn root() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "service": "bankpilot",
        "version": env!("CARGO_PKG_VERSION"),
        "docs": "/api/v1/docs",
        "simulation": true,
        "message": "BankPilot API is running. This is a simulation — no real money is moved."
    }))
}

pub mod auth;
pub mod accounts;
pub mod budgets;
pub mod payees;
pub mod subscriptions;
pub mod cards;
pub mod wealth;
pub mod agent;
pub mod statements;
pub mod actions;
pub mod admin;
pub mod features;
pub mod smart_transfers;
pub mod util;
