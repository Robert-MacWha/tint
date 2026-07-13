use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, OptimizationGoal,
};
use ark_snark::SNARK;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use tint_rs::circuit::join_split::JoinSplit;
use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};

fn main() {
    let mut rng = StdRng::seed_from_u64(42);

    // Mock witness: proving time and constraint counts depend only on the
    // circuit's structure (fixed by N_INPUTS/N_OUTPUTS/N_WITHDRAWALS), not on
    // the specific values being proven, so default (zero-valued) values give
    // a representative benchmark without needing a real deposit/spend.
    let circuit = JoinSplit::default();

    // Trusted setup: computed once, not part of the timed benchmark.
    let (pk, _vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();

    let (full_cs, breakdown) = profile_constraints(|cs| circuit.clone().generate_constraints(cs));
    println!("circuit stats:");
    println!("  constraints:       {}", full_cs.num_constraints());
    println!(
        "  public inputs:     {}",
        full_cs.num_instance_variables() - 1
    );
    println!("  witness variables: {}", full_cs.num_witness_variables());
    println!();
    print_breakdown(&breakdown);
    println!();

    let start = Instant::now();
    let _proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();
    println!("groth16 prove: {:?}", start.elapsed());
}

#[derive(Default, Clone, Copy)]
struct Stats {
    calls: usize,
    /// Constraints added while this span was active, including its children.
    inclusive: usize,
    /// Constraints added directly by this span, excluding named children.
    exclusive: usize,
}

struct Frame {
    key: String,
    entered_at: usize,
    children: usize,
}

thread_local! {
    // `ConstraintSystemRef` isn't `Send`/`Sync` (it's `Rc`-based), so it can't
    // live in a `ConstraintProfiler` field -- `tracing::subscriber::with_default`
    // requires the subscriber (and thus every `Layer`) to be `Send + Sync`.
    // A thread-local sidesteps that: it's never actually shared across threads.
    static ACTIVE_CS: RefCell<Option<ConstraintSystemRef<Fr>>> = const { RefCell::new(None) };
}

fn active_num_constraints() -> usize {
    ACTIVE_CS.with(|cs| cs.borrow().as_ref().map(|cs| cs.num_constraints()).unwrap_or(0))
}

/// A `tracing_subscriber::Layer` that attributes constraints to whichever
/// `#[tracing::instrument(target = "r1cs")]`-annotated function was active
/// when they were added, by snapshotting `cs.num_constraints()` (via
/// [`ACTIVE_CS`]) on span enter/exit.
#[derive(Clone, Default)]
struct ConstraintProfiler {
    stack: Arc<Mutex<Vec<Frame>>>,
    stats: Arc<Mutex<HashMap<String, Stats>>>,
}

impl<S> Layer<S> for ConstraintProfiler
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_enter(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if span.metadata().target() != "r1cs" {
            return;
        }
        let key = format!(
            "{}::{}",
            span.metadata().module_path().unwrap_or("?"),
            span.metadata().name()
        );
        self.stack.lock().unwrap().push(Frame {
            key,
            entered_at: active_num_constraints(),
            children: 0,
        });
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        if span.metadata().target() != "r1cs" {
            return;
        }
        let Some(frame) = self.stack.lock().unwrap().pop() else {
            return;
        };
        let now = active_num_constraints();
        let inclusive = now - frame.entered_at;
        let exclusive = inclusive.saturating_sub(frame.children);

        let mut stats = self.stats.lock().unwrap();
        let entry = stats.entry(frame.key).or_default();
        entry.calls += 1;
        entry.inclusive += inclusive;
        entry.exclusive += exclusive;
        drop(stats);

        if let Some(parent) = self.stack.lock().unwrap().last_mut() {
            parent.children += inclusive;
        }
    }
}

/// Synthesizes `f` against a fresh constraint system while recording a
/// per-function constraint breakdown, via every `target = "r1cs"` tracing
/// span active during synthesis.
fn profile_constraints(
    f: impl FnOnce(ConstraintSystemRef<Fr>) -> Result<(), ark_relations::gr1cs::SynthesisError>,
) -> (ConstraintSystemRef<Fr>, Vec<(String, Stats)>) {
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);

    let profiler = ConstraintProfiler::default();
    ACTIVE_CS.with(|active| *active.borrow_mut() = Some(cs.clone()));

    let subscriber = tracing_subscriber::registry().with(profiler.clone());
    tracing::subscriber::with_default(subscriber, || {
        f(cs.clone()).unwrap();
    });
    ACTIVE_CS.with(|active| *active.borrow_mut() = None);
    cs.finalize();

    let mut entries: Vec<(String, Stats)> = profiler
        .stats
        .lock()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    entries.sort_by(|a, b| b.1.exclusive.cmp(&a.1.exclusive));

    (cs, entries)
}

fn print_breakdown(entries: &[(String, Stats)]) {
    println!(
        "constraint breakdown (from tracing spans, sorted by exclusive cost):"
    );
    println!(
        "  {:<55} {:>6} {:>10} {:>10}",
        "span", "calls", "inclusive", "exclusive"
    );
    for (key, stats) in entries {
        println!(
            "  {:<55} {:>6} {:>10} {:>10}",
            key, stats.calls, stats.inclusive, stats.exclusive
        );
    }
}
