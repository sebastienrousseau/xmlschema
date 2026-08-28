<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Governance

## Roles and responsibilities

| Role | Held by | Responsible for |
|---|---|---|
| Project lead | Sebastien Rousseau | Merging changes, cutting releases, responding to vulnerability reports, and deciding scope |
| Contributor | Anyone opening a pull request | The change they propose, and the tests that go with it |

There is no separate reviewer, release-manager or security-officer
role today, because there is one maintainer. Where this document names
a responsibility, the project lead holds it.

## Decision making

Changes arrive as pull requests and are merged by the project lead
once CI is green. Disagreements are settled in the pull request or the
issue that prompted it. There is no voting procedure, because there is
no second voter — see *Bus factor* below.

## Bus factor

**The bus factor of this project is one.** That is a statement of
fact, not an aspiration: a single person has commit access, publishes
releases, and holds the crates.io ownership.

The project does not pretend otherwise, and the mitigations are the
ones available to a single-maintainer project:

- Everything needed to build, test and release is in the repository —
  `scripts/gate.sh` runs what CI runs, and `scripts/publish.sh` runs a
  release. Neither depends on a machine only one person has.
- The licence is MIT OR Apache-2.0, so a fork needs no permission.
- Every release is tagged and signed, so a successor can establish
  what shipped and when.

## Access continuity

| Asset | Who holds it | If they are unavailable |
|---|---|---|
| GitHub repository | Project lead | The repository is public; anyone may fork and continue under the licence |
| crates.io ownership | Project lead | A successor publishes under a new name, or the lead adds an owner in advance |
| Release signing key | Project lead | A successor signs with their own key; historical tags remain verifiable |

If the project lead becomes unavailable for an extended period, the
honest expectation is that this repository stops receiving updates.
The public history, tags and documentation are sufficient for someone
else to take it up, and the licence permits it.

## Becoming a maintainer

The project would benefit from a second maintainer, and the criterion
that most obviously blocks a higher OpenSSF Best Practices level is
exactly that. A contributor who lands several substantive changes and
wants commit access should open an issue asking for it.

## Versioning and releases

See [CONTRIBUTING.md](CONTRIBUTING.md) and the release process the
suite shares. Every crate in the oxml suite carries the same version
number, so there is never a compatibility table to consult.
