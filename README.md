Package implement persistent array using a variant of rope data structure.

Fundamentally, it can be viewed as a binary-tree of array-blocks, where
each leaf-node is a contiguous-block of type `T` items, while intermediate
nodes only hold references to the child nodes, left and right.
To be more precise, intermediate nodes in the tree are organised similar
to rope structure, as a tuple of (weight, left, right) where weight is
the sum of all items present in the leaf-nodes under the left-branch.

**Stated goals**:

- [x] Vector parametrized over type T.
- [x] Immutable / Persistent collection of Vector<T>.
- [x] CRUD operation, get(), set(), delete(), insert(), all are persistent.
- [x] Convert from Vec<T> to ppar::Vector<T>.
- [ ] Convert from ppar::Vector<T> to Vec<T>.
- [x] Thread safe operations.
- [ ] std::vec::Vec like mutable API.
- [ ] Iteration over collection, item-wise, chunk-wise, reverse.
- [ ] Deduplication.
- [ ] Membership.
- [ ] Joining collections, splitting into collections.
- [ ] Partial collection.
- [ ] Extending collection.
- [ ] Queue operations, like pop(), push().
- [ ] Functional ops, like filter, map, reduce.
- [ ] Sort and search operations.
- [ ] Trait implementations.
  - [x] Clone
  - [ ] Eq, PartialEq, Ord, PartialOrd
  - [ ] Extend
  - [ ] From, FromIterator, IntoIterator
  - [ ] Hash
  - [ ] Index, IndexMut
  - [ ] Write
- [ ] Parallel iteration with [rayon](https://crates.io/crates/rayon).
- [ ] Arbitrary implementation from [quickcheck](https://crates.io/crates/quickcheck).

**Benchmark**:

On a 2010 8GB core2-duo machine, thread safe:

```bash
test bench_append      ... bench:       2,892 ns/iter (+/- 591)
test bench_delete_100K ... bench:       4,557 ns/iter (+/- 496)
test bench_get_100K    ... bench:          65 ns/iter (+/- 0)
test bench_insert_rand ... bench:       8,212 ns/iter (+/- 3,156)
test bench_prepend     ... bench:       3,631 ns/iter (+/- 479)
test bench_set_100K    ... bench:       1,670 ns/iter (+/- 149)
```


On a 2010 8GB core2-duo machine, single threaded:

```bash
test bench_append      ... bench:       2,214 ns/iter (+/- 304)
test bench_delete_100K ... bench:       3,876 ns/iter (+/- 499)
test bench_get_100K    ... bench:          64 ns/iter (+/- 0)
test bench_insert_rand ... bench:       6,105 ns/iter (+/- 2,989)
test bench_prepend     ... bench:       2,833 ns/iter (+/- 470)
test bench_set_100K    ... bench:       1,307 ns/iter (+/- 83)
```

Via performance application,

```text
ppar::Vector performance characterization
-----------------------------------------
append-load(1000000 items)     : 8.918079ms
random-load(1000000 items)     : 5.854µs
get(1000000 ops)               : 124ns
set(1000000 ops)               : 5.489µs
delete-insert(1000000 ops)     : 10.313µs
overhead                       : "37.19%"
overhead after 90% delete      : "33.30%"
```

**Alternate libraries**:

* _[im](https://github.com/bodil/im-rs)_
* _[rpds](https://github.com/orium/rpds)_
