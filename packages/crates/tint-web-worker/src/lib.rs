//! A web worker running tint for the tint-web UI, so syncing and proving
//! never block the page.
//!
//! Requests are handled one at a time, in the order they arrive.

mod account_state;
mod artifacts;
mod handler;
mod keys;
mod public;
mod revert;
mod sponsor;
mod store;

use futures::{StreamExt, channel::mpsc};
use tint_web_protocol::{Request, WorkerMessage};
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::JsFuture;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

use crate::handler::Handler;

#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        post(&WorkerMessage::Crashed {
            message: info.to_string(),
        });
    }));

    let (tx, rx) = mpsc::unbounded::<Request>();
    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
        match parse_request(&event) {
            Ok(request) => {
                let _ = tx.unbounded_send(request);
            }
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&e)),
        }
    });
    scope().set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    wasm_bindgen_futures::spawn_local(async move {
        let pool = wasm_bindgen_rayon::init_thread_pool(thread_count());
        if let Err(e) = JsFuture::from(pool).await {
            post(&WorkerMessage::Crashed {
                message: format!("starting the proving thread pool: {e:?}"),
            });
            return;
        }
        post(&WorkerMessage::Ready);
        run(rx).await;
    });
}

async fn run(mut requests: mpsc::UnboundedReceiver<Request>) {
    let mut handler = Handler::default();
    while let Some(Request { id, kind }) = requests.next().await {
        let result = handler.handle(id, kind).await.map_err(|e| format!("{e:#}"));
        post(&WorkerMessage::Done { id, result });
    }
}

/// Posts a status update for request `id`.
fn progress(id: u64, message: &str) {
    post(&WorkerMessage::Progress {
        id,
        message: message.to_string(),
    });
}

fn post(message: &WorkerMessage) {
    let json = match serde_json::to_string(message) {
        Ok(json) => json,
        Err(e) => {
            web_sys::console::error_1(&JsValue::from_str(&e.to_string()));
            return;
        }
    };
    if let Err(e) = scope().post_message(&JsValue::from_str(&json)) {
        web_sys::console::error_1(&e);
    }
}

fn parse_request(event: &MessageEvent) -> Result<Request, String> {
    let json = event
        .data()
        .as_string()
        .ok_or("worker request is not a string")?;
    serde_json::from_str(&json).map_err(|e| format!("invalid worker request: {e}"))
}

/// The number of rayon threads to prove with, one per logical core.
#[expect(
    clippy::cast_sign_loss,
    reason = "hardwareConcurrency is a positive integer"
)]
fn thread_count() -> usize {
    scope().navigator().hardware_concurrency() as usize
}

fn scope() -> DedicatedWorkerGlobalScope {
    js_sys::global().unchecked_into()
}
