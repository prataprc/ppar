use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use std::fmt;
use test::Bencher;

use super::*;

#[test]
fn test_rope_new() {
    let rr: Rope<u64> = Rope::new();
    assert!(rr.len() == 0);
}

#[test]
fn test_rope_crud() {
    let seed: u128 = random();
    // let seed: u128 = 89704735013013664095413923566273445973;
    println!("test_rope_insert1 seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ops = [0, 1, 2, 3, 10, 100, 1000, 10_000, 1000_000];
    for n in ops.iter() {
        // println!("n .. {}", n);
        let mut rr = Rope::new();
        let mut refv = vec![];

        for _ in 0..*n {
            rr = match rng.gen::<u8>() % 4 {
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

#[test]
fn test_rope_prepend() {
    let seed: u128 = random();
    // let seed: u128 = 89704735013013664095413923566273445973;
    println!("test_rope_prepend seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let ops = [10_000, 1000_000];
    for n in ops.iter() {
        // println!("n .. {}", n);
        let mut rr = Rope::new();
        let mut refv: Vec<u64> = vec![];

        for _i in 0..*n {
            let val = rng.gen::<u64>();
            // println!("off:{} val:{}", n - _i - 1, val);
            refv.push(val);
            let r1 = rr.insert(0, val).unwrap();
            assert_eq!(rr.len() + 1, r1.len());
            rr = r1
        }

        refv.reverse();
        validate_root(&rr.root, &refv);

        let ratio = mem_ratio(8, rr.footprint(), rr.len());
        println!(
            "ops:{}, n:{} footprint:{} mem_ratio:{}",
            n,
            rr.len(),
            rr.footprint(),
            ratio
        );
        validate(&rr, &refv);
    }
}

#[test]
fn test_rope_delete_skew() {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let mut rr: Rope<u64> = Rope::new();
    let mut refv = vec![];

    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        let val = rng.gen::<u64>();
        rr = rr.insert(off, val).unwrap();
        refv.insert(off, val);
    }

    for _ in 0..90_000 {
        let off = rng.gen::<usize>() % rr.len();
        rr = rr.delete(off).unwrap();
        refv.remove(off);
    }

    validate(&rr, &refv);

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("test_rope_delete_skew n:{} mem_ratio:{}%", rr.len(), ratio,);
}

#[bench]
fn bench_rope_prepend(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    b.iter(|| {
        rr = rr.insert(0, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rope_prepend n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
fn bench_rope_append(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    b.iter(|| {
        rr = rr.insert(rr.len(), rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rope_append n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
fn bench_rope_insert_rand(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    b.iter(|| {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rope_rand n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_rope_get_100K(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % rr.len();
        rr.get(off).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rope_get_100K n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_rope_set_100K(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % rr.len();
        rr = rr.set(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rope_set_100K n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_rope_delete_100K(b: &mut Bencher) {
    let seed: u128 = random();
    // let seed: u128 = 165591759058987334402931296907057276118;
    println!("seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % rr.len();
        rr = rr.delete(off).unwrap();
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rope_delete_100K n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
fn bench_rope_footprint_u128(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u128> = Rope::new();
    b.iter(|| {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u128>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!(
        "bench_rope_footprint_u128 n:{} mem_ratio:{}%",
        rr.len(),
        ratio
    );
}

#[bench]
fn bench_rope_footprint_u64_delete_skew(_: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Rope<u64> = Rope::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    }

    for _ in 0..90_000 {
        let off = rng.gen::<usize>() % rr.len();
        rr = rr.delete(off).unwrap();
    }

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!(
        "bench_rope_footprint_u64_delete_skew n:{} mem_ratio:{}% {}",
        rr.len(),
        ratio,
        rr.footprint()
    );
}

fn validate<T>(r: &Rope<T>, refv: &[T])
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

fn validate_root<T>(root: &Rc<Node<T>>, refv: &[T])
where
    T: fmt::Debug + Clone + Eq + PartialEq,
{
    let data = Rope::collect_zs(root)
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
    ((((mem as f64) / (n as f64)) - (size as f64)) / size as f64) * 100_f64
}
