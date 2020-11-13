use arbitrary::{self, unstructured::Unstructured, Arbitrary};
use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};
use structopt::StructOpt;

use std::collections::BTreeMap;
use std::{fmt, thread};

use ppar;

// TODO: arbitrary auto-rebalance

#[macro_export]
macro_rules! pp {
    ($($arg:expr),+ => $val:expr) => {
        println!("{:<30} : {:?}", format!($($arg),+), $val)
    };
}

/// Command line options.
#[derive(Clone, StructOpt)]
pub struct Opt {
    #[structopt(long = "seed")]
    seed: Option<u128>,

    #[structopt(long = "load", default_value = "10000000")] // default 10M
    load: usize,

    #[structopt(long = "ops", default_value = "1000000")]
    ops: usize,

    #[structopt(long = "threads", default_value = "4")]
    threads: usize,
}

fn main() {
    let mut opts = Opt::from_args();
    opts.seed = Some(opts.seed.unwrap_or(random()));
    // opts.seed = Some(43412938081234232274443750093662763225);
    println!("seed: {}", opts.seed.unwrap());

    let rets: Vec<(Vec<*const u8>, usize)> = match opts.threads {
        0 => unreachable!(),
        1 => {
            let (arr, vec) = initialize_rc::<u64>(&opts);
            vec![fuzzy_ops_rc(0, arr, vec, &opts).fetch_multiversions()]
        }
        _ => {
            let (arr, vec) = initialize_arc::<u64>(&opts);

            let mut handles = vec![];
            for i in 0..opts.threads {
                let (arr, vec) = (arr.clone(), vec.clone());
                let opts = opts.clone();
                handles.push(thread::spawn(move || fuzzy_ops_arc(i, arr, vec, &opts)));
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
            "thread-{} number of multi-reference nodes {} / {}",
            id,
            ptrs.len(),
            total_nodes
        );
    }
    println!("total shared nodes {}", map.len());
}

macro_rules! initialize {
    ($func:ident, $ref:ident) => {
        fn $func<T>(opts: &Opt) -> (ppar::$ref::Vector<T>, Vec<T>)
        where
            T: fmt::Debug + Clone + Eq + PartialEq + Arbitrary,
            rand::distributions::Standard: rand::distributions::Distribution<T>,
        {
            let mut rng = SmallRng::from_seed(opts.seed.unwrap().to_le_bytes());
            let bytes = rng.gen::<[u8; 32]>();
            let mut uns = Unstructured::new(&bytes);

            let mut arr = ppar::$ref::Vector::<T>::new();
            let k = std::mem::size_of::<T>();
            let leaf_cap = *uns.choose(&[k * 100, k * 1000, k * 10000]).unwrap();
            println!("leaf_cap: {}", leaf_cap);
            arr.set_leaf_size(leaf_cap);
            arr.set_auto_rebalance(true);

            let prepend_load = opts.load / 2;
            let append_load = opts.load - prepend_load;

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

            ppar::$ref::validate(&arr, &vec);
            println!("fuzzy load {} items", arr.len());

            (arr, vec)
        }
    };
}

initialize!(initialize_arc, arc);
initialize!(initialize_rc, rc);

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
            mut arr: ppar::$ref::Vector<T>,
            mut vec: Vec<T>,
            opts: &Opt,
        ) -> ppar::$ref::Vector<T>
        where
            T: fmt::Debug + Clone + Eq + PartialEq + Arbitrary,
        {
            let seed = opts.seed.unwrap() + (((id as u128) + 100) * 123);
            let mut rng = SmallRng::from_seed(seed.to_le_bytes());

            let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
            for _ in 0..opts.ops {
                let op: Op<T> = {
                    let bytes: [u8; 32] = rng.gen();
                    let mut uns = Unstructured::new(&bytes);
                    uns.arbitrary().unwrap()
                };

                op.count(&mut counts);

                match op {
                    Op::ToFromVec(leaf_size) => {
                        let a = ppar::$ref::Vector::from_slice(&vec, Some(leaf_size));
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
                    Op::InsertMut(Index(off), val) if opts.threads == 1 && off <= arr.len() => {
                        arr.insert_mut(off, val.clone()).unwrap();
                        vec.insert(off, val);
                    }
                    Op::InsertMut(Index(off), val) if opts.threads == 1 => {
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
                    Op::RemoveMut(Index(off)) if opts.threads == 1 && off < arr.len() => {
                        let a = arr.remove_mut(off).unwrap();
                        let b = vec.remove(off);
                        assert_eq!(a, b);
                    }
                    Op::RemoveMut(Index(off)) if opts.threads == 1 => {
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
                    Op::UpdateMut(Index(off), val) if opts.threads == 1 && off < arr.len() => {
                        let a = arr.update(off, val.clone()).ok();
                        let b = vec.get(off).cloned().map(|x| {
                            vec[off] = val;
                            x
                        });
                        assert_eq!(a, b);
                    }
                    Op::UpdateMut(Index(off), val) if opts.threads == 1 => {
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

            ppar::$ref::validate(&arr, &vec);

            println!(
                "validated thread-{} using {} ops with {} items",
                id,
                opts.ops,
                arr.len()
            );

            for (k, v) in counts.iter() {
                println!("{:14}: {}", k, v)
            }

            arr
        }
    };
}

fuzzy_ops!(fuzzy_ops_arc, arc);
fuzzy_ops!(fuzzy_ops_rc, rc);
