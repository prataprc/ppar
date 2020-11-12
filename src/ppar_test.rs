use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

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
    // let seed: u128 = 148687161270367758201020080252240195663;
    println!("test_crud seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ops = [0, 1, 2, 3, 10, 100, 1000, 10_000, 1000_000];
    for n in ops.iter() {
        let mut arr = Vector::new();
        let mut refv = vec![];

        for _ in 0..*n {
            match rng.gen::<u8>() % 7 {
                // get
                0 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();
                    assert_eq!(refv[off], *arr.get(off).unwrap());
                }
                // update
                1 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();
                    let val = rng.gen::<u64>();

                    refv[off] = val;
                    let n = arr.len();
                    arr.update(off, val).unwrap();
                    assert_eq!(arr.len(), n);
                }
                // update mut
                2 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();
                    let val = rng.gen::<u64>();

                    refv[off] = val;
                    let n = arr.len();
                    arr.update_mut(off, val).unwrap();
                    assert_eq!(arr.len(), n);
                }
                // remove
                3 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();

                    refv.remove(off);
                    let n = arr.len();
                    arr.remove(off).unwrap();
                    assert_eq!(arr.len(), n - 1);
                }
                // remove mut
                4 if arr.len() > 0 => {
                    let off = rng.gen::<usize>() % arr.len();

                    refv.remove(off);
                    let n = arr.len();
                    arr.remove_mut(off).unwrap();
                    assert_eq!(arr.len(), n - 1);
                }
                // insert
                5 => {
                    let off = rng.gen::<usize>() % (arr.len() + 1);
                    let val = rng.gen::<u64>();

                    refv.insert(off, val);
                    let n = arr.len();
                    arr.insert(off, val).unwrap();
                    assert_eq!(arr.len(), n + 1);
                }
                // insert mut
                _ => {
                    let off = rng.gen::<usize>() % (arr.len() + 1);
                    let val = rng.gen::<u64>();

                    refv.insert(off, val);
                    let n = arr.len();
                    arr.insert_mut(off, val).unwrap();
                    assert_eq!(arr.len(), n + 1);
                }
            };
        }
        validate(&arr, &refv);
    }
}

#[test]
fn test_split_off() {
    let seed: u128 = random();
    // let seed: u128 = 252658238112610282069224390866000906287;
    println!("test_split_off seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ns = [10_000, 1000_000, 10_000_000];
    for n in ns.iter() {
        let mut refv: Vec<u64> = (0..*n).collect();
        let mut arr = Vector::from_slice(&refv, Some(128));

        while arr.len() > 0 {
            let off = rng.gen::<usize>() % arr.len();
            // println!("test_split_off off:{} len:{}", off, arr.len());
            let (a, b) = (arr.split_off(off).unwrap(), refv.split_off(off));
            arr = arr.rebalance().unwrap();
            validate(&a, &b);
            validate(&arr, &refv);
        }
    }
}

#[test]
fn test_append() {
    let seed: u128 = random();
    // let seed: u128 = 252658238112610282069224390866000906287;
    println!("test_append seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    for i in 1..100 {
        let mut a: Vec<u64> = (0..rng.gen::<u64>() % (i * 1000)).collect();
        let mut b: Vec<u64> = (0..rng.gen::<u64>() % (i * 1000)).collect();

        let mut x = Vector::from_slice(&a, None);
        let y = Vector::from_slice(&b, None);

        a.append(&mut b);
        x.append(y);

        validate(&x, &a);
    }
}

#[test]
fn test_prepend() {
    let seed: u128 = random();
    // let seed: u128 = 252658238112610282069224390866000906287;
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
}

#[test]
fn test_from_slice() {
    let seed: u128 = random();
    println!("test_from_slice seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let vals: Vec<u64> = (0..1000_000).map(|_| rng.gen()).collect();
    let arr = Vector::from_slice(&vals, None);
    validate(&arr, &vals);
}

#[test]
fn test_to_vec() {
    let seed: u128 = random();
    println!("test_from_slice seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let vals: Vec<u64> = (0..1000_000).map(|_| rng.gen()).collect();
    let vect: Vec<u64> = Vector::from_slice(&vals, None).into();
    assert!(vals == vect);
}

#[test]
fn test_iter() {
    let seed: u128 = random();
    println!("test_iter seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let vals: Vec<u64> = (0..1000_000).map(|_| rng.gen()).collect();
    let arr = Vector::from_slice(&vals, None);
    let iter_vals: Vec<u64> = arr.iter().map(|x| *x).collect();

    println!("{:?}", iter_vals);
    assert_eq!(vals, iter_vals);
}

#[test]
fn test_into_iter() {
    let seed: u128 = random();
    println!("test_iter seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let vals: Vec<u64> = (0..1000_000).map(|_| rng.gen()).collect();
    let arr = Vector::from_slice(&vals, None);
    let iter_vals: Vec<u64> = arr.into_iter().collect();

    println!("{:?}", iter_vals);
    assert_eq!(vals, iter_vals);
}

#[test]
fn test_rebalance() {
    let seed: u128 = random();
    println!("test_rebalance seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let mut arr = Vector::new();
    let mut refv: Vec<u64> = vec![];

    for _i in 0..10_000 {
        arr = arr.rebalance().unwrap();

        let val = rng.gen::<u64>();
        refv.push(val);
        arr.insert(0, val).unwrap();
    }

    refv.reverse();
    validate(&arr, &refv);
}
