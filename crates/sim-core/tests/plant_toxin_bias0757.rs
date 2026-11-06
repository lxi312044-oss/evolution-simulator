use sim_core::toxin::{founder_plant_toxin_type_biased, ToxinTypeId};
#[test]
fn biased_founders_and_legacy_compatibility() {
    for i in 0..1000 {
        assert_eq!(
            founder_plant_toxin_type_biased(123, i, true, 2, 0.5),
            sim_core::toxin::founder_plant_toxin_type(123, i, true, 2)
        );
        assert_eq!(
            founder_plant_toxin_type_biased(123, i, true, 2, 1.0),
            ToxinTypeId(0)
        );
        assert_eq!(
            founder_plant_toxin_type_biased(123, i, true, 2, 0.0),
            ToxinTypeId(1)
        );
        assert_eq!(
            founder_plant_toxin_type_biased(123, i, false, 2, 0.0),
            ToxinTypeId(0)
        );
    }
    let a = (0..10000)
        .filter(|&i| founder_plant_toxin_type_biased(123, i, true, 2, 0.8) == ToxinTypeId(0))
        .count();
    let b = (0..10000)
        .filter(|&i| founder_plant_toxin_type_biased(123, i, true, 2, 0.2) == ToxinTypeId(0))
        .count();
    assert!((7500..8500).contains(&a));
    assert!((1500..2500).contains(&b));
}
