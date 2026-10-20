//! Fixed-size sample history behind every live graph.

use std::collections::VecDeque;

/// Points kept per graph series (one per second).
pub const HISTORY_LEN: usize = 60;

/// Ring buffer of the most recent `cap` samples.
#[derive(Clone, Debug)]
pub struct History {
    cap: usize,
    buf: VecDeque<f64>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            buf: VecDeque::with_capacity(cap),
        }
    }

    pub fn push(&mut self, v: f64) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        self.buf.iter().copied()
    }

    pub fn last(&self) -> Option<f64> {
        self.buf.back().copied()
    }

    /// Largest visible value, or 0 when empty (rates and percentages are never negative).
    pub fn max(&self) -> f64 {
        self.values().fold(0.0, f64::max)
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(HISTORY_LEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_caps_at_60() {
        let mut h = History::new(60);
        for i in 0..61 {
            h.push(i as f64);
        }
        assert_eq!(h.len(), 60);
        assert_eq!(h.values().next(), Some(1.0));
        assert_eq!(h.last(), Some(60.0));
    }

    #[test]
    fn history_max_is_largest_value() {
        let mut h = History::new(3);
        assert_eq!(h.max(), 0.0);
        for v in [5.0, 9.0, 2.0] {
            h.push(v);
        }
        assert_eq!(h.max(), 9.0);
        h.push(1.0); // 5.0 drops out; 9.0 is still visible
        assert_eq!(h.max(), 9.0);
    }
}
