use super::*;

#[test]
fn area_masks_ignore_out_of_range_areas_instead_of_shifting_invalid_bits() {
    let mask = NavmeshAreaMask::from_area(NavmeshArea(63));
    assert!(mask.contains_area(NavmeshArea(63)));

    let out_of_range = NavmeshAreaMask::from_area(NavmeshArea(64));
    assert_eq!(out_of_range, NavmeshAreaMask::empty());
    assert!(!out_of_range.contains_area(NavmeshArea(64)));
}
