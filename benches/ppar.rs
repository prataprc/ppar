#![feature(test)]
extern crate test;

use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use test::Bencher;

use ppar::Vector;

#[bench]
fn bench_prepend(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut arr: Vector<u64> = Vector::new();
    b.iter(|| arr.insert(0, rng.gen::<u64>()).unwrap());

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_prepend n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
fn bench_append(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut arr: Vector<u64> = Vector::new();
    b.iter(|| arr.insert(arr.len(), rng.gen::<u64>()).unwrap());

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_append n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
fn bench_insert_rand(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());

    let mut arr: Vector<u64> = Vector::new();
    b.iter(|| {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>()).unwrap()
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_rand n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_get_100K(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut arr: Vector<u64> = Vector::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>()).unwrap()
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % arr.len();
        arr.get(off).unwrap();
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_get_100K n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_set_100K(b: &mut Bencher) {
    let seed: u128 = random();
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut arr: Vector<u64> = Vector::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>()).unwrap()
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % arr.len();
        arr.set(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_set_100K n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_delete_100K(b: &mut Bencher) {
    let seed: u128 = random();
    // let seed: u128 = 165591759058987334402931296907057276118;
    println!("seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut arr: Vector<u64> = Vector::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>()).unwrap();
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % arr.len();
        arr.remove(off).unwrap();
        arr.insert(off, rng.gen::<u64>()).unwrap();
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_delete_100K n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
fn bench_clone(b: &mut Bencher) {
    let seed: u128 = random();
    // let seed: u128 = 165591759058987334402931296907057276118;
    println!("seed {}", seed);
    let mut rng = SmallRng::from_seed(seed.to_le_bytes());
    let mut arr: Vector<u64> = Vector::new();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>()).unwrap();
    }
    let mut n = 0;
    b.iter(|| {
        let arr1 = arr.clone();
        n += arr1.len()
    });
}

fn mem_ratio(size: usize, mem: usize, n: usize) -> f64 {
    ((((mem as f64) / (n as f64)) - (size as f64)) / size as f64) * 100_f64
}
