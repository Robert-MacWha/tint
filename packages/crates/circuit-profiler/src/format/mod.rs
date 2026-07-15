mod flat;
mod folded;
mod tree;

use std::{
    fmt::Debug,
    ops::{Add, Sub},
};

use crate::profiling::Node;

#[derive(Clone, Copy)]
pub enum OutputFormat {
    /// One row per span, merged across the whole tree, sorted by exclusive cost.
    Flat,
    /// Indented call tree.
    Tree,
    /// Collapsed-stack text (`span_a;span_b;span_c count`) for use with other
    /// perf tools.
    Folded,
}

pub fn render<T>(nodes: Vec<Node<T>>, format: OutputFormat)
where
    T: Copy + PartialEq + Add<Output = T> + Sub<Output = T> + Default + Ord + Debug,
{
    match format {
        OutputFormat::Flat => flat::print_flat(&nodes),
        OutputFormat::Tree => tree::print_tree(nodes),
        OutputFormat::Folded => folded::print_folded(&nodes),
    }
}
