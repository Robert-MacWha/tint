use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use dioxus::prelude::*;
use futures::{
    StreamExt,
    channel::{mpsc, oneshot},
};
use tint_web_protocol::{
    AccountInfo, AccountState, Connection, PublicState, Reply, Request, RequestKind, WorkerMessage,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{ErrorEvent, MessageEvent, Worker, WorkerOptions, WorkerType};

/// Where the page serves the worker's bootstrap script.
const WORKER_URL: &str = "/worker.js";

/// A handle to the tint worker.
#[derive(Clone)]
pub struct WorkerClient(Rc<Inner>);

/// A line in the worker activity log.
#[derive(Clone, PartialEq)]
pub struct LogEntry {
    pub time: String,
    pub id: u64,
    pub request: &'static str,
    pub message: String,
    /// Whether this is a request failing, or the worker stopping.
    pub failed: bool,
}

struct Inner {
    worker: Worker,
    next_id: Cell<u64>,
    pending: RefCell<HashMap<u64, Pending>>,
    /// The latest progress update of the in-flight request.
    status: Signal<Option<String>>,
    log: Signal<Vec<LogEntry>>,
    /// Requests made before the worker was ready, or `None` once it is.
    queued: RefCell<Option<Vec<String>>>,
    /// Why the worker stopped, once it has.
    crashed: RefCell<Option<String>>,
    _on_message: Closure<dyn Fn(MessageEvent)>,
    _on_error: Closure<dyn Fn(ErrorEvent)>,
}

struct Pending {
    tx: oneshot::Sender<Result<Reply, String>>,
    request: &'static str,
    started: f64,
}

impl WorkerClient {
    /// Starts the worker, reporting the in-flight request's progress to
    /// `status` and all activity to `log`.
    ///
    /// Must be called from within a component, which owns the listener.
    pub fn spawn(
        status: Signal<Option<String>>,
        log: Signal<Vec<LogEntry>>,
    ) -> Result<Self, String> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker =
            Worker::new_with_options(WORKER_URL, &options).map_err(|e| format!("{e:?}"))?;

        let (tx, rx) = mpsc::unbounded();
        let error_tx = tx.clone();
        let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
            let message = event
                .data()
                .as_string()
                .and_then(|json| serde_json::from_str::<WorkerMessage>(&json).ok());
            if let Some(message) = message {
                let _ = tx.unbounded_send(message);
            }
        });
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        // Fires when the worker fails to load, or throws outside a panic.
        let on_error = Closure::<dyn Fn(ErrorEvent)>::new(move |event: ErrorEvent| {
            let _ = error_tx.unbounded_send(WorkerMessage::Crashed {
                message: event.message(),
            });
        });
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let client = Self(Rc::new(Inner {
            worker,
            next_id: Cell::new(0),
            pending: RefCell::default(),
            status,
            log,
            queued: RefCell::new(Some(Vec::new())),
            crashed: RefCell::default(),
            _on_message: on_message,
            _on_error: on_error,
        }));
        spawn(listen(client.clone(), rx));
        Ok(client)
    }

    /// Syncs and recovers the private key's accounts, at least `min` of them.
    pub async fn accounts(&self, conn: Connection, min: u32) -> Result<Vec<AccountInfo>, String> {
        match self.request(RequestKind::Accounts { conn, min }).await? {
            Reply::Accounts(accounts) => Ok(accounts),
            _ => Err("worker replied with the wrong type".to_string()),
        }
    }

    /// Makes a request that replies with the account's state.
    pub async fn account_state(&self, kind: RequestKind) -> Result<AccountState, String> {
        match self.request(kind).await? {
            Reply::State(state) => Ok(state),
            _ => Err("worker replied with the wrong type".to_string()),
        }
    }

    /// Makes a request that replies with the signer's public balances.
    pub async fn public(&self, kind: RequestKind) -> Result<PublicState, String> {
        match self.request(kind).await? {
            Reply::Public(state) => Ok(state),
            _ => Err("worker replied with the wrong type".to_string()),
        }
    }

    async fn request(&self, kind: RequestKind) -> Result<Reply, String> {
        if let Some(message) = self.0.crashed.borrow().as_ref() {
            return Err(format!("the tint worker stopped: {message}"));
        }
        let id = self.0.next_id.get();
        self.0.next_id.set(id + 1);
        let request = kind.label();
        let json = serde_json::to_string(&Request { id, kind }).map_err(|e| e.to_string())?;

        let (tx, rx) = oneshot::channel();
        self.0.pending.borrow_mut().insert(
            id,
            Pending {
                tx,
                request,
                started: js_sys::Date::now(),
            },
        );
        self.log(id, request, "requested".to_string(), false);
        self.send(json)?;
        rx.await
            .map_err(|_| "worker dropped the request".to_string())?
    }

    fn send(&self, json: String) -> Result<(), String> {
        if let Some(queued) = self.0.queued.borrow_mut().as_mut() {
            queued.push(json);
            return Ok(());
        }
        self.post(&json)
    }

    /// Sends the requests made before the worker was ready.
    fn flush(&self) {
        let queued = self.0.queued.borrow_mut().take().unwrap_or_default();
        for json in queued {
            if let Err(e) = self.post(&json) {
                web_sys::console::error_1(&JsValue::from_str(&e));
            }
        }
    }

    fn log(&self, id: u64, request: &'static str, message: String, failed: bool) {
        let time = String::from(js_sys::Date::new_0().to_locale_time_string("en-GB"));
        let mut log = self.0.log;
        log.write().push(LogEntry {
            time,
            id,
            request,
            message,
            failed,
        });
    }

    fn post(&self, json: &str) -> Result<(), String> {
        self.0
            .worker
            .post_message(&JsValue::from_str(json))
            .map_err(|e| format!("{e:?}"))
    }
}

impl PartialEq for WorkerClient {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Routes the worker's messages to their pending requests, the status, and
/// the log.
async fn listen(client: WorkerClient, mut messages: mpsc::UnboundedReceiver<WorkerMessage>) {
    let mut status = client.0.status;
    while let Some(message) = messages.next().await {
        match message {
            WorkerMessage::Ready => client.flush(),
            WorkerMessage::Progress { id, message } => {
                let request = client.0.pending.borrow().get(&id).map(|p| p.request);
                client.log(id, request.unwrap_or("?"), message.clone(), false);
                status.set(Some(message));
            }
            WorkerMessage::Done { id, result } => {
                status.set(None);
                let Some(pending) = client.0.pending.borrow_mut().remove(&id) else {
                    continue;
                };
                let elapsed = js_sys::Date::now() - pending.started;
                let outcome = match &result {
                    Ok(_) => format!("done in {elapsed:.0}ms"),
                    Err(e) => format!("failed in {elapsed:.0}ms: {e}"),
                };
                client.log(id, pending.request, outcome, result.is_err());
                let _ = pending.tx.send(result);
            }
            WorkerMessage::Crashed { message } => {
                status.set(None);
                let error = format!("the tint worker stopped: {message}");
                for (id, pending) in client.0.pending.borrow_mut().drain() {
                    client.log(id, pending.request, error.clone(), true);
                    let _ = pending.tx.send(Err(error.clone()));
                }
                *client.0.crashed.borrow_mut() = Some(message);
            }
        }
    }
}
