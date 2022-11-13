# bevy_system_graph

[![crates.io](https://img.shields.io/crates/v/bevy_system_graph.svg)](https://crates.io/crates/bevy_system_graph)
[![Documentation](https://docs.rs/bevy_system_graph/badge.svg)](https://docs.rs/bevy_system_graph)
![License](https://img.shields.io/crates/l/bevy_system_graph)
[![Discord](https://img.shields.io/discord/151219753434742784.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/VuZhs9V)

This crate provides the utilities for creating strictly ordered execution graphs
of systems for the [Bevy][bevy] game engine.

## Bevy Version Supported

|Bevy Version|bevy\_system\_graph|
|:-----------|:------------------|
|0.9         |0.4                |
|0.8         |0.3                |
|0.7         |0.2                |
|0.6         |0.1                |

### Starting a Graph
To start building a system graph, one or more systems must be added to the graph
as root nodes. Root systems have no dependencies within the graph.
```rust
let graph = SystemGraph::new();

// Create a root system for the graph.
let root_a = graph.root(sys_a);

// Graphs can have multiple root nodes.
let root_b = graph.root(sys_b);
let root_c = graph.root(sys_c);
```

### Using SystemLabels
Systems can still use labels to establish the ordering of systems relative
to other systems outside of the graph.
```rust
let graph = SystemGraph::new();
let root_a = graph.root(
	sys_a
	   .label("Physics")
	   .before("Propagate Transforms")
);
```

### Conversion into SystemSet
To ease adding all of the graph's systems into a `Schedule`, both 
`SystemGraph` and `SystemGraphNode` implement `Into<SystemSet>`.
```rust
let graph = SystemGraph::new();
let root_a = graph.root(sys_a);

// Convert into a SystemSet
let system_set: SystemSet = graph.into();
```

### Sequential Execution
Graph nodes can be sequentially chained via `SystemGraphNode::then`. This 
creates a new node from a system and adds a "after" dependency on the original
system.
```rust
let graph = SystemGraph::new();
graph
  .root(sys_a)
  .then(sys_b)
  .then(sys_c);

// Convert into a SystemSet
let system_set: SystemSet = graph.into();
```

### Fan Out
`SystemGraphNode::fork` can be used to fan out into multiple branches. All fanned out systems will not execute
until the original has finished, but do not have a mutual dependency on each other.
```rust
let graph = SystemGraph::new();

// Fork out from one original node.
// sys_b, sys_c, and sys_d will only start when sys_a finishes.
let (c, b, d) = graph.root(sys_a)
    .fork((
        sys_b,
        sys_c,
        sys_d,
    ));

// Alternatively, calling "then" repeatedly achieves the same thing.
let e = d.then(sys_e);
let f = d.then(sys_f);

// Convert into a SystemSet
let system_set: SystemSet = graph.into();
```

### Fan In
A graph node can wait on multiple systems before running via `SystemJoin::join`. 
The system will not run until all prior systems are finished.
```rust
let graph = SystemGraph::new();

let start_a = graph.root(sys_a);
let start_b = graph.root(sys_b);
let start_c = graph.root(sys_c);

(start_a, start_b, start_c)
    .join(sys_d)
    .then(sys_e);

// Convert into a SystemSet
let system_set: SystemSet = graph.into();
```

### Fan Out into Fan In
The types used to implement `fork` and `join` are composable.
```rust
let graph = SystemGraph::new();
graph.root(sys_a)
     .fork((sys_b, sys_c, sys_d))
     .join(sys_e)
     .then(sys_f);

// Convert into a SystemSet
let system_set: SystemSet = graph.into();
```

### Cloning
Individual [graph nodes] are backed by a `Rc`, so cloning it will still 
point to the same logical underlying graph.

[bevy]: https://bevyengine.org/