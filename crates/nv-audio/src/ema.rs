//! Exponential moving averages, scalar and per-band.

/// `state = factor * state + (1 - factor) * x`, seeded by the first value
/// so a reading never has to climb out of zero.
#[derive(Debug, Clone)]
pub struct Ema {
    pub factor: f64,
    state: Option<f64>,
}

impl Ema {
    pub fn new(factor: f64) -> Self {
        Self { factor, state: None }
    }

    pub fn push(&mut self, x: f64) -> f64 {
        let next = match self.state {
            None => x,
            Some(s) => self.factor * s + (1.0 - self.factor) * x,
        };
        self.state = Some(next);
        next
    }
}

/// The same, over a fixed-width array of bands.
#[derive(Debug, Clone)]
pub struct EmaN<const N: usize> {
    pub factor: f64,
    state: Option<[f64; N]>,
}

impl<const N: usize> EmaN<N> {
    pub fn new(factor: f64) -> Self {
        Self { factor, state: None }
    }

    pub fn push(&mut self, x: &[f64; N]) -> [f64; N] {
        let next = match self.state {
            None => *x,
            Some(s) => {
                let mut out = [0.0; N];
                for i in 0..N {
                    out[i] = self.factor * s[i] + (1.0 - self.factor) * x[i];
                }
                out
            }
        };
        self.state = Some(next);
        next
    }
}
