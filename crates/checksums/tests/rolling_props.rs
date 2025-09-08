// crates/checksums/tests/rolling_props.rs
use checksums::{Rolling, rolling_checksum_seeded};
use proptest::prelude::*;

proptest! {
    #[test]
    fn rolling_round_trip(
        mut block in proptest::collection::vec(any::<u8>(), 1..65),
        seed in any::<u32>(),
        inp in any::<u8>(),
    ) {
        let mut r = Rolling::with_seed(&block, seed);
        let out = block[0];
        block.remove(0);
        block.push(inp);
        r.roll(out, inp);
        prop_assert_eq!(r.digest(), rolling_checksum_seeded(&block, seed));
    }
}
