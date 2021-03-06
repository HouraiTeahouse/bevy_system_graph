#![deny(missing_docs)]
//! This crate provides the utilities for creating strictly ordered execution graphs
//! of systems for the [Bevy][bevy] game engine.
//!
//! # Starting a Graph
//! To start building a system graph, one or more systems must be added to the graph
//! as root nodes. Root systems have no dependencies within the graph.
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! # fn sys_b() {}
//! # fn sys_c() {}
//! let graph = SystemGraph::new();
//!
//! // Create a root system for the graph.
//! let root_a = graph.root(sys_a);
//!
//! // Graphs can have multiple root nodes.
//! let root_b = graph.root(sys_b);
//! let root_c = graph.root(sys_c);
//! ```
//!
//! # Using SystemLabels
//! Systems can still use labels to establish the ordering of systems relative
//! to other systems outside of the graph.
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! let graph = SystemGraph::new();
//! let root_a = graph.root(
//! 	sys_a
//! 	   .label("Physics")
//! 	   .before("Propagate Transforms")
//! );
//! ```
//!
//! # Conversion into SystemSet
//! To ease adding all of the graph's systems into a [`Schedule`], both
//! [`SystemGraph`] and [`SystemGraphNode`] implement [`Into<SystemSet>`].
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! let graph = SystemGraph::new();
//! let root_a = graph.root(sys_a);
//!
//! // Convert into a SystemSet
//! let system_set: SystemSet = graph.into();
//! ```
//!
//! # Sequential Execution
//! Graph nodes can be sequentially chained via [`SystemGraphNode::then`]. This
//! creates a new node from a system and adds a "after" dependency on the original
//! system.
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! # fn sys_b() {}
//! # fn sys_c() {}
//! let graph = SystemGraph::new();
//! graph
//!   .root(sys_a)
//!   .then(sys_b)
//!   .then(sys_c);
//!
//! // Convert into a SystemSet
//! let system_set: SystemSet = graph.into();
//! ```
//!
//! # Fan Out
//! [`SystemGraphNode::fork`] can be used to fan out into multiple branches. All fanned out systems will not execute
//! until the original has finished, but do not have a mutual dependency on each other.
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! # fn sys_b() {}
//! # fn sys_c() {}
//! # fn sys_d() {}
//! # fn sys_e() {}
//! # fn sys_f() {}
//! let graph = SystemGraph::new();
//!
//! // Fork out from one original node.
//! // sys_b, sys_c, and sys_d will only start when sys_a finishes.
//! let (c, b, d) = graph.root(sys_a)
//!     .fork((
//!         sys_b,
//!         sys_c,
//!         sys_d,
//!     ));
//!
//! // Alternatively, calling "then" repeatedly achieves the same thing.
//! let e = d.then(sys_e);
//! let f = d.then(sys_f);
//!
//! // Convert into a SystemSet
//! let system_set: SystemSet = graph.into();
//! ```
//!
//! # Fan In
//! A graph node can wait on multiple systems before running via [`SystemJoin::join`].
//! The system will not run until all prior systems are finished.
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! # fn sys_b() {}
//! # fn sys_c() {}
//! # fn sys_d() {}
//! # fn sys_e() {}
//! let graph = SystemGraph::new();
//!
//! let start_a = graph.root(sys_a);
//! let start_b = graph.root(sys_b);
//! let start_c = graph.root(sys_c);
//!
//! (start_a, start_b, start_c)
//!     .join(sys_d)
//!     .then(sys_e);
//!
//! // Convert into a SystemSet
//! let system_set: SystemSet = graph.into();
//! ```
//!
//! # Fan Out into Fan In
//! The types used to implement [fork] and [join] are composable.
//! ```rust
//! # use bevy_system_graph::*;
//! # use bevy_ecs::prelude::*;
//! # fn sys_a() {}
//! # fn sys_b() {}
//! # fn sys_c() {}
//! # fn sys_d() {}
//! # fn sys_e() {}
//! # fn sys_f() {}
//! let graph = SystemGraph::new();
//! graph.root(sys_a)
//!      .fork((sys_b, sys_c, sys_d))
//!      .join(sys_e)
//!      .then(sys_f);
//!
//! // Convert into a SystemSet
//! let system_set: SystemSet = graph.into();
//! ```
//!
//! # Cloning
//! Individual [graph nodes] are backed by a [`Rc`], so cloning it will still
//! point to the same logical underlying graph.
//!
//! [`Schedule`]: bevy_ecs::schedule::Schedule
//! [`Into<SystemSet>`]: bevy_ecs::schedule::SystemSet
//! [graph nodes]: crate::SystemGraphNode
//! [fork]: crate::SystemGraphNode::fork
//! [join]: crate::SystemJoin::join
//! [bevy]: https://bevyengine.org/

use bevy_ecs::schedule::{
    ExclusiveSystemDescriptorCoercion, IntoSystemDescriptor, ParallelSystemDescriptorCoercion,
    SystemDescriptor, SystemLabel, SystemSet,
};
use bevy_utils::HashMap;
use std::{
    cell::RefCell,
    fmt::Debug,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

static NEXT_GRAPH_ID: AtomicU32 = AtomicU32::new(0);

/// A builder for creating graphs of dependent parallel execution within a [`SystemStage`].
///
/// Please see the crate level docs for examples on how to use this type.
///
/// This type implements [`Clone`] by wrapping a [`Rc`] and can be safely
/// clone and still have the clone refer to the same underlying graph.
///
/// [`SystemStage`]: bevy_ecs::schedule::SystemStage
#[derive(Clone)]
pub struct SystemGraph {
    id: u32,
    nodes: Rc<RefCell<HashMap<NodeId, SystemDescriptor>>>,
}

impl Default for SystemGraph {
    fn default() -> Self {
        Self {
            id: NEXT_GRAPH_ID.fetch_add(1, Ordering::Relaxed),
            nodes: Default::default(),
        }
    }
}

impl SystemGraph {
    /// Creates a new, empty [`SystemGraph`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new [`SystemGraph`] with specific graph ID.
    /// Should be exclusively for testing purposes.
    // #[cfg(test)]
    // pub(crate) fn with_id(id: u32) -> Self {
    //     Self {
    //         id,
    //         nodes: Default::default(),
    //     }
    // }

    /// Creates a root graph node without any dependencies. A graph can have multiple distinct
    /// root nodes.
    pub fn root<Params>(&self, system: impl IntoSystemDescriptor<Params>) -> SystemGraphNode {
        self.create_node(system.into_descriptor())
    }

    /// Checks if two graphs instances point to the same logical underlying graph.
    pub fn is_same_graph(&self, other: &Self) -> bool {
        self.id == other.id
    }

    fn create_node(&self, mut system: SystemDescriptor) -> SystemGraphNode {
        let mut nodes = self.nodes.borrow_mut();
        assert!(
            nodes.len() <= u32::MAX as usize,
            "Cannot add more than {} systems to a SystemGraph",
            u32::MAX
        );
        let id = NodeId(self.id, nodes.len() as u32);
        system = match system {
            SystemDescriptor::Parallel(descriptor) => {
                SystemDescriptor::Parallel(descriptor.label(id))
            }
            SystemDescriptor::Exclusive(descriptor) => {
                SystemDescriptor::Exclusive(descriptor.label(id))
            }
        };
        nodes.insert(id, system);
        SystemGraphNode {
            id,
            graph: self.clone(),
        }
    }

    fn add_dependency(&self, origin: NodeId, dependent: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(system) = nodes.remove(&dependent) {
            nodes.insert(
                dependent,
                match system {
                    SystemDescriptor::Parallel(descriptor) => {
                        SystemDescriptor::Parallel(descriptor.after(origin))
                    }
                    SystemDescriptor::Exclusive(descriptor) => {
                        SystemDescriptor::Exclusive(descriptor.after(origin))
                    }
                },
            );
        } else {
            panic!(
                "Attempted to add dependency for {:?}, which doesn't exist.",
                dependent
            );
        }
    }
}

/// A draining conversion to [`SystemSet`]. All other clones of the same graph will be empty
/// afterwards.
///
/// [`SystemSet`]: bevy_ecs::schedule::SystemSet
impl From<SystemGraph> for SystemSet {
    fn from(graph: SystemGraph) -> Self {
        let mut system_set = SystemSet::new();
        for (_, system) in graph.nodes.borrow_mut().drain() {
            match system {
                SystemDescriptor::Parallel(descriptor) => {
                    system_set = system_set.with_system(descriptor);
                }
                SystemDescriptor::Exclusive(descriptor) => {
                    system_set = system_set.with_system(descriptor);
                }
            }
        }
        system_set
    }
}

/// A single node within a `SystemGraph` that represents a system.
///
/// This type implements [`Clone`] by wrapping a [`Rc`] and can be safely
/// clone and still have the clone refer to the same node in the graph.
#[derive(Clone)]
pub struct SystemGraphNode {
    id: NodeId,
    graph: SystemGraph,
}

impl SystemGraphNode {
    /// Gets the underlying `SystemGraph` that the node belongs to.
    ///
    /// `SystemGraph` is internally ref counted, so the returned value will always point to the
    /// same graph even if the node itself is dropped.
    #[inline]
    pub fn graph(&self) -> SystemGraph {
        self.graph.clone()
    }

    /// Creates a new node in the graph and adds the current node as its dependency.
    ///
    /// This function can be called multiple times to add mulitple systems to the graph,
    /// all of which will not execute until original node's system has finished running.
    pub fn then<Param>(&self, next: impl IntoSystemDescriptor<Param>) -> SystemGraphNode {
        let node = self.graph.create_node(next.into_descriptor());
        self.graph.add_dependency(self.id, node.id);
        node
    }

    /// Fans out from the given node into multiple dependent systems. All provided
    /// systems will not run until the original node's system finishes running.
    ///
    /// Functionally equivalent to calling `SystemGraphNode::then` multiple times.
    #[inline]
    pub fn fork<Param, T: SystemGroup<Param>>(&self, system_group: T) -> T::Output {
        system_group.fork_from(self)
    }
}

/// Represents a collection of systems. Used for grouping systems together for making
/// [`SystemGraph`]s.
pub trait SystemGroup<Param> {
    /// The output of forking or joining a group of [`SystemGraphNode`]s.
    type Output;

    /// Creates a group of [`SystemGraphNode`] forked from one source node.
    fn fork_from(self, src: &SystemGraphNode) -> Self::Output;
    /// Creates a group of [`SystemGraphNode`] joining from multiple source node.
    fn join_from<J: SystemJoin>(self, src: &J) -> Self::Output;
}

/// A collection of `SystemGraphNode`s that can be joined together into one or more dependent
/// systems.
pub trait SystemJoin: Sized {
    /// Adds a system to the graph dependent on all of the nodes contained within the join.
    fn join<Param>(&self, next: impl IntoSystemDescriptor<Param>) -> SystemGraphNode;

    /// Adds a [`SystemGraphNode`] to the graph that is dependent on all of nodes contained
    /// within the join.
    ///
    /// Functionally equivalent to calling `join` on every node created from the group.
    #[inline]
    fn join_all<Param, G: SystemGroup<Param>>(&self, next: G) -> G::Output {
        next.join_from(self)
    }
}

impl<Param, T: IntoSystemDescriptor<Param>> SystemGroup<Param> for Vec<T> {
    type Output = Vec<SystemGraphNode>;
    fn fork_from(self, src: &SystemGraphNode) -> Self::Output {
        self.into_iter().map(|sys| src.then(sys)).collect()
    }

    fn join_from<J: SystemJoin>(self, src: &J) -> Self::Output {
        self.into_iter().map(|sys| src.join(sys)).collect()
    }
}

impl SystemJoin for Vec<SystemGraphNode> {
    fn join<Param>(&self, next: impl IntoSystemDescriptor<Param>) -> SystemGraphNode {
        let mut nodes = self.iter().peekable();
        let output = nodes
            .peek()
            .map(|node| node.graph.create_node(next.into_descriptor()))
            .expect("Attempted to join a collection of zero nodes.");

        for node in nodes {
            assert!(
                output.graph.is_same_graph(&node.graph),
                "Joined graph nodes should be from the same graph."
            );
            output.graph.add_dependency(node.id, output.id);
        }

        output
    }
}

// HACK: using repeat macros without using the param in it fails to compile. The ignore_first
// here "uses" the parameter by discarding it.
macro_rules! ignore_first {
    ($_first:ident, $second:ty) => {
        $second
    };
}

macro_rules! impl_system_tuple {
    ($($param: ident),*) => {
        impl<Param, $($param: IntoSystemDescriptor<Param>),*> SystemGroup<Param> for ($($param,)*) {
            type Output = ($(ignore_first!($param, SystemGraphNode),)*);

            #[inline]
            #[allow(non_snake_case)]
            fn fork_from(self, src: &SystemGraphNode) -> Self::Output {
                let ($($param,)*) = self;
                ($(src.then($param),)*)
            }

            #[inline]
            #[allow(non_snake_case)]
            fn join_from<J: SystemJoin>(self, src: &J) -> Self::Output {
                let ($($param,)*) = self;
                ($(src.join($param),)*)
            }
        }

        impl SystemJoin for ($(ignore_first!($param, SystemGraphNode),)*) {
            #[inline]
            #[allow(non_snake_case)]
            fn join<Param>(&self, next: impl IntoSystemDescriptor<Param>) -> SystemGraphNode {
                let output = self.0.graph.create_node(next.into_descriptor());
                let ($($param,)*) = self;
                $(
                    assert!(output.graph.is_same_graph(&$param.graph),
                            "Joined graph nodes must be from the same graph.");
                    output.graph.add_dependency($param.id, output.id);
                )*
                output
            }
        }
    };
}

impl_system_tuple!(T1, T2);
impl_system_tuple!(T1, T2, T3);
impl_system_tuple!(T1, T2, T3, T4);
impl_system_tuple!(T1, T2, T3, T4, T5);
impl_system_tuple!(T1, T2, T3, T4, T5, T6);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_system_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NodeId(u32, u32);

impl SystemLabel for NodeId {
    fn dyn_clone(&self) -> Box<dyn SystemLabel> {
        Box::new(<NodeId>::clone(self))
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // use bevy_ecs::schedule::SystemDescriptor;

    // fn dummy_system() {}

    // fn assert_eq_after(sys: &SystemDescriptor, expected: Vec<NodeId>) {
    //     let deps = match sys {
    //         SystemDescriptor::Parallel(desc) => &desc.after,
    //         SystemDescriptor::Exclusive(desc) => &desc.after,
    //     };
    //     let after: Vec<Box<dyn SystemLabel>> =
    //         expected.into_iter().map(|id| id.dyn_clone()).collect();
    //     assert_eq!(deps, &after);
    // }

    // #[test]
    // pub fn then_creates_accurate_dependencies() {
    //     let graph = SystemGraph::with_id(0);
    //     graph
    //         .root(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system);

    //     let systems = graph.nodes.borrow();

    //     assert_eq!(systems.len(), 4);
    //     assert_eq_after(&systems[&NodeId(0, 0)], vec![]);
    //     assert_eq_after(&systems[&NodeId(0, 1)], vec![NodeId(0, 0)]);
    //     assert_eq_after(&systems[&NodeId(0, 2)], vec![NodeId(0, 1)]);
    //     assert_eq_after(&systems[&NodeId(0, 3)], vec![NodeId(0, 2)]);
    // }

    // #[test]
    // pub fn fork_creates_accurate_dependencies() {
    //     let graph = SystemGraph::with_id(0);
    //     graph
    //         .root(dummy_system)
    //         .fork((dummy_system, dummy_system, dummy_system));

    //     let systems = graph.nodes.borrow();

    //     assert_eq!(systems.len(), 4);
    //     assert_eq_after(&systems[&NodeId(0, 0)], vec![]);
    //     assert_eq_after(&systems[&NodeId(0, 1)], vec![NodeId(0, 0)]);
    //     assert_eq_after(&systems[&NodeId(0, 2)], vec![NodeId(0, 0)]);
    //     assert_eq_after(&systems[&NodeId(0, 3)], vec![NodeId(0, 0)]);
    // }

    // #[test]
    // pub fn join_creates_accurate_dependencies() {
    //     let graph = SystemGraph::with_id(0);
    //     let a = graph.root(dummy_system);
    //     let b = graph.root(dummy_system);
    //     let c = graph.root(dummy_system);

    //     (a, b, c).join(dummy_system);

    //     let systems = graph.nodes.borrow();

    //     assert_eq!(systems.len(), 4);
    //     assert_eq_after(&systems[&NodeId(0, 0)], vec![]);
    //     assert_eq_after(&systems[&NodeId(0, 1)], vec![]);
    //     assert_eq_after(&systems[&NodeId(0, 2)], vec![]);
    //     assert_eq_after(
    //         &systems[&NodeId(0, 3)],
    //         vec![NodeId(0, 0), NodeId(0, 1), NodeId(0, 2)],
    //     );
    // }

    // #[test]
    // pub fn graph_creates_accurate_system_counts() {
    //     let graph = SystemGraph::new();
    //     let a = graph
    //         .root(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system);
    //     let b = graph.root(dummy_system).then(dummy_system);
    //     let c = graph
    //         .root(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system);
    //     vec![a, b, c].join(dummy_system).then(dummy_system);
    //     let system_set: SystemSet = graph.into();
    //     let (_, systems) = system_set.bake();

    //     assert_eq!(systems.len(), 11);
    // }

    // #[test]
    // pub fn all_nodes_are_labeled() {
    //     let graph = SystemGraph::new();
    //     let a = graph
    //         .root(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system);
    //     let b = graph.root(dummy_system).then(dummy_system);
    //     let c = graph
    //         .root(dummy_system)
    //         .then(dummy_system)
    //         .then(dummy_system);
    //     vec![a, b, c].join(dummy_system).then(dummy_system);
    //     let system_set: SystemSet = graph.into();
    //     let (_, systems) = system_set.bake();

    //     let mut root_count = 0;
    //     for system in systems {
    //         match system {
    //             SystemDescriptor::Parallel(desc) => {
    //                 assert!(!desc.labels.is_empty());
    //                 if desc.after.is_empty() {
    //                     root_count += 1;
    //                 }
    //             }
    //             SystemDescriptor::Exclusive(desc) => {
    //                 assert!(!desc.labels.is_empty());
    //                 if desc.after.is_empty() {
    //                     root_count += 1;
    //                 }
    //             }
    //         }
    //     }
    //     assert_eq!(root_count, 3);
    // }
}
