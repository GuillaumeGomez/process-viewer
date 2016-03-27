use gdk;

pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

fn convert(v: u8) -> f64 {
    v as f64 / 255.0
}

fn apply(i: isize) -> u8 {
    let mut value = i - 1;
    let mut v = 0;

    for _ in 0..8 {
        v = v | (value & 1);
        v = v << 1;
        value = value >> 1;
    }
    v = v >> 1;
    v as u8 & 255
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: convert(r),
            g: convert(g),
            b: convert(b),
        }
    }

    pub fn generate(index: usize) -> Color {
        let n = (index as f64).cbrt() as isize;
        let mut index = index as isize - (n * n * n);
        let mut p = &mut [n, n, n];

        if index == 0 {
            return Color::new(apply(p[0]), apply(p[1]), apply(p[2]));
        }
        index -= 1;
        let v = (index % 3) as usize;
        index = index / 3;

        if index < n {
            p[v] = index % n;
            return Color::new(apply(p[0]), apply(p[1]), apply(p[2]));
        }
        index -= n;
        p[v] = index / n;
        p[(v + 1) % 3] = index % n;
        Color::new(apply(p[0]), apply(p[1]), apply(p[2]))
    }

    /*pub fn to_int(&self) -> usize {
        0xFF << 24 | (self.r as usize) << 16 | (self.g as usize) << 8 | (self.b as usize)
    }*/

    pub fn to_gdk(&self) -> gdk::RGBA {
        gdk::RGBA {
            red: self.r,
            green: self.g,
            blue: self.b,
            alpha: 1.0,
        }
    }
}