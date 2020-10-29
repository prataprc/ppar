#![feature(test)]
extern crate test;

use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use test::Bencher;

use ppar::Vector;

#[bench]
fn bench_prepend(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Vector<u64> = Vector::new();
    b.iter(|| {
        rr = rr.insert(0, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_prepend n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
fn bench_append(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Vector<u64> = Vector::new();
    b.iter(|| {
        rr = rr.insert(rr.len(), rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_append n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
fn bench_insert_rand(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let mut rr: Vector<u64> = Vector::new();
    b.iter(|| {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_rand n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_get_100K(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Vector<u64> = Vector::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % rr.len();
        rr.get(off).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_get_100K n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_set_100K(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Vector<u64> = Vector::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (rr.len() + 1);
        rr = rr.insert(off, rng.gen::<u64>()).unwrap();
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % rr.len();
        rr = rr.set(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, rr.footprint(), rr.len());
    println!("bench_set_100K n:{} mem_ratio:{}%", rr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_delete_100K(b: &mut Bencher) {
    let seed: u128 = random();
    // let seed: u128 = 165591759058987334402931296907057276118;
    println!("seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut rr: Vector<u64> = Vector::new();
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
    println!("bench_delete_100K n:{} mem_ratio:{}%", rr.len(), ratio);
}

fn mem_ratio(size: usize, mem: usize, n: usize) -> f64 {
    ((((mem as f64) / (n as f64)) - (size as f64)) / size as f64) * 100_f64
}
