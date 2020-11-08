[![Documentation](https://docs.rs/ppar/badge.svg?style=flat-square)](https://docs.rs/ppar)

Package implement persistent array using a variant of rope data structure.

Why would you want it ?
-----------------------

Array is implemented as [std::vec][std_vec] in rust-standard library.
For most cases that should be fine. But when we start working with arrays
that are super-large and/or step into requirements like  non-destructive
writes and concurrent access, we find [std::vec][std_vec] insufficient.
[im][im] is a popular alternative, but has `insert()` and `delete()`
penalties similar to `std::vec` for large arrays. While most implementation
prefer to use [RRB-Tree][rrb], `ppar` uses a modified version of
[Rope data structure][rope].

Fundamentally, it can be viewed as a binary-tree of array-blocks, where
each leaf-node is a contiguous-block of type `T` items, while intermediate
nodes only hold references to the child nodes - `left` and `right`.
To be more precise, intermediate nodes in the tree are organised similar
to rope structure, as a tuple of `(weight, left, right)` where weight is
the sum of all items present in the leaf-nodes under the left-branch.

A list of alternatives can be found [here][#alternate-solutions]. If you
find good alternatives please add it to the list and raise a PR.

If you are planning to use `ppar` for your project, do let us know.

Goals
-----

- [x] Vector parametrized over type T.
- [x] Immutable / Persistent collection of Vector<T>.
- [x] CRUD operation, get(), set(), delete(), insert(), all are persistent.
- [x] Convert from Vec<T> to ppar::Vector<T>.
- [ ] Convert from ppar::Vector<T> to Vec<T>.
- [x] Thread safe operations.
- [ ] [std::vec::Vec][std_vector] like mutable API.
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
- [ ] Parallel iteration with [rayon][rayon].
- [ ] Arbitrary implementation from [quickcheck][quickcheck].

The basic algorithm is fairly tight. Though we can make the `ppar::Vector`
type as rich as [std::vec::Vec][std_vector] and [im::Vector][im_vector].

Contributions
-------------

* Simple workflow. Fork, modify and raise a pull request.
* Use [rust-fuzz](https://rust-fuzz.github.io/book) for fuzz-testing.
  * TODO: run test.sh (unit-testing, unit-benchmarks, fuzz-testing).
  * TODO: run perf.sh
* TODO: Developer certificate of origin.

Benchmark
---------

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

Alternate solutions
-------------------

* [im][im]
* [rpds][rpds]

[im]: https://github.com/bodil/im-rs
[im_vector]: https://docs.rs/im/15.0.0/im/struct.Vector.html
[rope]: https://en.wikipedia.org/wiki/Rope_(data_structure)
[rpds]: https://github.com/orium/rpds
[std_vec]: https://doc.rust-lang.org/beta/std/vec/index.html
[std_vector]: https://doc.rust-lang.org/beta/std/vec/struct.Vec.html
[rrb]: https://infoscience.epfl.ch/record/213452/files/rrbvector.pdf
[rayon]: https://crates.io/crates/rayon
[quickcheck]: https://crates.io/crates/quickcheck
