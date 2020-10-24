use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use std::fmt;

use super::*;

#[test]
fn test_rope_new() {
    let rr: Rope<u64> = Rope::new();
    assert!(rr.len() == 0);
}

#[test]
fn test_rope_crud() {
    let seed: u128 = random();
    let seed: u128 = 89704735013013664095413923566273445973;
    println!("test_rope_insert1 seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ops = [0, 1, 2, 3, 10, 100, 1000, 10_000, 1000_000];
    for n in ops.iter() {
        // println!("n .. {}", n);
        let mut rr = Rope::new();
        let mut refv = vec![];

        for _ in 0..*n {
            rr = match rng.gen::<u8>() % 2 {
                // get
                0 if rr.len() > 0 => {
                    let off = rng.gen::<usize>() % rr.len();
                    // println!("get op {}", off);
                    assert_eq!(refv[off], *rr.get(off).unwrap());
                    rr
                }
                // insert
                0 | 1 => {
                    let off = rng.gen::<usize>() % (rr.len() + 1);
                    let val = rng.gen::<u64>();
                    // println!("insert op {} {}", off, val);

                    refv.insert(off, val);
                    let r1 = rr.insert(off, val).unwrap();
                    assert_eq!(rr.len() + 1, r1.len());
                    r1
                }
                // set
                2 => {
                    let off = rng.gen::<usize>() % rr.len();
                    let val = rng.gen::<u64>();
                    // println!("set op {} {}", off, val);

                    refv[off] = val;
                    let r1 = rr.set(off, val).unwrap();
                    assert_eq!(rr.len() + 1, r1.len());
                    r1
                }
                // delete
                3 => {
                    let off = rng.gen::<usize>() % rr.len();
                    // println!("del op {}", off);

                    refv.remove(off);
                    let r1 = rr.delete(off).unwrap();
                    assert_eq!(rr.len() + 1, r1.len());
                    r1
                }
                _ => unreachable!(),
            };
        }
        println!("ops:{}, n:{} footprint:{}", n, rr.len(), rr.footprint());
        validate(&rr, &refv);
    }
}

fn validate<T: fmt::Debug + Clone + Eq + PartialEq>(r: &Rope<T>, refv: &[T]) {
    assert_eq!(refv.len(), r.len());
    assert_eq!(r.len(), r.calculate_len());

    for (off, val) in refv.iter().enumerate() {
        assert_eq!(r.get(off).unwrap(), val);
    }

    assert!(r.get(r.len()).is_err());
}
