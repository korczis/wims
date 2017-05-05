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
