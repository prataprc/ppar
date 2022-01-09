use arbitrary::{self, unstructured::Unstructured, Arbitrary};
use rand::{prelude::random, rngs::StdRng, Rng, SeedableRng};

use std::collections::BTreeMap;
use std::{fmt, thread};

use super::*;

#[test]
fn test_fuzzy() {
    let seed: u64 = random();
    let mut rng = StdRng::seed_from_u64(seed);

    let is_rc = Vector::<u64>::is_rc_type();

    let n_threads: usize = match is_rc {
        false => [2, 4, 8, 16, 32, 64][rng.gen::<usize>() % 6],
        true => 1,
    };
    let n_loads: usize = [0, 1, 1000, 1_000_000, 10_000_000][rng.gen::<usize>() % 5];
    let n_ops: usize = [0, 1, 1_000, 10_000, 100_000, 1_000_000][rng.gen::<usize>() % 6];

    println!(
        "test_fuzzy(rc:{}) seed:{} n_loads:{} n_ops:{} n_threads:{}",
        is_rc, seed, n_loads, n_ops, n_threads
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
            let n = map.get(&ptr).copied().unwrap_or(0);
            map.insert(ptr, n + 1);
        }
        println!(
            "test_fuzzy(rc:{}) thread-{} number of multi-reference nodes {} / {}",
            is_rc,
            id,
            ptrs.len(),
            total_nodes
        );
    }
    println!("test_fuzzy(rc:{}) total shared nodes {}", is_rc, map.len());
}

macro_rules! initialize {
    ($func:ident, $ref:ident) => {
        fn $func<T>(seed: u64, n_loads: usize) -> (crate::$ref::Vector<T>, Vec<T>)
        where
            T: fmt::Debug + Clone + Eq + PartialEq + Arbitrary,
            rand::distributions::Standard: rand::distributions::Distribution<T>,
        {
            let mut rng = StdRng::seed_from_u64(seed);

            let bytes = rng.gen::<[u8; 32]>();
            let mut uns = Unstructured::new(&bytes);

            let is_rc = Vector::<T>::is_rc_type();

            let k = std::mem::size_of::<T>();
            let leaf_cap = *uns.choose(&[k * 10, k * 100, k * 1000, k * 10000]).unwrap();

            println!("test_fuzzy(rc:{}) leaf_cap:{}", is_rc, leaf_cap);

            let mut vec: Vec<T> = Vec::default();
            for _i in 0..n_loads {
                vec.push(rng.gen());
            }

            let mut arr = crate::$ref::Vector::from_slice(&vec, Some(leaf_cap));
            arr.set_auto_rebalance(uns.arbitrary().unwrap());

            crate::$ref::validate(&arr, &vec);
            println!("test_fuzzy(rc:{}) load {} items", is_rc, arr.len());

            (arr, vec)
        }
    };
}

initialize!(do_initial_arc, arc);
initialize!(do_initial_rc, rc);

#[derive(Arbitrary, Clone, Debug)]
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

#[derive(Clone, Debug)]
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
            seed: u64,
            n_ops: usize,
            n_threads: usize,
        ) -> crate::$ref::Vector<T>
        where
            T: fmt::Debug + Clone + Eq + PartialEq + Arbitrary,
        {
            let seed = seed + (((id as u64) + 100) * 123);
            let mut rng = StdRng::seed_from_u64(seed);

            let is_rc = Vector::<T>::is_rc_type();

            let (
                mut n_to_from_vec,
                mut n_clone,
                mut n_footprint,
                mut n_into_iter,
                mut n_iter,
            ) = (0, 0, 0, 0, 0);

            let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
            for _i in 0..n_ops {
                let op: Op<T> = {
                    let bytes: [u8; 32] = rng.gen();
                    let mut uns = Unstructured::new(&bytes);
                    uns.arbitrary().unwrap()
                };

                let ok = match op.clone() {
                    Op::ToFromVec(leaf_size) if n_to_from_vec < 5 => {
                        let a = crate::$ref::Vector::from_slice(&vec, Some(leaf_size));
                        let a: Vec<T> = a.into();
                        assert_eq!(a, vec);
                        n_to_from_vec += 1;
                        true
                    }
                    Op::ToFromVec(_) => false,
                    Op::Clone if n_clone < 5 => {
                        let a = arr.clone();
                        let a: Vec<T> = a.into();
                        assert_eq!(a, vec);
                        n_clone += 1;
                        true
                    }
                    Op::Clone => false,
                    Op::Len => {
                        assert_eq!(arr.len(), vec.len());
                        true
                    }
                    Op::Footprint if n_footprint < 5 => {
                        arr.footprint();
                        n_footprint += 1;
                        true
                    }
                    Op::Footprint => false,
                    Op::Insert(Index(off), val) if off <= arr.len() => {
                        arr.insert(off, val.clone()).unwrap();
                        vec.insert(off, val);
                        true
                    }
                    Op::Insert(Index(off), val) => {
                        assert!(arr.insert(off, val.clone()).is_err());
                        true
                    }
                    Op::InsertMut(Index(off), val)
                        if n_threads == 1 && off <= arr.len() =>
                    {
                        arr.insert_mut(off, val.clone()).unwrap();
                        vec.insert(off, val);
                        true
                    }
                    Op::InsertMut(Index(off), val) if n_threads == 1 => {
                        assert!(arr.insert_mut(off, val.clone()).is_err());
                        true
                    }
                    Op::InsertMut(_, _) => false,
                    Op::Remove(Index(off)) if off < arr.len() => {
                        let a = arr.remove(off).unwrap();
                        let b = vec.remove(off);
                        assert_eq!(a, b);
                        true
                    }
                    Op::Remove(Index(off)) => {
                        assert!(arr.remove(off).is_err());
                        true
                    }
                    Op::RemoveMut(Index(off)) if n_threads == 1 && off < arr.len() => {
                        let a = arr.remove_mut(off).unwrap();
                        let b = vec.remove(off);
                        assert_eq!(a, b);
                        true
                    }
                    Op::RemoveMut(Index(off)) if n_threads == 1 => {
                        assert!(arr.remove_mut(off).is_err());
                        true
                    }
                    Op::RemoveMut(_) => false,
                    Op::Update(Index(off), val) if off < arr.len() => {
                        let a = arr.update(off, val.clone()).ok();
                        let b = vec.get(off).cloned().map(|x| {
                            vec[off] = val;
                            x
                        });
                        assert_eq!(a, b);
                        true
                    }
                    Op::Update(Index(off), val) => {
                        assert!(arr.update(off, val).is_err());
                        true
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
                        true
                    }
                    Op::UpdateMut(Index(off), val) if n_threads == 1 => {
                        assert!(arr.update(off, val).is_err());
                        true
                    }
                    Op::UpdateMut(_, _) => false,
                    Op::Get(Index(off)) => {
                        assert_eq!(arr.get(off).ok(), vec.get(off));
                        true
                    }
                    Op::IntoIter if n_into_iter < 5 => {
                        let ii = arr.clone().into_iter();
                        let ij = vec.clone().into_iter();
                        let a: Vec<T> = ii.collect();
                        let b: Vec<T> = ij.collect();
                        assert_eq!(a, b);
                        n_into_iter += 1;
                        true
                    }
                    Op::Iter if n_iter < 5 => {
                        let a: Vec<T> = arr.iter().map(|x| x.clone()).collect();
                        let b: Vec<T> = vec.iter().map(|x| x.clone()).collect();
                        assert_eq!(a, b);
                        n_iter += 1;
                        true
                    }
                    Op::IntoIter | Op::Iter => false,
                    Op::SplitOff(Index(off)) if off < arr.len() => {
                        let a = arr.split_off(off).unwrap();
                        arr.append(a);
                        let mut b = vec.split_off(off);
                        vec.append(&mut b);
                        true
                    }
                    Op::SplitOff(Index(off)) => {
                        assert!(arr.split_off(off).is_err());
                        true
                    }
                };

                if ok {
                    op.count(&mut counts);
                }
            }

            crate::$ref::validate(&arr, &vec);

            println!(
                "test_fuzzy(rc:{}) validated thread-{} using {} ops with {} items",
                is_rc,
                id,
                n_ops,
                arr.len()
            );

            for (k, v) in counts.iter() {
                println!("test_fuzzy(rc:{}) {:14}: {}", is_rc, k, v)
            }

            arr
        }
    };
}

fuzzy_ops!(do_incremental_arc, arc);
fuzzy_ops!(do_incremental_rc, rc);
