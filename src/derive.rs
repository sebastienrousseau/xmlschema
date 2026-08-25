// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 xmlschema. All rights reserved.

//! Whether one content model is a valid restriction of another.
//!
//! XSD calls this *Particle Valid (Restriction)*, and it is the
//! hardest constraint in the specification: not a structural rule but
//! a **subsumption relation** between whole content models. A
//! restriction must accept no document its base would reject, and
//! deciding that means relating every particle of the derived model to
//! one of the base's.
//!
//! The relation is defined case by case, on the pair of terms:
//!
//! | derived | base | rule |
//! |---|---|---|
//! | element | element | names match, occurrences narrow, type derives |
//! | element | wildcard | the base admits the element's namespace |
//! | wildcard | wildcard | the base admits every namespace the derived does |
//! | group | wildcard | every particle restricts the wildcard |
//! | sequence | sequence | order-preserving map; unmapped base particles emptiable |
//! | sequence | all | as above, unordered |
//! | choice | choice | order-preserving map, unmapped base particles need not be emptiable |
//! | sequence | choice | every particle restricts some branch |
//!
//! **Where a case cannot be decided, it is accepted.** Type derivation
//! between named types is the main one: establishing it needs the
//! whole type hierarchy, and guessing risks rejecting a schema that is
//! valid. Accepting leaves a test counted as a miss rather than as a
//! wrong answer, which is the direction this crate errs in
//! everywhere.

use oxml::{Document, NodeId};

/// Which compositor a model group uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    /// `xs:sequence` — in order.
    Sequence,
    /// `xs:choice` — one of.
    Choice,
    /// `xs:all` — any order, each at most once.
    All,
}

/// What a particle stands for.
#[derive(Debug, Clone)]
pub enum Term {
    /// An element declaration.
    Element {
        /// Its name.
        name: String,
        /// The type it names, if it names one rather than nesting it.
        type_name: Option<String>,
    },
    /// An `xs:any` wildcard, with its namespace constraint as written.
    Wildcard(String),
    /// A nested model group.
    Group {
        /// How its particles combine.
        compositor: Compositor,
        /// The particles themselves.
        particles: Vec<Particle>,
    },
}

/// A term with an occurrence range.
#[derive(Debug, Clone)]
pub struct Particle {
    /// Minimum occurrences.
    pub min: usize,
    /// Maximum occurrences, or `None` for unbounded.
    pub max: Option<usize>,
    /// What occurs.
    pub term: Term,
}

impl Particle {
    /// A group of one, occurring exactly once, is the particle it
    /// contains.
    ///
    /// The specification calls this eliminating a *pointless
    /// particle*, and it has to happen before the cases are matched:
    /// `<sequence><any/></sequence>` and a bare `<any/>` are the same
    /// content model, and only one of them has a rule against a group.
    #[must_use]
    pub fn collapsed(&self) -> &Self {
        match &self.term {
            Term::Group { particles, .. }
                if particles.len() == 1
                    && self.min == 1
                    && self.max == Some(1) =>
            {
                particles[0].collapsed()
            }
            _ => self,
        }
    }

    /// How many times this particle can match in total, counting what
    /// is inside it.
    ///
    /// The specification calls this the *effective total range*. A
    /// group of three elements occurring once matches three times, and
    /// comparing that against a wildcard's own range is how a group
    /// restricting a wildcard is decided.
    #[must_use]
    pub fn effective_total_range(&self) -> (usize, Option<usize>) {
        let (min, max) = match &self.term {
            Term::Element { .. } | Term::Wildcard(_) => (1, Some(1)),
            Term::Group {
                compositor,
                particles,
            } => {
                let inner: Vec<(usize, Option<usize>)> = particles
                    .iter()
                    .map(Particle::effective_total_range)
                    .collect();
                let maxes = |combine: fn(usize, usize) -> usize| {
                    inner.iter().try_fold(0usize, |acc, (_, m)| {
                        m.map(|m| combine(acc, m))
                    })
                };
                match compositor {
                    // One branch is taken, so the least and greatest
                    // any branch offers.
                    Compositor::Choice => (
                        inner.iter().map(|(m, _)| *m).min().unwrap_or(0),
                        maxes(usize::max),
                    ),
                    // Every particle contributes.
                    Compositor::Sequence | Compositor::All => (
                        inner.iter().map(|(m, _)| *m).sum(),
                        maxes(usize::saturating_add),
                    ),
                }
            }
        };
        (
            min.saturating_mul(self.min),
            match (max, self.max) {
                (Some(a), Some(b)) => Some(a.saturating_mul(b)),
                _ => None,
            },
        )
    }

    /// Whether this particle can match nothing at all.
    ///
    /// Needed because a restriction may leave a base particle
    /// unmapped, but only one the base could have skipped anyway.
    #[must_use]
    pub fn emptiable(&self) -> bool {
        if self.min == 0 {
            return true;
        }
        match &self.term {
            Term::Element { .. } | Term::Wildcard(_) => false,
            // A sequence is emptiable when every particle is; a choice
            // when any is; `all` behaves as a sequence here.
            Term::Group {
                compositor,
                particles,
            } => match compositor {
                Compositor::Choice => {
                    particles.is_empty()
                        || particles.iter().any(Particle::emptiable)
                }
                Compositor::Sequence | Compositor::All => {
                    particles.iter().all(Particle::emptiable)
                }
            },
        }
    }
}

/// Whether `derived` accepts nothing `base` would reject.
///
/// `resolve` answers whether one type name is validly derived from
/// another; it is consulted only for the element-to-element case.
#[must_use]
pub fn is_valid_restriction(
    derived: &Particle,
    base: &Particle,
    type_derives: &dyn Fn(&str, &str) -> bool,
) -> bool {
    // Pointless particles are eliminated first, so a group of one is
    // matched as the thing it wraps.
    let derived = derived.collapsed();
    let base = base.collapsed();

    match (&derived.term, &base.term) {
        // Elt:Elt:NameAndTypeOK
        (
            Term::Element {
                name: r,
                type_name: rt,
            },
            Term::Element {
                name: b,
                type_name: bt,
            },
        ) => {
            r == b
                && occurrence_ok(derived, base)
                && match (rt.as_deref(), bt.as_deref()) {
                    // Two named types: the relation decides.
                    (Some(r), Some(b)) => r == b || type_derives(r, b),
                    // A base with no named type is the ur-type, which
                    // everything derives from; and a derived type
                    // given inline cannot have its derivation
                    // established without the whole hierarchy. Both
                    // are accepted rather than guessed at.
                    _ => true,
                }
        }

        // Elt:Any:NSCompat -- an element restricting a wildcard. The
        // element has no namespace of its own in this crate's model,
        // so only the occurrence range is decided here.
        (Term::Element { .. }, Term::Wildcard(_)) => {
            occurrence_ok(derived, base)
        }

        // Any:Any:NSSubset
        (Term::Wildcard(r), Term::Wildcard(b)) => {
            occurrence_ok(derived, base) && namespace_subset(r, b)
        }

        // NSRecurseCheckCardinality -- a group restricting a
        // wildcard. Each particle is checked against the wildcard's
        // *term* rather than its occurrence range, and the range is
        // compared once against the group's effective total range.
        // Comparing each particle against the range instead rejects
        // three elements restricting `<any minOccurs="3"/>`, which is
        // exactly what a valid restriction of it looks like.
        (Term::Group { particles, .. }, Term::Wildcard(namespace)) => {
            let unbounded = Particle {
                min: 0,
                max: None,
                term: Term::Wildcard(namespace.clone()),
            };
            if !particles
                .iter()
                .all(|p| is_valid_restriction(p, &unbounded, type_derives))
            {
                return false;
            }
            let (min, max) = derived.effective_total_range();
            occurrence_ok(
                &Particle {
                    min,
                    max,
                    term: Term::Wildcard(String::new()),
                },
                base,
            )
        }

        // A wildcard cannot restrict an element or a group: it admits
        // more than either.
        (Term::Wildcard(_), Term::Element { .. } | Term::Group { .. }) => false,

        // A group restricting an element. The specification has no
        // case for this, but it eliminates *pointless particles*
        // first: a group of one, occurring exactly once, is the
        // particle it contains. Anything richer genuinely has no rule.
        (
            Term::Group {
                particles,
                compositor: _,
            },
            Term::Element { .. },
        ) => match particles.as_slice() {
            [only] if derived.min == 1 && derived.max == Some(1) => {
                is_valid_restriction(only, base, type_derives)
            }
            _ => false,
        },

        // An element restricting a group. Wrapping the element in a
        // group and recursing would be neat and does not terminate:
        // collapsing immediately unwraps it again. So the two cases
        // are written out.
        (
            Term::Element { .. },
            Term::Group {
                compositor,
                particles,
            },
        ) => {
            match compositor {
                // One branch is taken, so restricting any branch is
                // enough.
                Compositor::Choice => particles
                    .iter()
                    .any(|b| is_valid_restriction(derived, b, type_derives)),
                // Every other particle has to be skippable, or the
                // base required something the restriction dropped.
                Compositor::Sequence | Compositor::All => {
                    particles.iter().enumerate().any(|(i, b)| {
                        is_valid_restriction(derived, b, type_derives)
                            && particles
                                .iter()
                                .enumerate()
                                .all(|(j, other)| j == i || other.emptiable())
                    })
                }
            }
        }

        (
            Term::Group {
                compositor: rc,
                particles: rp,
            },
            Term::Group {
                compositor: bc,
                particles: bp,
            },
        ) => {
            occurrence_ok(derived, base)
                && groups_match(*rc, rp, *bc, bp, type_derives)
        }
    }
}

/// One model group restricting another, by compositor pair.
///
/// Split out of [`is_valid_restriction`] because the eight cases read
/// as a table and do not belong inside a match on term kinds.
fn groups_match(
    rc: Compositor,
    rp: &[Particle],
    bc: Compositor,
    bp: &[Particle],
    type_derives: &dyn Fn(&str, &str) -> bool,
) -> bool {
    match (rc, bc) {
        // RecurseUnordered / RecurseAsIfGroup: `all` imposes no
        // order, so neither does a restriction of it. This case has
        // to come first -- a combined `(Sequence | All, Sequence |
        // All)` arm swallows it, and then a sequence restricting an
        // `all` in a different order is rejected for an ordering the
        // base never required.
        (Compositor::Sequence | Compositor::All, Compositor::All) => {
            map_unordered(rp, bp, type_derives)
        }
        // Recurse: order is preserved, and anything the restriction
        // steps over must have been skippable.
        (Compositor::Sequence | Compositor::All, Compositor::Sequence) => {
            map_in_order(rp, bp, true, type_derives)
        }
        // RecurseLax: the base was free to skip any branch, so an
        // unmapped one need not be emptiable.
        (Compositor::Choice, Compositor::Choice) => {
            map_in_order(rp, bp, false, type_derives)
        }
        // MapAndSum: every particle restricts some branch.
        (Compositor::Sequence | Compositor::All, Compositor::Choice) => {
            rp.iter().all(|r| {
                bp.iter().any(|b| is_valid_restriction(r, b, type_derives))
            })
        }
        // A choice cannot restrict a sequence or an `all`: it drops
        // the requirement that every particle appear.
        (Compositor::Choice, _) => false,
    }
}

/// Occurrence Range OK: the derived range lies inside the base's.
fn occurrence_ok(derived: &Particle, base: &Particle) -> bool {
    if derived.min < base.min {
        return false;
    }
    match (derived.max, base.max) {
        // An unbounded base contains everything.
        (_, None) => true,
        // An unbounded derived does not fit a bounded base.
        (None, Some(_)) => false,
        (Some(r), Some(b)) => r <= b,
    }
}

/// Map every derived particle onto a distinct base particle, keeping
/// order.
///
/// `skipped_must_be_emptiable` distinguishes a sequence, where a base
/// particle the derivation drops must have been optional, from a
/// choice, where the base was free to skip any branch anyway.
fn map_in_order(
    derived: &[Particle],
    base: &[Particle],
    skipped_must_be_emptiable: bool,
    type_derives: &dyn Fn(&str, &str) -> bool,
) -> bool {
    let mut at = 0usize;
    for r in derived {
        let mut found = None;
        for (offset, b) in base[at..].iter().enumerate() {
            if is_valid_restriction(r, b, type_derives) {
                found = Some(at + offset);
                break;
            }
            // Anything stepped over must have been skippable.
            if skipped_must_be_emptiable && !b.emptiable() {
                return false;
            }
        }
        let Some(index) = found else {
            return false;
        };
        at = index + 1;
    }
    // Whatever is left must be skippable too.
    !skipped_must_be_emptiable || base[at..].iter().all(Particle::emptiable)
}

/// Map every derived particle onto a distinct base particle, in any
/// order, each used at most once.
fn map_unordered(
    derived: &[Particle],
    base: &[Particle],
    type_derives: &dyn Fn(&str, &str) -> bool,
) -> bool {
    let mut used = vec![false; base.len()];
    for r in derived {
        let Some(index) = base.iter().enumerate().position(|(i, b)| {
            !used[i] && is_valid_restriction(r, b, type_derives)
        }) else {
            return false;
        };
        used[index] = true;
    }
    base.iter()
        .zip(&used)
        .all(|(b, taken)| *taken || b.emptiable())
}

/// Whether `derived` admits no namespace `base` excludes.
#[must_use]
pub fn namespace_subset(derived: &str, base: &str) -> bool {
    if base == "##any" {
        return true;
    }
    if derived == "##any" {
        return false;
    }
    if base == derived {
        return true;
    }
    // `##other` is defined against a target namespace this function
    // does not have, so a list against it is left undecided -- and
    // undecided means accepted.
    if base == "##other" || derived == "##other" {
        return true;
    }
    let allowed: Vec<&str> = base.split_whitespace().collect();
    derived.split_whitespace().all(|n| allowed.contains(&n))
}

/// Read a model group out of the schema document.
///
/// Returns `None` for a host with no content model, and follows a
/// `group` reference through `groups`.
#[must_use]
pub fn particle_of(
    doc: &Document,
    host: NodeId,
    groups: &dyn Fn(&str) -> Option<NodeId>,
    depth: usize,
) -> Option<Particle> {
    if depth > 32 {
        return None;
    }
    let local = |id: NodeId| doc.element_name(id).map(|n| n.local.clone());
    let name = local(host)?;

    let compositor = match name.as_str() {
        "sequence" => Compositor::Sequence,
        "choice" => Compositor::Choice,
        "all" => Compositor::All,
        "group" => {
            // A reference stands for what it names, with this
            // occurrence's own range.
            let target = doc
                .attribute(host, "ref")
                .and_then(|r| groups(r.rsplit(':').next().unwrap_or(r)))?;
            let inner =
                ["sequence", "choice", "all"].into_iter().find_map(|g| {
                    doc.children(target)
                        .iter()
                        .copied()
                        .find(|&c| local(c).is_some_and(|n| n == g))
                })?;
            let mut particle = particle_of(doc, inner, groups, depth + 1)?;
            particle.min = min_occurs(doc, host);
            particle.max = max_occurs(doc, host);
            return Some(particle);
        }
        "element" => {
            return Some(Particle {
                min: min_occurs(doc, host),
                max: max_occurs(doc, host),
                term: Term::Element {
                    name: doc
                        .attribute(host, "name")
                        .or_else(|| doc.attribute(host, "ref"))
                        .unwrap_or_default()
                        .rsplit(':')
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                    type_name: doc
                        .attribute(host, "type")
                        .map(|t| t.rsplit(':').next().unwrap_or(t).to_owned()),
                },
            });
        }
        "any" => {
            return Some(Particle {
                min: min_occurs(doc, host),
                max: max_occurs(doc, host),
                term: Term::Wildcard(
                    doc.attribute(host, "namespace")
                        .unwrap_or("##any")
                        .to_owned(),
                ),
            });
        }
        _ => return None,
    };

    let particles = doc
        .children(host)
        .iter()
        .copied()
        .filter_map(|c| particle_of(doc, c, groups, depth + 1))
        .collect();
    Some(Particle {
        min: min_occurs(doc, host),
        max: max_occurs(doc, host),
        term: Term::Group {
            compositor,
            particles,
        },
    })
}

fn min_occurs(doc: &Document, id: NodeId) -> usize {
    doc.attribute(id, "minOccurs")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
}

fn max_occurs(doc: &Document, id: NodeId) -> Option<usize> {
    match doc.attribute(id, "maxOccurs") {
        Some("unbounded") => None,
        Some(v) => v.parse().ok().or(Some(1)),
        None => Some(1),
    }
}
