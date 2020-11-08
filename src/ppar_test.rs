use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use std::fmt;

use super::*;

#[test]
fn test_new() {
    let arr: Vector<u64> = Vector::new();
    assert!(arr.len() == 0);
    println!("is thread-safe {}", arr.is_thread_safe());
}

#[test]
fn test_crud() {
    let seed: u128 = random();
    // let seed: u128 = 89704735013013664095413923566273445973;
    println!("test_crud seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ops = [0, 1, 2, 3, 10, 100, 1000, 10_000, 1000_000];
    for n in ops.iter() {
        let mut arr = Vector::new();
        let mut refv = vec![];

        for _ in 0..*n {
            match rng.gen::<u8>() % 4 {
                // get
                0 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();
                    assert_eq!(refv[off], *arr.get(off).unwrap());
                }
                // set
                2 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();
                    let val = rng.gen::<u64>();

                    refv[off] = val;
                    let n = arr.len();
                    arr.set(off, val).unwrap();
                    assert_eq!(arr.len(), n);
                }
                // delete
                3 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();

                    refv.remove(off);
                    let n = arr.len();
                    arr.remove(off).unwrap();
                    assert_eq!(arr.len(), n - 1);
                }
                // insert
                _ => {
                    let off = rng.gen::<usize>() % (arr.len() + 1);
                    let val = rng.gen::<u64>();

                    refv.insert(off, val);
                    let n = arr.len();
                    arr.insert(off, val).unwrap();
                    assert_eq!(arr.len(), n + 1);
                }
            };
        }
        let ratio = mem_ratio(8, arr.footprint(), arr.len());
        assert!(
            ratio < 100.0,
            "n:{} footprint:{} ratio:{}",
            arr.len(),
            arr.footprint(),
            ratio,
        );
        validate(&arr, &refv);
    }
}

#[test]
fn test_prepend() {
    let seed: u128 = random();
    // let seed: u128 = 89704735013013664095413923566273445973;
    println!("test_prepend seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ops = [10_000, 1000_000];
    for n in ops.iter() {
        let mut arr = Vector::new();
        let mut refv: Vec<u64> = vec![];

        for i in 0..*n {
            let val = rng.gen::<u64>();
            refv.push(val);
            arr.insert(0, val).unwrap();
            assert_eq!(arr.len(), i + 1);
        }

        refv.reverse();
        validate_root(&arr.root, &refv);

        let ratio = mem_ratio(8, arr.footprint(), arr.len());
        assert!(
            ratio < 100.0,
            "n:{} footprint:{} ratio:{}",
            arr.len(),
            arr.footprint(),
            ratio
        );
        validate(&arr, &refv);
    }
}

#[test]
fn test_delete_skew() {
    let seed: u128 = random();
    println!("test_delete_skew seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let mut arr: Vector<u64> = Vector::new();
    let mut refv = vec![];

    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        let val = rng.gen::<u64>();
        arr.insert(off, val).unwrap();
        refv.insert(off, val);
    }

    for _ in 0..90_000 {
        let off = rng.gen::<usize>() % arr.len();
        arr.remove(off).unwrap();
        refv.remove(off);
    }

    validate(&arr, &refv);

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    assert!(
        ratio < 100.0,
        "n:{} footprint:{} ratio:{}",
        arr.len(),
        arr.footprint(),
        ratio
    );
}

#[test]
fn test_from_slice() {
    let seed: u128 = random();
    println!("test_from_slice seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let vals: Vec<u64> = (0..1000_000).map(|_| rng.gen()).collect();
    let arr = Vector::from_slice(&vals, None);
    validate(&arr, &vals);

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    assert!(
        ratio < 100.0,
        "n:{} footprint:{} ratio:{}",
        arr.len(),
        arr.footprint(),
        ratio
    );
}

fn validate<T>(r: &Vector<T>, refv: &[T])
where
    T: fmt::Debug + Clone + Eq + PartialEq,
{
    assert_eq!(refv.len(), r.len());
    assert_eq!(r.len(), r.root.len());

    for (off, val) in refv.iter().enumerate() {
        assert_eq!(r.get(off).unwrap(), val, "off-{}", off);
    }

    assert!(r.get(r.len()).is_err());
}

fn validate_root<T>(root: &NodeRef<T>, refv: &[T])
where
    T: fmt::Debug + Clone + Eq + PartialEq,
{
    let data = Node::collect_zs(root)
        .into_iter()
        .map(|n| {
            if let Node::Z { data } = n.as_ref() {
                data.to_vec()
            } else {
                panic!()
            }
        })
        .flatten()
        .collect::<Vec<T>>();

    assert_eq!(data.len(), refv.len());
    assert_eq!(data, refv);
}

fn mem_ratio(size: usize, mem: usize, n: usize) -> f64 {
    match n {
        0 => {
            assert!(mem < 100, "{}", mem);
            0.1
        }
        n if n < 10 => {
            assert!(mem < ((10 * size) + 100), "{}", mem);
            0.1
        }
        n => ((((mem as f64) / (n as f64)) - (size as f64)) / size as f64) * 100_f64,
    }
}
