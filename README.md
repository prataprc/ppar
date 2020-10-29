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
- [ ] Thread safe operations.

**Benchmark**:

On a 2010 8GB core2-duo machine ...

```bash
# Random insertion of u64 values.
test bench_insert_rand ... bench:       7,444 ns/iter (+/- 3,092)
# Sequential append of u64 values.
test bench_append      ... bench:       2,236 ns/iter (+/- 429)
# Random delete, followed by insert into same position, for a 100K collection.
test bench_delete_100K ... bench:       3,776 ns/iter (+/- 510)
# Random get over a 100K collection.
test bench_get_100K    ... bench:          63 ns/iter (+/- 0)
# Sequential prepend for u64 values.
test bench_prepend     ... bench:       2,924 ns/iter (+/- 427)
# Random set over a 100K collection.
test bench_set_100K    ... bench:       1,330 ns/iter (+/- 165)
```

Benchmark comparison with [im::Vector](https://docs.rs/im/15.0.0/im/struct.Vector.html)

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

im::Vector performance characterization
---------------------------------------

append-load(1000000 items)     : 43.340755ms
get(1000000 ops)               : 147ns
update(1000000 ops)            : 9.385µs
```

When compared to im::Vector,

* ppar::Vector supports persistent insert() and delete() operations.
* Around 4x faster when converting from Vec<T>.
* Random index operation is around 15% faster.
* Persistent update operation is around 2x faster.

**Alternate libraries**:

* _[im](https://github.com/bodil/im-rs)_
* _[rpds](https://github.com/orium/rpds)_
