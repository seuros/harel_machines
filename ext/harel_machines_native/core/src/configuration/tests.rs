use crate::Configuration;

#[test]
fn leaves_are_held_in_ascending_order_however_they_arrive() {
    let mut configuration = Configuration::<5>::new();
    configuration.insert(4);
    configuration.insert(0);
    configuration.insert(2);

    assert_eq!(configuration.as_slice(), &[0, 2, 4]);
    assert_eq!(configuration.len(), 3);
}

#[test]
fn inserting_a_leaf_twice_changes_nothing() {
    let mut configuration = Configuration::<3>::new();
    configuration.insert(1);
    configuration.insert(1);

    assert_eq!(configuration.as_slice(), &[1]);
}

#[test]
fn removing_closes_the_gap_and_removing_an_absent_leaf_does_nothing() {
    let mut configuration = Configuration::<4>::new();
    configuration.insert(1);
    configuration.insert(2);
    configuration.insert(3);

    configuration.remove(2);
    configuration.remove(2);

    assert_eq!(configuration.as_slice(), &[1, 3]);
    assert!(!configuration.contains(2));
}

#[test]
fn a_chart_that_has_not_been_entered_is_empty() {
    let configuration = Configuration::<2>::new();

    assert!(configuration.is_empty());
    assert_eq!(configuration.iter().count(), 0);
}

#[test]
fn two_configurations_holding_the_same_leaves_are_equal() {
    let mut left = Configuration::<4>::new();
    left.insert(3);
    left.insert(1);
    let mut right = Configuration::<4>::new();
    right.insert(1);
    right.insert(3);
    right.insert(2);
    right.remove(2);

    assert_eq!(left, right);
}
