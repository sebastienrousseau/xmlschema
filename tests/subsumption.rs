// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! *Particle Valid (Restriction)* — the subsumption relation.
//!
//! A restriction must accept nothing its base would reject. The
//! relation is defined case by case on the pair of terms, and each
//! case is exercised here on the pair that distinguishes it from its
//! neighbour.

use xmlschema::derive::{
    Compositor, Particle, Term, is_valid_restriction, namespace_subset,
};

/// Nothing derives from anything, unless it is the same name.
fn nothing(_: &str, _: &str) -> bool {
    false
}

fn element(name: &str, min: usize, max: Option<usize>) -> Particle {
    Particle {
        min,
        max,
        term: Term::Element {
            name: name.to_owned(),
            type_name: None,
        },
    }
}

fn typed(name: &str, type_name: &str) -> Particle {
    Particle {
        min: 1,
        max: Some(1),
        term: Term::Element {
            name: name.to_owned(),
            type_name: Some(type_name.to_owned()),
        },
    }
}

fn wildcard(ns: &str, min: usize, max: Option<usize>) -> Particle {
    Particle {
        min,
        max,
        term: Term::Wildcard(ns.to_owned()),
    }
}

fn group(
    compositor: Compositor,
    particles: Vec<Particle>,
    min: usize,
    max: Option<usize>,
) -> Particle {
    Particle {
        min,
        max,
        term: Term::Group {
            compositor,
            particles,
        },
    }
}

fn ok(derived: &Particle, base: &Particle) -> bool {
    is_valid_restriction(derived, base, &nothing)
}

#[test]
fn an_occurrence_range_must_lie_inside_the_base() {
    let base = element("a", 1, Some(5));
    assert!(ok(&element("a", 1, Some(5)), &base), "identical");
    assert!(ok(&element("a", 2, Some(4)), &base), "narrower");
    assert!(!ok(&element("a", 0, Some(5)), &base), "a lower minimum");
    assert!(!ok(&element("a", 1, Some(6)), &base), "a higher maximum");
    assert!(!ok(&element("a", 1, None), &base), "unbounded past a bound");

    // An unbounded base contains everything.
    let unbounded = element("a", 0, None);
    assert!(ok(&element("a", 5, Some(9)), &unbounded));
    assert!(ok(&element("a", 0, None), &unbounded));
}

#[test]
fn an_element_must_keep_its_name() {
    assert!(ok(&element("a", 1, Some(1)), &element("a", 1, Some(1))));
    assert!(!ok(&element("b", 1, Some(1)), &element("a", 1, Some(1))));
}

#[test]
fn a_base_with_no_named_type_accepts_any_type() {
    // The base is the ur-type, which everything derives from.
    assert!(ok(&typed("a", "MyType"), &element("a", 1, Some(1))));
    // A different named type cannot be established as derived, and
    // `nothing` says so.
    assert!(!ok(&typed("a", "X"), &typed("a", "Y")));
    // The same type is trivially fine.
    assert!(ok(&typed("a", "X"), &typed("a", "X")));
    // A real derivation relation is consulted when one is given.
    let type_derives = |d: &str, b: &str| d == "X" && b == "Y";
    assert!(is_valid_restriction(
        &typed("a", "X"),
        &typed("a", "Y"),
        &type_derives
    ));
}

#[test]
fn a_wildcard_may_only_narrow_its_namespaces() {
    let base = wildcard("urn:a urn:b", 1, Some(1));
    assert!(ok(&wildcard("urn:a", 1, Some(1)), &base));
    assert!(ok(&wildcard("urn:a urn:b", 1, Some(1)), &base));
    assert!(!ok(&wildcard("urn:c", 1, Some(1)), &base));
    assert!(!ok(&wildcard("##any", 1, Some(1)), &base));
    // Anything fits inside `##any`.
    assert!(ok(
        &wildcard("urn:a", 1, Some(1)),
        &wildcard("##any", 1, Some(1))
    ));

    assert!(namespace_subset("urn:a", "##any"));
    assert!(!namespace_subset("##any", "urn:a"));
    assert!(namespace_subset("urn:a urn:b", "urn:a urn:b urn:c"));
    assert!(!namespace_subset("urn:a urn:z", "urn:a urn:b"));
}

#[test]
fn a_wildcard_cannot_restrict_an_element() {
    // It admits more, which is the opposite of restricting.
    assert!(!ok(
        &wildcard("##any", 1, Some(1)),
        &element("a", 1, Some(1))
    ));
}

/// A group of one, occurring once, *is* the particle it contains.
#[test]
fn a_pointless_particle_is_eliminated() {
    let wrapped = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&wrapped, &element("a", 1, Some(1))));
    assert!(ok(&element("a", 1, Some(1)), &wrapped));

    // Nested wrappers collapse all the way down.
    let deep = group(Compositor::Choice, vec![wrapped.clone()], 1, Some(1));
    assert!(ok(&deep, &element("a", 1, Some(1))));

    // A group that occurs more than once is not pointless.
    let repeated = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1))],
        1,
        Some(3),
    );
    assert!(!ok(&repeated, &element("a", 1, Some(1))));
}

/// A group restricting a wildcard is decided on its *total* range,
/// not on each particle's.
#[test]
fn a_group_restricting_a_wildcard_counts_its_whole_content() {
    let base = wildcard("##any", 3, Some(3));
    let three = group(
        Compositor::All,
        vec![
            element("e1", 1, Some(1)),
            element("e2", 1, Some(1)),
            element("e3", 1, Some(1)),
        ],
        1,
        Some(1),
    );
    assert!(ok(&three, &base), "three elements match three wildcards");

    // Two do not reach the base's minimum of three.
    let two = group(
        Compositor::All,
        vec![element("e1", 1, Some(1)), element("e2", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&two, &base));

    // Namespace still applies to each particle.
    let narrow = wildcard("urn:a", 1, Some(3));
    let any_group = group(
        Compositor::Sequence,
        vec![wildcard("urn:b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&any_group, &narrow));
}

#[test]
fn a_sequence_restricting_a_sequence_keeps_its_order() {
    let base = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    let same = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&same, &base));

    let swapped = group(
        Compositor::Sequence,
        vec![element("b", 1, Some(1)), element("a", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&swapped, &base), "a sequence's order is required");
}

/// Anything the restriction drops must have been skippable.
#[test]
fn a_dropped_particle_must_have_been_optional() {
    let optional_b = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 0, Some(1))],
        1,
        Some(1),
    );
    let just_a = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&just_a, &optional_b), "b was optional");

    let required_b = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&just_a, &required_b), "b was required");
}

/// `xs:all` imposes no order, so a restriction of it need not keep one.
#[test]
fn restricting_an_all_group_ignores_order() {
    let base = group(
        Compositor::All,
        vec![element("e1", 1, Some(1)), element("e2", 1, Some(1))],
        1,
        Some(1),
    );
    let reversed = group(
        Compositor::Sequence,
        vec![element("e2", 1, Some(1)), element("e1", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&reversed, &base), "order is not the base's requirement");

    // But every required particle must still be there.
    let partial = group(
        Compositor::Sequence,
        vec![element("e2", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&partial, &base), "e1 was required");
}

#[test]
fn a_choice_may_drop_branches_but_not_add_them() {
    let base = group(
        Compositor::Choice,
        vec![
            element("a", 1, Some(1)),
            element("b", 1, Some(1)),
            element("c", 1, Some(1)),
        ],
        1,
        Some(1),
    );
    let fewer = group(
        Compositor::Choice,
        vec![element("a", 1, Some(1)), element("c", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&fewer, &base), "dropping a branch narrows the choice");

    let extra = group(
        Compositor::Choice,
        vec![element("a", 1, Some(1)), element("z", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&extra, &base), "z was never on offer");
}

/// A choice cannot restrict a sequence: it drops the requirement that
/// every particle appear.
#[test]
fn a_choice_cannot_restrict_a_sequence() {
    let base = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    let choice = group(
        Compositor::Choice,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&choice, &base));
}

#[test]
fn a_sequence_restricting_a_choice_maps_each_particle_to_a_branch() {
    let base = group(
        Compositor::Choice,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    let both = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&both, &base), "each particle is one of the branches");

    let stranger = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("z", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&stranger, &base));
}

#[test]
fn emptiability_follows_the_compositor() {
    // A sequence is emptiable when every particle is.
    let seq = group(
        Compositor::Sequence,
        vec![element("a", 0, Some(1)), element("b", 0, Some(1))],
        1,
        Some(1),
    );
    assert!(seq.emptiable());

    let seq_required = group(
        Compositor::Sequence,
        vec![element("a", 0, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!seq_required.emptiable());

    // A choice is emptiable when any branch is.
    let choice = group(
        Compositor::Choice,
        vec![element("a", 0, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(choice.emptiable());

    // And anything with a zero minimum is emptiable outright.
    assert!(element("a", 0, Some(1)).emptiable());
    assert!(!element("a", 1, Some(1)).emptiable());
}

#[test]
fn the_effective_total_range_multiplies_through() {
    // Two elements in a sequence occurring twice: four in total.
    let seq = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        2,
        Some(2),
    );
    assert_eq!(seq.effective_total_range(), (4, Some(4)));

    // A choice takes the least and greatest any branch offers.
    let choice = group(
        Compositor::Choice,
        vec![element("a", 1, Some(1)), element("b", 2, Some(3))],
        1,
        Some(1),
    );
    assert_eq!(choice.effective_total_range(), (1, Some(3)));

    // Unbounded anywhere makes the total unbounded.
    let open = group(
        Compositor::Sequence,
        vec![element("a", 1, None)],
        1,
        Some(1),
    );
    assert_eq!(open.effective_total_range(), (1, None));
}

/// An element restricting a choice takes any branch; restricting a
/// sequence requires the rest to be skippable.
#[test]
fn an_element_restricting_a_group_is_decided_by_the_compositor() {
    let choice = group(
        Compositor::Choice,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(
        ok(&element("a", 1, Some(1)), &choice),
        "one branch is enough"
    );
    assert!(!ok(&element("z", 1, Some(1)), &choice), "z is no branch");

    // A sequence whose other particles are optional.
    let optional = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 0, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&element("a", 1, Some(1)), &optional));

    // And one whose others are not.
    let required = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&element("a", 1, Some(1)), &required));
}

/// A group of one, occurring once, restricting an element unwraps to
/// that element; anything richer has no rule.
#[test]
fn a_wrapped_element_restricts_the_element_it_wraps() {
    let base = element("a", 1, Some(1));
    let wrapped = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&wrapped, &base));

    // Two particles is not a wrapper.
    let two = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("b", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&two, &base));
}

/// An unordered mapping uses each base particle at most once, and
/// leaves nothing required behind.
#[test]
fn an_unordered_mapping_consumes_each_particle_once() {
    let base = group(
        Compositor::All,
        vec![
            element("a", 1, Some(1)),
            element("b", 1, Some(1)),
            element("c", 0, Some(1)),
        ],
        1,
        Some(1),
    );
    // Both required particles present, in any order; `c` was optional.
    let reordered = group(
        Compositor::Sequence,
        vec![element("b", 1, Some(1)), element("a", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(ok(&reordered, &base));

    // `a` twice cannot map onto one `a`.
    let doubled = group(
        Compositor::Sequence,
        vec![element("a", 1, Some(1)), element("a", 1, Some(1))],
        1,
        Some(1),
    );
    assert!(!ok(&doubled, &base), "each base particle is used once");
}

/// A wildcard restricting a wildcard of a different namespace form.
#[test]
fn namespace_subsetting_covers_the_special_forms() {
    // `##other` against a list is left undecided, which means accepted.
    assert!(namespace_subset("##other", "urn:a"));
    assert!(namespace_subset("urn:a", "##other"));
    // Identical constraints are trivially a subset.
    assert!(namespace_subset("##other", "##other"));
    assert!(namespace_subset("##any", "##any"));
}
