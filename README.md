# Distributed ID Allocator

A library which generates small number representations of arbitrary non-colliding Version 4 UUIDs (stable IDs) across multiple sessions in a network. This scheme enables a distributed application to utilize the global uniqueness guarantees of UUIDs while maintaining the performance advantages of small integers.

## Overview

The distributed ID allocation scheme allows clients to use, store, and reference small numbers that map to UUIDs. The primary benefits are improved memory efficiency and reduced storage size, as well as improved runtime performance when used as keys in collections or other data structures in place of UUIDs.

The scheme requires a total order for ID allocation, so the allocator is designed to work within a service that provides centralized total order broadcast.

## Sessions

A session can be thought of as a unique identifier for a single allocator. Even if an allocator is taken offline, serialized to disk, and rehydrated back, it's considered the same session if it has the same session ID.

## Generation API

New IDs are generated via an API endpoint on the allocator. The API returns a small number representation of the UUID with the following guarantees:

- The ID can be converted into a full UUID at any time.
- Prior to the service being notified of the ID's creation, it is unique within the scope of the session (e.g. it can collide with IDs allocated by a different allocator in the same network).
- After the service has been notified of the ID's creation and has determined a total ordering between all clients creating IDs, the ID will have a _final_ form that is a small number representation that is unique across all sessions.

## ID Spaces

The allocation scheme separates the small-number IDs into two "spaces":

1. **Session space**: IDs are unique within the session scope, but are not guaranteed to be globally unique without the accompanying context of their session ID.
2. **Op space**: IDs are in their most final form, as unique as possible within the entire system. IDs that have been ordered by the central service ("finalized") are represented in their _final_ form, while any other IDs that have not yet been finalized are left in their _session space_ form.

Each of these spaces is represented by a discrete type. This allows for the direct encoding of an ID's uniqueness guarantees without contextual ambiguity.

## Normalization

The process of translating an ID from session space to op space or vice versa is called normalization.

## Usage Guidelines

- Allocated IDs can be delivered to application authors for use in place of V4 UUIDs.
- Expose only session space IDs to application authors. This means that application developers don't need to manage multiple ID spaces or worry about converting IDs between session space and op space. This simplifies the data management process and reduces the chances of errors or inconsistencies.
- Use op space IDs for serialized forms, such as sending information over the wire or storing it in a file. This is because op space IDs are unique within the entire system, whereas session space IDs are unique only within a specific session. When persisting data, it's crucial to ensure that IDs remain unique across different sessions, clients, or machines to avoid collisions or conflicts.
- When serializing op space IDs, annotate the entire context (e.g., file or network operation) with the session ID. This is necessary because not all IDs in op space are in their final form. Any non-finalized IDs will still be in their session space form, unique only within that specific session. Annotating the entire context with the session ID ensures that recipients of the serialized data can correctly interpret and process the non-finalized IDs.

## Efficiency Properties

### UUID Generation

The allocator generates UUIDs in non-random ways to reduce entropy, optimizing storage of data. A given session's UUIDs start from a random UUID, and subsequent IDs are allocated sequentially. UUIDs generated with this strategy are less likely to collide than fully random UUIDs, and this fact can be leveraged to compact the on-disk and in-memory representations of the allocator.

UUID generation occurs in constant time.

### Clustering

The sequential allocation approach allows the system to implement a clustering scheme.

As allocators across the network create IDs, they reserve a block of positive ID space (a cluster) for that allocator. The positive integer space is sharded across different clients, and session space IDs are mapped into these clusters.

Clusters are efficient to store, requiring only two integers: the base positive integer in the cluster and the count of reserved IDs in that cluster.

### Normalization Process

The normalization process involves a simple binary search on the cluster table stored by all allocators.

## Architecture

This library consists of three components:

1. A Rust crate containing the core allocator logic and data structures, as well as the full API surface
2. A TypeScript package which translates the API surface for integration into TypeScript or JavaScript codebases
3. A Rust crate which generates the Rust - TypeScript interop logic for compilation to WASM (via WASM Bindgen)

This version of the allocator is a rewrite of a prior version in the Microsoft Fluid Framework, which was implemented fully in TypeScript. This effort aimed to move the core allocator from TypeScript to Rust for performance gains and other benefits.

### Performance

Performance comparison of this allocator and the preexisting TypeScript allocator shows appreciable improvement across most operation types (including all hotpath operations). This is achieved even with the overhead of the TypeScript <-> WASM interop boundary.

### Rust codebase integration

The decision to separate the core allocator as a standalone crate, separate from the WASM interop logic, was made so that the allocator can also be integrated into Rust codebases.

# Building the repo

## Rust

To build the Rust workspace, run `cargo build` from the rust-wasm-id-allocator folder.

## TypeScript

- Debug
  - To build the TS/WASM package for debugging, run `npm run build` from the typescript-id-allocator folder.
- Benchmarking
  - To build the TS/WASM package for benchmarking, run `npm run build:bench` from the typescript-id-allocator folder.
- Release
  - To build the TS/WASM package for release, run `npm run build:release` from the typescript-id-allocator folder.
