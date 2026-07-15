//! Tree format: indented call tree, with same-key siblings merged for
//! readability (e.g. a function called once per note in a loop).

use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Add, Sub},
};

use crate::profiling::Node;

pub fn print_tree<T>(roots: Vec<Node<T>>)
where
    T: Copy + PartialEq + Add<Output = T> + Sub<Output = T> + Default + Debug,
{
    println!("tree, same-key siblings merged, excl / incl:");
    for (root, calls) in fold_siblings(roots) {
        print_tree_node(root, calls, String::new());
    }
}

fn print_tree_node<T>(node: Node<T>, calls: usize, prefix: String)
where
    T: Copy + PartialEq + Add<Output = T> + Sub<Output = T> + Default + Debug,
{
    if node.exclusive() == node.value {
        println!("{prefix}{} ({calls}x, excl {:?})", node.key, node.value);
    } else {
        println!(
            "{prefix}{} ({calls}x, excl {:?}, incl {:?})",
            node.key,
            node.exclusive(),
            node.value
        );
    }

    let child_prefix = format!("{prefix}  ");
    for (child, child_calls) in fold_siblings(node.children) {
        print_tree_node(child, child_calls, child_prefix.clone());
    }
}

fn fold_siblings<T>(nodes: Vec<Node<T>>) -> Vec<(Node<T>, usize)>
where
    T: Copy + Add<Output = T>,
{
    let mut order = Vec::new();
    let mut merged: HashMap<String, (Node<T>, usize)> = HashMap::new();
    for node in nodes {
        match merged.get_mut(&node.key) {
            Some((existing, calls)) => {
                existing.value = existing.value + node.value;
                existing.children.extend(node.children);
                *calls += 1;
            }
            None => {
                order.push(node.key.clone());
                merged.insert(node.key.clone(), (node, 1));
            }
        }
    }
    order
        .into_iter()
        .map(|key| merged.remove(&key).unwrap())
        .collect()
}
