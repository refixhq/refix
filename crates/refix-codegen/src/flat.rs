//! The fields and groups of a scope, with its components expanded in place.

use refix_dictionary::dictionary;
use refix_dictionary::spec::Field;

/// A field or group of a message or group instance, whichever component
/// lists it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Member<'a> {
    Field { field: &'a Field },
    Group(dictionary::Group<'a>),
}

/// The fields and groups in `members` in source order, with each component
/// replaced by its own.
pub(crate) fn members<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
) -> impl Iterator<Item = Member<'a>> {
    let mut flat = Vec::new();
    collect(members, &mut flat);
    flat.into_iter()
}

fn collect<'a>(members: impl Iterator<Item = dictionary::Member<'a>>, flat: &mut Vec<Member<'a>>) {
    for member in members {
        match member {
            dictionary::Member::Field { field, .. } => flat.push(Member::Field { field }),
            dictionary::Member::Group(group) => flat.push(Member::Group(group)),
        }
    }
}
