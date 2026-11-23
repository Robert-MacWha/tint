use std::sync::Arc;

use alloy_primitives::{Address, B256};
use ark_bn254::Fr;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};
use tint_groth16::serde::field::FieldAsBytes;
use tint_intent_policy::intent::Intent;
use tokio::sync::Mutex;

use crate::now;

/// Filters intents by the assets they offer and want.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IntentFilter {
    pub offer_asset: Option<Address>,
    pub want_asset: Option<Address>,
}

/// Removes an intent from the relay.
#[serde_with::serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelRequest {
    /// The intent's `CancelKey::token`.
    #[serde_as(as = "FieldAsBytes")]
    pub cancel_token: Fr,
}

/// Open intents, in the order they were posted.
type Store = Arc<Mutex<Vec<Intent>>>;

/// The relay's routes, backed by a fresh in-memory store.
pub fn router() -> Router {
    Router::new()
        .route("/intents", post(post_intent).get(list_intents))
        .route("/intents/{id}", delete(cancel_intent))
        .with_state(Store::default())
}

/// Stores a valid, unexpired intent, returning its id.
async fn post_intent(
    State(store): State<Store>,
    Json(intent): Json<Intent>,
) -> Result<Json<B256>, StatusCode> {
    if intent.validate().is_err() || intent.expiry < now() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = intent.id();
    let mut store = store.lock().await;
    store.retain(|i| i.id() != id);
    store.push(intent);
    Ok(Json(id))
}

/// Lists unexpired intents matching `filter`, dropping expired ones.
async fn list_intents(
    State(store): State<Store>,
    Query(filter): Query<IntentFilter>,
) -> Json<Vec<Intent>> {
    let now = now();
    let mut store = store.lock().await;
    store.retain(|i| i.expiry >= now);

    let matching = store
        .iter()
        .filter(|i| filter.offer_asset.is_none_or(|a| i.offer.asset.0 == a))
        .filter(|i| filter.want_asset.is_none_or(|a| i.want.asset.0 == a))
        .cloned()
        .collect();
    Json(matching)
}

/// Removes intent `id` if the request carries its cancel token.
async fn cancel_intent(
    State(store): State<Store>,
    Path(id): Path<B256>,
    Json(request): Json<CancelRequest>,
) -> StatusCode {
    let mut store = store.lock().await;
    let Some(index) = store.iter().position(|i| i.id() == id) else {
        return StatusCode::NOT_FOUND;
    };
    if !store[index].accepts_cancel_token(request.cancel_token) {
        return StatusCode::FORBIDDEN;
    }

    store.remove(index);
    StatusCode::NO_CONTENT
}
