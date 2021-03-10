use arbitrary::{self, unstructured::Unstructured, Arbitrary};
use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use std::collections::BTreeMap;
use std::{fmt, thread};

use super::*;

#[test]
fn test_fuzzy() {
    let seed: u128 = random();
    // let seed: u128 = 148687161270367758201020080252240195663;
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let n_loads: usize = [0, 1, 1000, 1_000_000, 10_000_000][rng.gen::<usize>() % 5];
    let n_ops: usize = [0, 1, 1000, 1_000_000][rng.gen::<usize>() % 4];
    let n_threads: usize = [1, 2, 4, 8, 16, 256, 1024][rng.gen::<usize>() % 7];

    println!(
        "test_fuzzy seed:{} n_loads:{} n_ops:{} n_threads:{}",
        seed, n_loads, n_ops, n_threads
    );

    let rets: Vec<(Vec<*const u8>, usize)> = match n_threads {
        0 => unreachable!(),
        1 => {
            let (arr, vec) = do_initial_rc::<u64>(seed, n_loads);
            vec![do_incremental_rc(0, arr, vec, seed, n_ops, n_threads)
                .fetch_multiversions()]
        }
        _ => {
            let (arr, vec) = do_initial_arc::<u64>(seed, n_loads);

            let mut handles = vec![];
            for i in 0..n_threads {
                let (arr, vec) = (arr.clone(), vec.clone());
                handles.push(thread::spawn(move || {
                    do_incremental_arc(i, arr, vec, seed, n_ops, n_threads)
                }));
            }

            handles
                .into_iter()
                .map(|h| h.join().unwrap().fetch_multiversions())
                .collect()
        }
    };

    let mut map: BTreeMap<*const u8, u32> = BTreeMap::new();
    for (id, (ptrs, total_nodes)) in rets.into_iter().enumerate() {
        for ptr in ptrs.clone().into_iter() {
            let n = map.get(&ptr).map(|x| *x).unwrap_or(0);
            map.insert(ptr, n + 1);
        }
        println!(
            "test_fuzzy thread-{} number of multi-reference nodes {} / {}",
            id,
            ptrs.len(),
            total_nodes
        );
    }
    println!("test_fuzzy total shared nodes {}", map.len());
}

macro_rules! initialize {
    ($func:ident, $ref:ident) => {
        fn $func<T>(seed: u128, n_loads: usize) -> (crate::$ref::Vector<T>, Vec<T>)
        where
            T: fmt::Debug + Clone + Eq + PartialEq + Arbitrary,
            rand::distributions::Standard: rand::distributions::Distribution<T>,
        {
            let mut rng = SmallRng::from_seed(seed.to_le_bytes());
            let bytes = rng.gen::<[u8; 32]>();
            let mut uns = Unstructured::new(&bytes);

            let mut arr = crate::$ref::Vector::<T>::new();

            let k = std::mem::size_of::<T>();
            let leaf_cap = *uns.choose(&[k * 10, k * 100, k * 1000, k * 10000]).unwrap();
            println!("test_fuzzy leaf_cap: {}", leaf_cap);
            arr.set_leaf_size(leaf_cap);
            arr.set_auto_rebalance(true);

            let prepend_load = n_loads / 2;
            let append_load = n_loads - prepend_load;

            let mut vec: Vec<T> = arr.clone().into();
            for _i in 0..prepend_load {
                let val: T = rng.gen();
                arr.insert(0, val.clone()).unwrap();
                vec.push(val);
            }
            vec.reverse();

            for _i in 0..append_load {
                let val: T = rng.gen();
                arr.insert(arr.len(), val.clone()).unwrap();
                vec.push(val);
            }

            arr.set_auto_rebalance(uns.arbitrary().unwrap());

            crate::$ref::validate(&arr, &vec);
            println!("test_fuzzy load {} items", arr.len());

            (arr, vec)
        }
    };
}

initialize!(do_initial_arc, arc);
initialize!(do_initial_rc, rc);

#[derive(Arbitrary)]
enum Op<T>
where
    T: Clone,
{
    ToFromVec(usize), // (leaf_size)
    Clone,
    Len,
    Footprint,
    Insert(Index, T),
    InsertMut(Index, T),
    Remove(Index),
    RemoveMut(Index),
    Update(Index, T),
    UpdateMut(Index, T),
    Get(Index),
    IntoIter,
    Iter,
    SplitOff(Index),
}

impl<T> Op<T>
where
    T: Clone,
{
    fn count(&self, counts: &mut BTreeMap<&'static str, usize>) {
        let key = match self {
            Op::ToFromVec(_) => "to_from_vec",
            Op::Clone => "clone",
            Op::Len => "len",
            Op::Footprint => "footprint",
            Op::Insert(_, _) => "insert",
            Op::InsertMut(_, _) => "insert_mut",
            Op::Remove(_) => "remove",
            Op::RemoveMut(_) => "remove_mut",
            Op::Update(_, _) => "update",
            Op::UpdateMut(_, _) => "update_mut",
            Op::Get(_) => "get",
            Op::IntoIter => "into_iter",
            Op::Iter => "iter",
            Op::SplitOff(_) => "split_off",
        };
        let val = counts.get(key).map(|v| v + 1).unwrap_or(1);
        counts.insert(key, val);
    }
}

struct Index(usize);

impl Arbitrary for Index {
    fn arbitrary(u: &mut Unstructured) -> arbitrary::Result<Self> {
        let index: usize = u.arbitrary().unwrap();
        Ok(Index(index % 100_000_000))
    }
}

macro_rules! fuzzy_ops {
    ($func:ident, $ref:ident) => {
        fn $func<T>(
            id: usize,
            mut arr: crate::$ref::Vector<T>,
            mut vec: Vec<T>,
            seed: u128,
            n_ops: usize,
            n_threads: usize,
        ) -> crate::$ref::Vector<T>
        where
            T: fmt::Debug + Clone + Eq + PartialEq + Arbitrary,
        {
            let seed = seed + (((id as u128) + 100) * 123);
            let mut rng = SmallRng::from_seed(seed.to_le_bytes());

            let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
            for _ in 0..n_ops {
                let op: Op<T> = {
                    let bytes: [u8; 32] = rng.gen();
                    let mut uns = Unstructured::new(&bytes);
                    uns.arbitrary().unwrap()
                };

                op.count(&mut counts);

                match op {
                    Op::ToFromVec(leaf_size) => {
                        let a = crate::$ref::Vector::from_slice(&vec, Some(leaf_size));
                        let a: Vec<T> = a.into();
                        assert_eq!(a, vec);
                    }
                    Op::Clone => {
                        let a = arr.clone();
                        let a: Vec<T> = a.into();
                        assert_eq!(a, vec);
                    }
                    Op::Len => {
                        assert_eq!(arr.len(), vec.len());
                    }
                    Op::Footprint => {
                        arr.footprint();
                    }
                    Op::Insert(Index(off), val) if off <= arr.len() => {
                        arr.insert(off, val.clone()).unwrap();
                        vec.insert(off, val);
                    }
                    Op::Insert(Index(off), val) => {
                        assert!(arr.insert(off, val.clone()).is_err());
                    }
                    Op::InsertMut(Index(off), val)
                        if n_threads == 1 && off <= arr.len() =>
                    {
                        arr.insert_mut(off, val.clone()).unwrap();
                        vec.insert(off, val);
                    }
                    Op::InsertMut(Index(off), val) if n_threads == 1 => {
                        assert!(arr.insert_mut(off, val.clone()).is_err())
                    }
                    Op::InsertMut(_, _) => (),
                    Op::Remove(Index(off)) if off < arr.len() => {
                        let a = arr.remove(off).unwrap();
                        let b = vec.remove(off);
                        assert_eq!(a, b);
                    }
                    Op::Remove(Index(off)) => {
                        assert!(arr.remove(off).is_err());
                    }
                    Op::RemoveMut(Index(off)) if n_threads == 1 && off < arr.len() => {
                        let a = arr.remove_mut(off).unwrap();
                        let b = vec.remove(off);
                        assert_eq!(a, b);
                    }
                    Op::RemoveMut(Index(off)) if n_threads == 1 => {
                        assert!(arr.remove_mut(off).is_err())
                    }
                    Op::RemoveMut(_) => (),
                    Op::Update(Index(off), val) if off < arr.len() => {
                        let a = arr.update(off, val.clone()).ok();
                        let b = vec.get(off).cloned().map(|x| {
                            vec[off] = val;
                            x
                        });
                        assert_eq!(a, b);
                    }
                    Op::Update(Index(off), val) => {
                        assert!(arr.update(off, val).is_err());
                    }
                    Op::UpdateMut(Index(off), val)
                        if n_threads == 1 && off < arr.len() =>
                    {
                        let a = arr.update(off, val.clone()).ok();
                        let b = vec.get(off).cloned().map(|x| {
                            vec[off] = val;
                            x
                        });
                        assert_eq!(a, b);
                    }
                    Op::UpdateMut(Index(off), val) if n_threads == 1 => {
                        assert!(arr.update(off, val).is_err())
                    }
                    Op::UpdateMut(_, _) => (),
                    Op::Get(Index(off)) => {
                        assert_eq!(arr.get(off).ok(), vec.get(off));
                    }
                    Op::IntoIter => {
                        let ii = arr.clone().into_iter();
                        let ij = vec.clone().into_iter();
                        let a: Vec<T> = ii.collect();
                        let b: Vec<T> = ij.collect();
                        assert_eq!(a, b);
                    }
                    Op::Iter => {
                        let a: Vec<T> = arr.iter().map(|x| x.clone()).collect();
                        let b: Vec<T> = vec.iter().map(|x| x.clone()).collect();
                        assert_eq!(a, b);
                    }
                    Op::SplitOff(Index(off)) if off < arr.len() => {
                        let a = arr.split_off(off).unwrap();
                        arr.append(a);
                        let mut b = vec.split_off(off);
                        vec.append(&mut b);
                    }
                    Op::SplitOff(Index(off)) => assert!(arr.split_off(off).is_err()),
                }
            }

            crate::$ref::validate(&arr, &vec);

            println!(
                "test_fuzzy validated thread-{} using {} ops with {} items",
                id,
                n_ops,
                arr.len()
            );

            for (k, v) in counts.iter() {
                println!("test_fuzzy {:14}: {}", k, v)
            }

            arr
        }
    };
}

fuzzy_ops!(do_incremental_arc, arc);
fuzzy_ops!(do_incremental_rc, rc);
