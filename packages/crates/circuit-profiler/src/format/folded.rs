use std::{
    fmt::Debug,
    ops::{Add, Sub},
};

use crate::profiling::Node;

pub fn print_folded<T>(roots: &[Node<T>])
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Default + Ord + Debug,
{
    for root in roots {
        print_folded_node(root, String::new());
    }
}

fn print_folded_node<T>(node: &Node<T>, stack: String)
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Default + Ord + Debug,
{
    let stack = if stack.is_empty() {
        node.key.clone()
    } else {
        format!("{stack};{}", node.key)
    };
    let exclusive = node.exclusive();
    if exclusive > T::default() {
        println!("{stack} {exclusive:?}");
    }
    for child in &node.children {
        print_folded_node(child, stack.clone());
    }
}
