//! Self-authored, non-sensitive input for the background-provider qualification.

pub struct PulseWindow {
    samples: Vec<u32>,
}

impl PulseWindow {
    pub fn new(samples: Vec<u32>) -> Self {
        Self { samples }
    }

    pub fn average(&self) -> Option<u32> {
        let total = self.samples.iter().copied().sum::<u32>();
        let count = u32::try_from(self.samples.len()).ok()?;
        (count != 0).then(|| total / count)
    }
}

#[cfg(test)]
mod tests {
    use super::PulseWindow;

    #[test]
    fn averages_a_bounded_window() {
        assert_eq!(PulseWindow::new(vec![3, 6, 9]).average(), Some(6));
    }
}
