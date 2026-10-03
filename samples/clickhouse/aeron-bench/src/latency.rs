use hdrhistogram::Histogram;

pub fn record(histogram: &mut Histogram<u64>, value: u64, interval: u64) {
    let value = value.max(1);
    if interval == 0 || value / interval < 10_000 {
        let _ = histogram.record_correct(value, interval);
        return;
    }
    // Group the descending correction series by HDR bucket. At high rates
    // a single stall can represent billions of samples; this records every
    // one of them without a billion-step loop or an arbitrary sample cap.
    histogram.saturating_record(value);
    let mut missing = value - interval;
    while missing >= interval {
        let lower = histogram.lowest_equivalent(missing).max(interval);
        let count = (missing - lower) / interval + 1;
        histogram.saturating_record_n(missing, count);
        missing -= count * interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correction_matches_hdr_across_bucket_boundaries() -> Result<(), Box<dyn std::error::Error>> {
        for interval in [0, 1, 7, 1000] {
            let mut actual = Histogram::new_with_bounds(1, 60_000_000_000, 3)?;
            let mut expected = actual.clone();
            for value in [1, 7, 1000, 8193, 160_003] {
                record(&mut actual, value, interval);
                expected.record_correct(value, interval)?;
            }
            assert_eq!(actual, expected);
        }
        Ok(())
    }
}
