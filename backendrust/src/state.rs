//! Per-request state propagated through Actix extensions.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::cell::RefCell;
use std::rc::Rc;

use crate::models::User;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Debug, Default, Clone)]
pub struct RequestMeta {
    pub request_id: String,
    pub user: Option<User>,
    pub session_id: Option<String>,
}

#[derive(Default)]
pub struct RequestMetaCell {
    inner: Rc<RefCell<RequestMeta>>,
}

impl RequestMetaCell {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, meta: RequestMeta) {
        *self.inner.borrow_mut() = meta;
    }

    pub fn get(&self) -> RequestMeta {
        self.inner.borrow().clone()
    }
}

impl Clone for RequestMetaCell {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}
