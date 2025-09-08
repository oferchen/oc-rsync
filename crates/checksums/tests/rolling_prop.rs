// crates/checksums/tests/rolling_prop.rs
use checksums::{Rolling, rolling_checksum_seeded};
use proptest::prelude::*;

proptest! {
    #[test]
    fn rolling_update_matches_naive(
        seed in any::<u32>(),
        data in prop::collection::vec(any::<u8>(), 2..512usize),
        window in 1usize..128usize,
    ) {
        prop_assume!(window < data.len());
        let mut roll = Rolling::with_seed(&data[..window], seed);
        let mut idx = 0;
        loop {
            let expected = rolling_checksum_seeded(&data[idx..idx + window], seed);
            prop_assert_eq!(roll.digest(), expected);
            if idx + window == data.len() {
                break;
            }
            let out = data[idx];
            let inp = data[idx + window];
            roll.roll(out, inp);
            idx += 1;
        }
    }
}
