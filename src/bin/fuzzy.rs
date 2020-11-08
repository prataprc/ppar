#[cfg(not(feature = "ppar-rc"))]
use im::Vector as ImVector;
#[cfg(feature = "ppar-rc")]
use im_rc::Vector as ImVector;
use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};
use structopt::StructOpt;

use std::{fmt, thread};

use ppar;

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

    #[structopt(long = "ops", default_value = "10000")]
    ops: usize,

    #[structopt(long = "threads", default_value = "4")]
    threads: usize,

    #[structopt(long = "rc", default_value = "false")]
    rc: bool,

    #[structopt(long = "leaf-size")]
    leaf_size: Option<usize>,
}

fn main() {
    use std::collections::BTreeMap;

    let mut opts = Opt::from_args();
    // opts.seed = Some(43412938081234232274443750093662763225);
    opts.seed = Some(opts.seed.unwrap_or(random()));
    println!("seed: {}", opts.seed.unwrap());

    let mut arr: ppar::Vector<u64> = ppar::Vector::new();
    opts.leaf_size.map(|s| arr.set_leaf_size(s));
    let (arr, im_arr) = test_load(arr, opts.clone());

    let mut handles = vec![];
    for i in 0..opts.threads {
        let (arr, im_arr) = (arr.clone(), im_arr.clone());
        let opts = opts.clone();
        handles.push(thread::spawn(move || test_ops(i, arr, im_arr, opts)));
    }

    let mut arrs = vec![];
    for h in handles.into_iter() {
        arrs.push(h.join().unwrap());
    }

    let mut map: BTreeMap<*const u8, u32> = BTreeMap::new();
    for (i, arr) in arrs.into_iter().enumerate() {
        let (ptrs, total_nodes) = arr.fetch_multiversions();
        for ptr in ptrs.clone().into_iter() {
            let n = map.get(&ptr).map(|x| *x).unwrap_or(0);
            map.insert(ptr, n + 1);
        }
        println!(
            "thread-{} number of multi-reference nodes {} / {}",
            i,
            ptrs.len(),
            total_nodes
        );
    }
    println!("total shared nodes {}", map.len());
}

fn test_load(mut arr: ppar::Vector<u64>, opts: Opt) -> (ppar::Vector<u64>, ImVector<u64>) {
    let mut rng = SmallRng::from_seed(opts.seed.unwrap().to_le_bytes());

    let prepend_load = opts.load / 2;
    let append_load = opts.load - prepend_load;

    let mut vals = vec![];
    for _i in 0..prepend_load {
        let val: u64 = rng.gen();
        arr.insert(0, val).unwrap();
        vals.push(val);
    }
    vals.reverse();

    for _ in 0..append_load {
        let val: u64 = rng.gen();
        arr.insert(arr.len(), val).unwrap();
        vals.push(val);
    }

    for _ in 0..opts.load {
        let off: usize = rng.gen::<usize>() % arr.len();
        let val: u64 = rng.gen();
        arr.set(off, val).unwrap();
        vals[off] = val;
    }

    let im_arr = ImVector::from(&vals);

    validate(&arr, &im_arr);
    println!("validated load {} items", arr.len());

    (arr, im_arr)
}

fn test_ops(
    n: usize,
    mut arr: ppar::Vector<u64>,
    mut im_arr: ImVector<u64>,
    opts: Opt,
) -> ppar::Vector<u64> {
    let mut rng = SmallRng::from_seed(opts.seed.unwrap().to_le_bytes());

    for _ in 0..opts.ops {
        let op: usize = rng.gen::<usize>() % 4;
        let off: usize = rng.gen::<usize>() % arr.len();
        match op {
            0 => {
                let val: u64 = rng.gen();
                // println!("off: {:10} {} op:{} thread:{}", off, val, op, n);
                arr.set(off, val).unwrap();
                im_arr = im_arr.update(off, val);
            }
            1 => {
                // println!("off: {:10} op:{} thread:{}", off, op, n);
                assert_eq!(
                    arr.get(off).unwrap(),
                    im_arr.get(off).unwrap(),
                    "off:{} thread:{}",
                    off,
                    n
                );
            }
            2 => {
                // println!("off: {:10} op:{} thread:{}", off, op, n);
                arr.remove(off).unwrap();
                im_arr.remove(off);
            }
            3 => {
                let val: u64 = rng.gen();
                // println!("off: {:10} {} op:{} thread:{}", off, val, op, n);
                arr.insert(off, val).unwrap();
                im_arr.insert(off, val);
            }
            _ => unreachable!(),
        }
    }

    validate(&arr, &im_arr);
    println!(
        "validated thread {} ops {} with {} items",
        n,
        opts.ops,
        arr.len()
    );

    arr
}

fn validate<T>(arr: &ppar::Vector<T>, im_arr: &ImVector<T>)
where
    T: fmt::Debug + fmt::Display + Clone + Eq + PartialEq,
{
    assert_eq!(im_arr.len(), arr.len());

    for (off, val) in im_arr.iter().enumerate() {
        assert_eq!(arr.get(off).unwrap(), val, "off-{}", off);
    }
}
