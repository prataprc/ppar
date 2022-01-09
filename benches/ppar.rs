#![feature(test)]
extern crate test;

use rand::{prelude::random, rngs::StdRng, Rng, SeedableRng};

use test::Bencher;

use ppar::arc::Vector;

#[bench]
fn bench_prepend(b: &mut Bencher) {
    let seed: u64 = random();
    println!("bench_prepend seed:{}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    b.iter(|| {
        arr.insert(0, rng.gen::<u64>())
            .expect("bench_prepend: fail insert")
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_prepend n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
fn bench_append(b: &mut Bencher) {
    let seed: u64 = random();
    println!("bench_append seed:{}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    b.iter(|| {
        arr.insert(arr.len(), rng.gen::<u64>())
            .expect("bench_append: fail insert")
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_append n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
fn bench_insert_rand(b: &mut Bencher) {
    let seed: u64 = random();
    println!("bench_insert_rand seed:{}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    b.iter(|| {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>())
            .expect("bench_insert_rand: fail insert")
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_insert_rand n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_get_100K(b: &mut Bencher) {
    let seed: u64 = random();
    println!("bench_get_100K seed:{}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>())
            .expect("bench_get_100K: fail insert")
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % arr.len();
        arr.get(off).expect("bench_get_100K: fail get");
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_get_100K n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_update_100K(b: &mut Bencher) {
    let seed: u64 = random();
    println!("bench_update_100K seed:{}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>())
            .expect("bench_update_100K: fail insert")
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % arr.len();
        arr.update(off, rng.gen::<u64>())
            .expect("bench_update_100K: fail update");
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_update_100K n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
#[allow(non_snake_case)]
fn bench_delete_100K(b: &mut Bencher) {
    let seed: u64 = random();
    // let seed: u128 = 165591759058987334402931296907057276118;
    println!("bench_delete_100K seed {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>())
            .expect("bench_delete_100K: fail insert");
    }
    b.iter(|| {
        let off = rng.gen::<usize>() % arr.len();
        arr.remove(off).expect("bench_delete_100K: fail remove");
        arr.insert(off, rng.gen::<u64>())
            .expect("bench_delete_100K: fail reinsert");
    });

    let ratio = mem_ratio(8, arr.footprint(), arr.len());
    println!("bench_delete_100K n:{} mem_ratio:{}%", arr.len(), ratio);
}

#[bench]
fn bench_clone(b: &mut Bencher) {
    let seed: u64 = random();
    // let seed: u128 = 165591759058987334402931296907057276118;
    let mut rng = StdRng::seed_from_u64(seed);

    let mut arr: Vector<u64> = Vector::default();
    for _ in 0..100_000 {
        let off = rng.gen::<usize>() % (arr.len() + 1);
        arr.insert(off, rng.gen::<u64>())
            .expect("bench_clone: fail insert");
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
