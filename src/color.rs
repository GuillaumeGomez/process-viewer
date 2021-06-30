pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

fn convert(v: u8) -> f64 {
    f64::from(v) / 255.0
}

fn apply(i: isize) -> u8 {
    let mut value = i - 1;
    let mut v = 0;

    for _ in 0..8 {
        v |= value & 1;
        v <<= 1;
        value >>= 1;
    }
    v >>= 1;
    v as u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: convert(r),
            g: convert(g),
            b: convert(b),
        }
    }

    pub fn generate(index: usize) -> (Color, u8, u8, u8) {
        let n = (index as f64).cbrt() as isize;
        let mut index = index as isize - (n * n * n);
        let p = &mut [n, n, n];

        if index == 0 {
            let r = apply(p[0]);
            let g = apply(p[1]);
            let b = apply(p[2]);
            return (Color::new(r, g, b), r, g, b);
        }
        index -= 1;
        let v = (index % 3) as usize;
        index /= 3;

        if index < n {
            p[v] = index % n;
            let r = apply(p[0]);
            let g = apply(p[1]);
            let b = apply(p[2]);
            return (Color::new(r, g, b), r, g, b);
        }
        index -= n;
        p[v] = index / n;
        p[(v + 1) % 3] = index % n;
        let r = apply(p[0]);
        let g = apply(p[1]);
        let b = apply(p[2]);
        (Color::new(r, g, b), r, g, b)
    }
}
