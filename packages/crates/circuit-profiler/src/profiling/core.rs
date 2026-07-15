//! The generic tracing-span profiler: samples a metric on span enter/exit
//! and assembles the deltas into a call tree. No arkworks dependency.

use std::{
    ops::{Add, Sub},
    sync::{Arc, Mutex},
};

use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};

/// One invocation of a tracing span, with the metric change recorded while
/// it and its descendants were active.
pub struct Node<T> {
    pub key: String,
    pub value: T,
    pub children: Vec<Node<T>>,
}

impl<T> Node<T>
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Default,
{
    /// The metric attributed directly to this span, excluding its children
    /// (`value` minus the sum of its direct children's `value`s).
    pub fn exclusive(&self) -> T {
        let children_sum = self
            .children
            .iter()
            .fold(T::default(), |acc, child| acc + child.value);
        self.value - children_sum
    }
}

/// Runs `f` while sampling `T` (via `sample`) on the enter/exit of every
/// span with `target = target`, and returns one call tree per top-level
/// span.
pub fn profile<T>(
    target: &'static str,
    sample: impl Fn() -> T + Send + Sync + 'static,
    f: impl FnOnce(),
) -> Vec<Node<T>>
where
    T: Copy + Sub<Output = T> + Send + Sync + 'static,
{
    let profiler = SpanProfiler {
        target,
        sample: Arc::new(sample),
        stack: Arc::new(Mutex::new(Vec::new())),
        roots: Arc::new(Mutex::new(Vec::new())),
    };

    let subscriber = tracing_subscriber::registry().with(profiler.clone());
    tracing::subscriber::with_default(subscriber, f);

    std::mem::take(&mut *profiler.roots.lock().unwrap())
}

struct SpanProfiler<T> {
    target: &'static str,
    sample: Arc<dyn Fn() -> T + Send + Sync>,
    // While a span is open, its `Node.value` holds the sample taken on entry
    // rather than the final delta -- `on_exit` overwrites it in place once
    // the span closes.
    stack: Arc<Mutex<Vec<Node<T>>>>,
    roots: Arc<Mutex<Vec<Node<T>>>>,
}

impl<T> Clone for SpanProfiler<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            sample: self.sample.clone(),
            stack: self.stack.clone(),
            roots: self.roots.clone(),
        }
    }
}

impl<T, S> Layer<S> for SpanProfiler<T>
where
    T: Copy + Sub<Output = T> + Send + Sync + 'static,
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_enter(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if span.metadata().target() != self.target {
            return;
        }
        let key = format!(
            "{}::{}",
            span.metadata().module_path().unwrap_or("?"),
            span.metadata().name()
        );
        self.stack.lock().unwrap().push(Node {
            key,
            value: (self.sample)(),
            children: Vec::new(),
        });
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if span.metadata().target() != self.target {
            return;
        }
        let mut stack = self.stack.lock().unwrap();
        let Some(mut node) = stack.pop() else {
            return;
        };
        node.value = (self.sample)() - node.value;

        match stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => {
                drop(stack);
                self.roots.lock().unwrap().push(node);
            }
        }
    }
}
