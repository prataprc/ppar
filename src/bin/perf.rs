#[cfg(not(feature = "im-rc"))]
use im::Vector as ImVector;
#[cfg(feature = "im-rc")]
use im_rc::Vector as ImVector;
use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};
use structopt::StructOpt;

use std::time;

use ppar;

// NOTE: im::Vector does not remove/delete op.
// NOTE: im::Vector, how to measure the value footprint.

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

    #[structopt(long = "load", default_value = "1000000")] // default 1M
    load: usize,

    #[structopt(long = "ops", default_value = "1000000")] // default 1M
    ops: usize,

    #[structopt(long = "leaf-size")]
    leaf_size: Option<usize>,

    #[structopt(long = "im-vector")]
    im_vector: bool,
}

fn main() {
    let mut opts = Opt::from_args();
    let mut rng = {
        let seed = opts.seed.unwrap_or(random());
        // let seed: u128 = 89704735013013664095413923566273445973;
        SmallRng::from_seed(seed.to_le_bytes())
    };

    println!("\nppar::Vector performance characterization");
    println!("-----------------------------------------");
    let mut arr: ppar::Vector<u64> = ppar::Vector::new();
    opts.leaf_size.map(|s| arr.set_leaf_size(s));
    let mut arr = ppar_load(arr, &mut opts, &mut rng);
    arr = ppar_ops(arr, &mut opts, &mut rng);
    ppar_delete_skew(arr, &mut rng);

    if opts.im_vector {
        println!("\nim::Vector performance characterization");
        println!("---------------------------------------\n");
        let arr = im_load(&mut opts, &mut rng);
        im_ops(arr, &mut opts, &mut rng);
    }
}

fn mem_ratio(mem: usize, n: usize) -> f64 {
    let s = 8;
    ((((mem as f64) / (n as f64)) - (s as f64)) / s as f64) * 100_f64
}

fn ppar_load(mut arr: ppar::Vector<u64>, opts: &mut Opt, rng: &mut SmallRng) -> ppar::Vector<u64> {
    let vals: Vec<u64> = (0..opts.load).map(|_| rng.gen()).collect();
    let offs: Vec<usize> = (0..opts.load)
        .map(|n| rng.gen::<usize>() % (n + 1))
        .collect();

    {
        let start = time::Instant::now();
        let arr = ppar::Vector::from_slice(&vals, opts.leaf_size);
        pp!(
            "append-load({} items)",
            arr.len()
            =>
            start.elapsed()
        );
    }

    {
        let (offs, vals) = (offs.clone(), vals.clone());
        let start = time::Instant::now();
        for (off, val) in offs.into_iter().zip(vals) {
            arr = arr.insert(off, val).unwrap();
        }
        pp!(
            "random-load({} items)",
            arr.len()
            =>
            start.elapsed() / (opts.load as u32)
        );
    }

    arr
}

fn ppar_ops(mut arr: ppar::Vector<u64>, opts: &Opt, rng: &mut SmallRng) -> ppar::Vector<u64> {
    let vals: Vec<u64> = (0..opts.ops).map(|_| rng.gen()).collect();
    let offs: Vec<usize> = (0..opts.ops)
        .map(|_| rng.gen::<usize>() % arr.len())
        .collect();

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        for off in offs.into_iter() {
            arr.get(off).unwrap();
        }
        pp!(
            "get({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        let vals = vals.clone();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            arr = arr.insert(off, val).unwrap();
        }
        pp!(
            "insert({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        let vals = vals.clone();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            arr = arr.set(off, val).unwrap();
        }
        pp!(
            "set({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        let vals = vals.clone();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            arr = arr.delete(off).unwrap();
            arr = arr.insert(off, val).unwrap();
        }
        pp!(
            "delete-insert({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    let ratio = format!("{:.2}%", mem_ratio(arr.footprint(), arr.len()));
    pp!("overhead" => ratio);

    arr
}

fn ppar_delete_skew(mut arr: ppar::Vector<u64>, rng: &mut SmallRng) -> ppar::Vector<u64> {
    let offs: Vec<usize> = (0..((arr.len() / 10) * 9))
        .map(|n| rng.gen::<usize>() % (arr.len() - n))
        .collect();

    for off in offs.into_iter() {
        arr = arr.delete(off).unwrap();
    }

    let ratio = format!("{:.2}%", mem_ratio(arr.footprint(), arr.len()));
    pp!("overhead after 90% delete" => ratio);

    arr
}

fn im_load(opts: &mut Opt, rng: &mut SmallRng) -> ImVector<u64> {
    let vals: Vec<u64> = (0..opts.load).map(|_| rng.gen()).collect();

    let arr = {
        let start = time::Instant::now();
        let arr: ImVector<u64> = ImVector::from(&vals);
        pp!(
            "append-load({} items)",
            arr.len()
            =>
            start.elapsed()
        );
        arr
    };

    arr
}

fn im_ops(mut arr: ImVector<u64>, opts: &Opt, rng: &mut SmallRng) -> ImVector<u64> {
    let vals: Vec<u64> = (0..opts.ops).map(|_| rng.gen()).collect();
    let offs: Vec<usize> = (0..opts.ops)
        .map(|_| rng.gen::<usize>() % arr.len())
        .collect();

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        for off in offs.into_iter() {
            arr.get(off).unwrap();
        }
        pp!(
            "get({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        let vals = vals.clone();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            arr = arr.update(off, val);
        }
        pp!(
            "update({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        let vals = vals.clone();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            arr.insert(off, val);
        }
        pp!(
            "insert({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    {
        let start = time::Instant::now();
        let offs = offs.clone();
        let vals = vals.clone();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            arr.remove(off);
            arr.insert(off, val);
        }
        pp!(
            "delete-insert({} ops)",
            opts.ops
            =>
            start.elapsed() / (opts.ops as u32)
        );
    }

    arr
}
