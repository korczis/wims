#[cfg(test)]
extern crate quickcheck;

const UNITS: [&str; 5] = ["", "K", "M", "G", "T"];

pub fn human_format(val: f32) -> (f32, &'static str) {
    let mut val = val;

    let mut i = 0;
    for item in UNITS.iter() {
        if val < 1024.0 {
            return (val, item);
        }

        val /= 1024.0;
        i += 1;
    }

    return (val, &UNITS[i]);
}


#[cfg(test)]
mod tests {
    fn reverse<T: Clone>(xs: &[T]) -> Vec<T> {
        let mut rev = vec![];
        for x in xs.iter() {
            rev.insert(0, x.clone())
        }
        rev
    }

    quickcheck! {
      fn prop(xs: Vec<u32>) -> bool {
          xs == reverse(&reverse(&xs))
      }
    }

    #[cfg(test)]
    extern crate test;

    #[bench]
    fn bench_year_flags_from_year(bh: &mut test::Bencher) {
        bh.iter(|| {
            for year in -999i32..1000 {
                true
            }
        });
    }

}
