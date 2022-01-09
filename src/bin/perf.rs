use rand::{prelude::random, rngs::StdRng, Rng, SeedableRng};
use structopt::StructOpt;

use std::{collections::BTreeMap, time};

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
    seed: Option<u64>,

    #[structopt(long = "loads", default_value = "1000000")] // default 1M
    loads: usize,

    #[structopt(long = "ops", default_value = "1000000")] // default 1M
    ops: usize,

    #[structopt(long = "im")]
    im: bool,

    #[structopt(long = "std-vec")]
    std_vec: bool,

    #[structopt(long = "leaf-size")]
    leaf_size: Option<usize>,
}

fn main() {
    use std::iter::repeat;

    let opts = Opt::from_args();
    let mut rng = {
        let seed = opts.seed.unwrap_or_else(random);
        StdRng::seed_from_u64(seed)
    };

    let arrs = if opts.im {
        vec![(Array::<u64>::new_im(), "im::Vector")]
    } else if opts.std_vec {
        vec![(Array::<u64>::new_vec(), "std::vec::Vec")]
    } else {
        let one = Array::new_vector(opts.leaf_size.unwrap_or(ppar::LEAF_CAP), true);
        let two = Array::new_vector_safe(opts.leaf_size.unwrap_or(ppar::LEAF_CAP), true);
        vec![(one, "ppar::rc::Vector"), (two, "ppar::arc::Vector")]
    };

    for (opts, (arr, log)) in repeat(opts).take(arrs.len()).zip(arrs.into_iter()) {
        let mut perf = Perf::new(arr, opts);
        println!("Performance report for {}", log);
        println!("--------------------------------------");
        perf.load(&mut rng);
        perf.run(&mut rng);
        perf.rebalance(true);
        perf.pretty_print();
        println!()
    }
}

fn mem_ratio<T>(mem: usize, n: usize) -> f64 {
    let s = std::mem::size_of::<T>();
    ((((mem as f64) / (n as f64)) - (s as f64)) / s as f64) * 100_f64
}

#[derive(Clone)]
enum Array<T>
where
    T: Clone,
{
    Vector(ppar::rc::Vector<T>),
    VectorSafe(ppar::arc::Vector<T>),
    Vec(Vec<T>),
    Im(im::Vector<T>),
}

impl<T> Array<T>
where
    T: Clone,
    rand::distributions::Standard: rand::distributions::Distribution<T>,
{
    fn new_vector(leaf_size: usize, auto_rebalance: bool) -> Self {
        let mut arr = ppar::rc::Vector::<T>::default();
        arr.set_leaf_size(leaf_size)
            .set_auto_rebalance(auto_rebalance);
        Array::Vector(arr)
    }

    fn new_vector_safe(leaf_size: usize, auto_rebalance: bool) -> Self {
        let mut arr = ppar::arc::Vector::<T>::default();
        arr.set_leaf_size(leaf_size)
            .set_auto_rebalance(auto_rebalance);
        Array::VectorSafe(arr)
    }

    fn new_vec() -> Self {
        Array::<T>::Vec(vec![])
    }

    fn new_im() -> Self {
        Array::Im(im::Vector::<T>::new())
    }

    #[allow(clippy::needless_collect)]
    fn load(&mut self, n: usize, rng: &mut StdRng) -> (time::Duration, usize) {
        let offs: Vec<usize> = (1..=n).map(|i| rng.gen::<usize>() % i).collect();
        let vals: Vec<T> = (0..n).map(|_| rng.gen::<T>()).collect();

        let start = time::Instant::now();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            match self {
                Array::Vector(arr) => arr.insert(off, val).unwrap(),
                Array::VectorSafe(arr) => arr.insert(off, val).unwrap(),
                Array::Vec(arr) => arr.insert(off, val),
                Array::Im(arr) => arr.insert(off, val),
            }
        }
        let elapsed = start.elapsed();

        (elapsed, n)
    }

    fn len(&self) -> usize {
        match self {
            Array::Vector(arr) => arr.len(),
            Array::VectorSafe(arr) => arr.len(),
            Array::Vec(arr) => arr.len(),
            Array::Im(arr) => arr.len(),
        }
    }

    fn rebalance(&self, packed: bool) -> Option<Self> {
        match self {
            Array::Vector(arr) => Some(Array::Vector(arr.rebalance(packed).unwrap())),
            Array::VectorSafe(arr) => {
                Some(Array::VectorSafe(arr.rebalance(packed).unwrap()))
            }
            Array::Vec(_) => None,
            Array::Im(_) => None,
        }
    }
}

struct Perf<T>
where
    T: Clone,
{
    opts: Opt,
    val: Array<T>,
    stats: BTreeMap<&'static str, (time::Duration, usize)>,
}

impl<T> Perf<T>
where
    T: Clone,
    rand::distributions::Standard: rand::distributions::Distribution<T>,
{
    fn new(val: Array<T>, opts: Opt) -> Self {
        let stats = BTreeMap::new();
        Perf { opts, val, stats }
    }

    fn len(&self) -> usize {
        match &self.val {
            Array::Vector(val) => val.len(),
            Array::VectorSafe(val) => val.len(),
            Array::Vec(val) => val.len(),
            Array::Im(val) => val.len(),
        }
    }

    fn rebalance(&mut self, packed: bool) {
        if let Some(val) = self.val.rebalance(packed) {
            self.val = val
        }
    }

    fn load(&mut self, rng: &mut StdRng) {
        self.stats
            .insert("load", self.val.load(self.opts.loads, rng));
    }

    fn run(&mut self, rng: &mut StdRng) {
        self.op_clone(self.opts.ops);
        self.op_insert(self.opts.ops, rng);
        self.op_insert_mut(self.opts.ops, rng);
        self.op_remove(self.opts.ops, rng);
        self.op_remove_mut(self.opts.ops, rng);
        self.op_update(self.opts.ops, rng);
        self.op_update_mut(self.opts.ops, rng);
        self.op_get(self.opts.ops, rng);
        self.op_iter(self.opts.ops);
        self.op_split_append(self.opts.ops, rng);
    }

    fn pretty_print(&self) {
        for (k, (elapsed, n)) in self.stats.iter() {
            println!("{:14} {:?}", k, *elapsed / (*n as u32));
        }
        let fp = match &self.val {
            Array::Vector(val) => Some((val.footprint(), val.len())),
            Array::VectorSafe(val) => Some((val.footprint(), val.len())),
            _ => None,
        };
        if let Some((mem, n)) = fp {
            let ratio = mem_ratio::<T>(mem, n);
            println!("{:14} {}% {:?}", "mem-ratio", ratio, (mem, n));
        }
    }

    fn op_clone(&mut self, n_ops: usize) -> usize {
        let start = time::Instant::now();
        let mut acc = vec![];
        for _i in 0..n_ops {
            acc.push(self.val.clone().len());
        }
        let elapsed = start.elapsed();

        self.stats.insert("clone", (elapsed, n_ops));
        acc.len()
    }

    #[allow(clippy::needless_collect)]
    fn op_insert(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs: Vec<usize> = (0..n_ops).map(|_| rng.gen::<usize>() % len).collect();
        let vals: Vec<T> = (0..n_ops).map(|_| rng.gen::<T>()).collect();

        let start = time::Instant::now();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            match &mut self.val {
                Array::Vector(arr) => arr.insert(off, val).unwrap(),
                Array::VectorSafe(arr) => arr.insert(off, val).unwrap(),
                Array::Vec(arr) => arr.insert(off, val),
                Array::Im(arr) => arr.insert(off, val),
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("insert", (elapsed, n_ops));
    }

    #[allow(clippy::needless_collect)]
    fn op_insert_mut(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs: Vec<usize> = (0..n_ops).map(|_| rng.gen::<usize>() % len).collect();
        let vals: Vec<T> = (0..n_ops).map(|_| rng.gen::<T>()).collect();

        let start = time::Instant::now();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            match &mut self.val {
                Array::Vector(arr) => arr.insert_mut(off, val).unwrap(),
                Array::VectorSafe(arr) => arr.insert_mut(off, val).unwrap(),
                Array::Vec(arr) => arr.insert(off, val),
                Array::Im(arr) => arr.insert(off, val),
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("insert_mut", (elapsed, n_ops));
    }

    #[allow(clippy::needless_collect)]
    fn op_remove(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs: Vec<usize> = (0..n_ops).map(|_| rng.gen::<usize>() % len).collect();
        let vals: Vec<T> = (0..n_ops).map(|_| rng.gen::<T>()).collect();

        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            match &mut self.val {
                Array::Vector(arr) => arr.insert(off, val).unwrap(),
                Array::VectorSafe(arr) => arr.insert(off, val).unwrap(),
                Array::Vec(arr) => arr.insert(off, val),
                Array::Im(arr) => arr.insert(off, val),
            };
        }

        let len = self.len();
        let offs = (0..n_ops).map(|i| rng.gen::<usize>() % (len - i));

        let start = time::Instant::now();
        for off in offs {
            match &mut self.val {
                Array::Vector(arr) => arr.remove(off).unwrap(),
                Array::VectorSafe(arr) => arr.remove(off).unwrap(),
                Array::Vec(arr) => arr.remove(off),
                Array::Im(arr) => arr.remove(off),
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("remove", (elapsed, n_ops));
    }

    #[allow(clippy::needless_collect)]
    fn op_remove_mut(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs: Vec<usize> = (0..n_ops).map(|_| rng.gen::<usize>() % len).collect();
        let vals: Vec<T> = (0..n_ops).map(|_| rng.gen::<T>()).collect();

        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            match &mut self.val {
                Array::Vector(arr) => arr.insert(off, val).unwrap(),
                Array::VectorSafe(arr) => arr.insert(off, val).unwrap(),
                Array::Vec(arr) => arr.insert(off, val),
                Array::Im(arr) => arr.insert(off, val),
            };
        }

        let len = self.len();
        let offs = (0..n_ops).map(|i| rng.gen::<usize>() % (len - i));

        let start = time::Instant::now();
        for off in offs {
            match &mut self.val {
                Array::Vector(arr) => {
                    arr.remove_mut(off).unwrap();
                }
                Array::VectorSafe(arr) => {
                    arr.remove_mut(off).unwrap();
                }
                Array::Vec(arr) => {
                    arr.remove(off);
                }
                Array::Im(arr) => {
                    arr.remove(off);
                }
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("remove_mut", (elapsed, n_ops));
    }

    #[allow(clippy::needless_collect)]
    fn op_update(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs: Vec<usize> = (0..n_ops).map(|_| rng.gen::<usize>() % len).collect();
        let vals: Vec<T> = (0..n_ops).map(|_| rng.gen::<T>()).collect();

        let start = time::Instant::now();
        for (off, val) in offs.into_iter().zip(vals.into_iter()) {
            match &mut self.val {
                Array::Vector(arr) => {
                    arr.update(off, val).unwrap();
                }
                Array::VectorSafe(arr) => {
                    arr.update(off, val).unwrap();
                }
                Array::Vec(arr) => arr[off] = val,
                Array::Im(arr) => arr[off] = val,
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("update", (elapsed, n_ops));
    }

    #[allow(clippy::needless_collect)]
    fn op_update_mut(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs: Vec<usize> = (0..n_ops).map(|_| rng.gen::<usize>() % len).collect();
        let vals = (0..n_ops).map(|_| rng.gen::<T>());

        let start = time::Instant::now();
        for (off, val) in offs.into_iter().zip(vals) {
            match &mut self.val {
                Array::Vector(arr) => {
                    arr.update_mut(off, val).unwrap();
                }
                Array::VectorSafe(arr) => {
                    arr.update_mut(off, val).unwrap();
                }
                Array::Vec(arr) => arr[off] = val,
                Array::Im(arr) => arr[off] = val,
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("update_mut", (elapsed, n_ops));
    }

    fn op_get(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs = (0..n_ops).map(|_| rng.gen::<usize>() % len);

        let start = time::Instant::now();
        for off in offs {
            match &mut self.val {
                Array::Vector(val) => val.get(off).unwrap(),
                Array::VectorSafe(val) => val.get(off).unwrap(),
                Array::Vec(val) => val.get(off).unwrap(),
                Array::Im(val) => val.get(off).unwrap(),
            };
        }
        let elapsed = start.elapsed();

        self.stats.insert("get", (elapsed, n_ops));
    }

    fn op_iter(&mut self, n_ops: usize) -> usize {
        let start = time::Instant::now();
        let mut count = 0_usize;
        for _i in 0..n_ops {
            let v: Vec<&T> = match &mut self.val {
                Array::Vector(val) => val.iter().collect(),
                Array::VectorSafe(val) => val.iter().collect(),
                Array::Vec(val) => val.iter().collect(),
                Array::Im(val) => val.iter().collect(),
            };
            count += v.len();
        }
        let elapsed = start.elapsed();

        self.stats.insert("iter", (elapsed, count));
        count
    }

    fn op_split_append(&mut self, n_ops: usize, rng: &mut StdRng) {
        let len = self.len();
        let offs = (0..n_ops).map(|_| rng.gen::<usize>() % len);

        let mut split_off_dur = time::Duration::default();
        let mut append_dur = time::Duration::default();

        for off in offs {
            match &mut self.val {
                Array::Vector(val) => {
                    let start = time::Instant::now();
                    let a = val.split_off(off).unwrap();
                    split_off_dur += start.elapsed();

                    let start = time::Instant::now();
                    val.append(a);
                    append_dur += start.elapsed();

                    *val = val.rebalance(true).unwrap();
                }
                Array::VectorSafe(val) => {
                    let start = time::Instant::now();
                    let a = val.split_off(off).unwrap();
                    split_off_dur += start.elapsed();

                    let start = time::Instant::now();
                    val.append(a);
                    append_dur += start.elapsed();

                    *val = val.rebalance(true).unwrap();
                }
                Array::Vec(val) => {
                    let start = time::Instant::now();
                    let mut a = val.split_off(off);
                    split_off_dur += start.elapsed();

                    let start = time::Instant::now();
                    val.append(&mut a);
                    append_dur += start.elapsed();
                }
                Array::Im(val) => {
                    let start = time::Instant::now();
                    let a = val.split_off(off);
                    split_off_dur += start.elapsed();

                    let start = time::Instant::now();
                    val.append(a);
                    append_dur += start.elapsed();
                }
            }
        }

        self.stats.insert("split_off", (split_off_dur, n_ops));
        self.stats.insert("append", (append_dur, n_ops));
    }
}
