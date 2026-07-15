use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Add, Sub},
};

use crate::profiling::Node;

#[derive(Default, Clone, Copy)]
struct Stats<T> {
    calls: usize,
    inclusive: T,
    exclusive: T,
}

pub fn print_flat<T>(roots: &[Node<T>])
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Default + Ord + Debug,
{
    let mut merged: HashMap<String, Stats<T>> = HashMap::new();
    for root in roots {
        flatten_into(root, &mut merged);
    }
    let mut entries: Vec<(String, Stats<T>)> = merged.into_iter().collect();
    entries.sort_by(|a, b| b.1.exclusive.cmp(&a.1.exclusive));

    let max_key_len = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    println!(
        "  {:<max_key_len$} {:>6} {:>14} {:>14}",
        "span", "calls", "inclusive", "exclusive"
    );
    for (key, stats) in entries {
        println!(
            "  {:<max_key_len$} {:>6} {:>14?} {:>14?}",
            key, stats.calls, stats.inclusive, stats.exclusive
        );
    }
}

fn flatten_into<T>(node: &Node<T>, out: &mut HashMap<String, Stats<T>>)
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Default,
{
    let entry = out.entry(node.key.clone()).or_default();
    entry.calls += 1;
    entry.inclusive = entry.inclusive + node.value;
    entry.exclusive = entry.exclusive + node.exclusive();
    for child in &node.children {
        flatten_into(child, out);
    }
}
