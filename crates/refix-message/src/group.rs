use crate::Tag;
use crate::message::{Scope, Slot};

/// CheckSum(10) closes every frame, so it always ends a group.
const CHECK_SUM: Tag = Tag(10);

/// The structure of a repeating group.
///
/// Codegen emits these as `const` items.
/// A dictionary loaded at runtime can build them borrowing from itself.
#[derive(Clone, Copy, Debug)]
pub struct GroupTable<'a> {
    count_tag: Tag,
    delimiter: Tag,
    members: &'a [Tag],
    nested: &'a [GroupTable<'a>],
}

impl<'a> GroupTable<'a> {
    /// A group counted by `count_tag`, whose instances start with
    /// `delimiter` and directly contain `members`.
    ///
    /// # Panics
    ///
    /// If `members` is not sorted and free of duplicates, or does not
    /// include the delimiter and every nested count tag. In a `const`,
    /// this is a compile error.
    pub const fn new(
        count_tag: Tag,
        delimiter: Tag,
        members: &'a [Tag],
        nested: &'a [GroupTable<'a>],
    ) -> Self {
        assert!(
            is_sorted_set(members),
            "group members must be sorted and unique"
        );
        assert!(
            contains(members, delimiter),
            "group members must include the delimiter"
        );
        let mut index = 0;
        while index < nested.len() {
            assert!(
                contains(members, nested[index].count_tag),
                "group members must include every nested count tag"
            );
            index += 1;
        }
        Self {
            count_tag,
            delimiter,
            members,
            nested,
        }
    }

    fn is_member(&self, tag: Tag) -> bool {
        self.members.binary_search(&tag).is_ok()
    }

    fn nested_counted_by(&self, tag: Tag) -> Option<&GroupTable<'a>> {
        self.nested.iter().find(|nested| nested.count_tag == tag)
    }
}

/// Every tag a message defines, at any depth, so the walker can tell a tag
/// that belongs elsewhere from one it does not know.
#[derive(Clone, Copy, Debug)]
pub struct KnownTags<'a>(&'a [Tag]);

impl<'a> KnownTags<'a> {
    /// # Panics
    ///
    /// If `tags` is not sorted and free of duplicates. In a `const`, this
    /// is a compile error.
    pub const fn new(tags: &'a [Tag]) -> Self {
        assert!(is_sorted_set(tags), "known tags must be sorted and unique");
        Self(tags)
    }

    fn contains(&self, tag: Tag) -> bool {
        self.0.binary_search(&tag).is_ok()
    }
}

/// What the walker found for one group, without judging it.
#[derive(Clone, Debug)]
pub struct Walk<'a> {
    declared_count: Option<usize>,
    instances: Vec<Scope<'a>>,
    anomalies: Vec<Anomaly>,
}

impl<'a> Walk<'a> {
    /// The group's NumInGroup value, or `None` if it is not a count.
    pub fn declared_count(&self) -> Option<usize> {
        self.declared_count
    }

    /// The instances found, in wire order.
    pub fn instances(&self) -> &[Scope<'a>] {
        &self.instances
    }

    /// How the group departs from its table; empty when it is well formed.
    pub fn anomalies(&self) -> &[Anomaly] {
        &self.anomalies
    }
}

/// A way a group on the wire departs from its table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Anomaly {
    /// The NumInGroup value is not a non-negative integer.
    InvalidCount,
    /// An instance starts with `tag` instead of the delimiter.
    MissingDelimiter { tag: Tag },
    /// The NumInGroup value disagrees with the instances found.
    CountMismatch { declared: usize, found: usize },
}

impl<'a> Scope<'a> {
    /// Walks the group `table` describes, or `None` if its count field is
    /// not in this scope.
    ///
    /// An instance starts at the delimiter, or wherever one of its tags
    /// repeats. A tag the message does not know stays in its instance; any
    /// other tag outside the table, or CheckSum, ends the group. Nothing is
    /// rejected: malformed groups are reported as anomalies.
    pub fn walk_group(&self, table: &GroupTable<'_>, known: &KnownTags<'_>) -> Option<Walk<'a>> {
        let (count_slot, _) = self.find(table.count_tag, self.start())?;
        Some(walk(self, count_slot, table, known).0)
    }
}

/// Walks the group whose count field sits at `count_slot`, returning what
/// it found and the slot where the group ends.
fn walk<'a>(
    scope: &Scope<'a>,
    count_slot: Slot,
    table: &GroupTable<'_>,
    known: &KnownTags<'_>,
) -> (Walk<'a>, Slot) {
    let declared_count = parse_count(scope.value_at(count_slot));
    let mut anomalies = Vec::new();
    if declared_count.is_none() {
        anomalies.push(Anomaly::InvalidCount);
    }

    let mut instances = Vec::new();
    let mut instance_start = None;
    let mut seen = Vec::new();
    let mut slot = count_slot.next();
    while slot < scope.end() {
        let tag = scope.tag_at(slot);
        if tag == CHECK_SUM {
            break;
        }
        if table.is_member(tag) {
            if instance_start.is_none() || seen.contains(&tag) {
                if let Some(start) = instance_start {
                    instances.push(scope.narrow(start, slot));
                }
                if tag != table.delimiter {
                    anomalies.push(Anomaly::MissingDelimiter { tag });
                }
                instance_start = Some(slot);
                seen.clear();
            }
            seen.push(tag);
            slot = match table.nested_counted_by(tag) {
                Some(nested) => walk(scope, slot, nested, known).1,
                None => slot.next(),
            };
        } else if known.contains(tag) {
            break;
        } else {
            slot = slot.next();
        }
    }
    if let Some(start) = instance_start {
        instances.push(scope.narrow(start, slot));
    }

    if let Some(declared) = declared_count
        && declared != instances.len()
    {
        anomalies.push(Anomaly::CountMismatch {
            declared,
            found: instances.len(),
        });
    }

    let walk = Walk {
        declared_count,
        instances,
        anomalies,
    };
    (walk, slot)
}

/// A NumInGroup value: a non-negative integer.
fn parse_count(value: &[u8]) -> Option<usize> {
    std::str::from_utf8(value).ok()?.parse().ok()
}

/// Whether `tags` is strictly increasing.
const fn is_sorted_set(tags: &[Tag]) -> bool {
    let mut index = 1;
    while index < tags.len() {
        if tags[index - 1].0 >= tags[index].0 {
            return false;
        }
        index += 1;
    }
    true
}

/// Whether `tags` contains `tag`.
const fn contains(tags: &[Tag], tag: Tag) -> bool {
    let mut index = 0;
    while index < tags.len() {
        if tags[index].0 == tag.0 {
            return true;
        }
        index += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::construct_valid_frame;
    use crate::{RawMessage, Tokenizer};
    use bytes::Bytes;

    const NO_PARTY_SUB_IDS: GroupTable<'static> =
        GroupTable::new(Tag(802), Tag(523), &[Tag(523), Tag(803)], &[]);

    const NO_PARTY_IDS: GroupTable<'static> = GroupTable::new(
        Tag(453),
        Tag(448),
        &[Tag(447), Tag(448), Tag(452), Tag(802)],
        &[NO_PARTY_SUB_IDS],
    );

    const KNOWN: KnownTags<'static> = KnownTags::new(&[
        Tag(11),
        Tag(35),
        Tag(54),
        Tag(447),
        Tag(448),
        Tag(452),
        Tag(453),
        Tag(523),
        Tag(802),
        Tag(803),
    ]);

    /// A FIX.4.4 frame around the `|`-delimited `body`.
    fn message(body: &str) -> RawMessage {
        let frame = construct_valid_frame("FIX.4.4", body);
        Tokenizer::default().tokenize(Bytes::from(frame)).unwrap()
    }

    fn walk_parties(message: &RawMessage) -> Walk<'_> {
        message.scope().walk_group(&NO_PARTY_IDS, &KNOWN).unwrap()
    }

    fn text<'a>(scope: &Scope<'a>, tag: u32) -> Option<&'a str> {
        scope.get_str(Tag(tag)).unwrap()
    }

    mod structure {
        use super::*;

        #[test]
        fn an_absent_group_walks_to_nothing() {
            let message = message("35=D|11=X|54=1|");
            assert!(message.scope().walk_group(&NO_PARTY_IDS, &KNOWN).is_none());
        }

        #[test]
        fn finds_each_instance() {
            let message = message("35=D|453=2|448=AL|452=1|448=BOB|452=3|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(walk.declared_count(), Some(2));
            assert!(walk.anomalies().is_empty());
            let [first, second] = walk.instances() else {
                panic!("expected two instances");
            };
            assert_eq!(
                (text(first, 448), text(first, 452)),
                (Some("AL"), Some("1"))
            );
            assert_eq!(
                (text(second, 448), text(second, 452)),
                (Some("BOB"), Some("3"))
            );
            assert_eq!(text(second, 54), None);
        }

        #[test]
        fn a_missing_optional_field_does_not_read_the_next_instance() {
            let message = message("35=D|453=2|448=AL|448=BOB|452=3|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(text(&walk.instances()[0], 452), None);
        }

        #[test]
        fn a_nested_group_stays_inside_its_instance() {
            let message = message("35=D|453=2|448=AL|802=1|523=S1|452=1|448=BOB|452=3|54=1|");
            let walk = walk_parties(&message);

            assert!(walk.anomalies().is_empty());
            let [first, second] = walk.instances() else {
                panic!("expected two instances");
            };
            assert_eq!(text(first, 452), Some("1"));
            let nested = first.walk_group(&NO_PARTY_SUB_IDS, &KNOWN).unwrap();
            assert_eq!(nested.instances().len(), 1);
            assert_eq!(text(&nested.instances()[0], 523), Some("S1"));
            assert!(second.walk_group(&NO_PARTY_SUB_IDS, &KNOWN).is_none());
        }

        #[test]
        fn a_nested_group_can_be_the_delimiter() {
            const OUTER: GroupTable<'static> = GroupTable::new(
                Tag(453),
                Tag(802),
                &[Tag(452), Tag(802)],
                &[NO_PARTY_SUB_IDS],
            );
            let message = message("35=D|453=2|802=1|523=A|452=1|802=1|523=B|54=1|");
            let walk = message.scope().walk_group(&OUTER, &KNOWN).unwrap();

            assert!(walk.anomalies().is_empty());
            let [first, second] = walk.instances() else {
                panic!("expected two instances");
            };
            assert_eq!((text(first, 523), text(first, 452)), (Some("A"), Some("1")));
            assert_eq!((text(second, 523), text(second, 452)), (Some("B"), None));
        }

        #[test]
        fn an_unknown_tag_stays_in_its_instance() {
            let message = message("35=D|453=2|448=AL|5001=x|452=1|448=BOB|54=1|");
            let walk = walk_parties(&message);

            assert!(walk.anomalies().is_empty());
            let first = &walk.instances()[0];
            assert_eq!(
                (text(first, 5001), text(first, 452)),
                (Some("x"), Some("1"))
            );
        }

        #[test]
        fn check_sum_ends_a_group_at_the_end_of_the_body() {
            let message = message("35=D|453=1|448=AL|");
            let walk = walk_parties(&message);

            assert_eq!(walk.instances().len(), 1);
            assert_eq!(walk.instances()[0].get(Tag(10)), None);
        }

        #[test]
        fn a_zero_count_is_an_empty_group() {
            let message = message("35=D|453=0|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(walk.declared_count(), Some(0));
            assert!(walk.instances().is_empty());
            assert!(walk.anomalies().is_empty());
        }
    }

    mod anomalies {
        use super::*;

        #[test]
        fn an_invalid_count_is_reported() {
            let message = message("35=D|453=x|448=AL|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(walk.declared_count(), None);
            assert_eq!(walk.anomalies(), [Anomaly::InvalidCount]);
            assert_eq!(walk.instances().len(), 1);
        }

        #[test]
        fn a_count_mismatch_is_reported() {
            let message = message("35=D|453=3|448=AL|448=BOB|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(
                walk.anomalies(),
                [Anomaly::CountMismatch {
                    declared: 3,
                    found: 2,
                }]
            );
        }

        #[test]
        fn a_reordered_instance_is_kept_whole() {
            let message = message("35=D|453=1|452=1|448=AL|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(
                walk.anomalies(),
                [Anomaly::MissingDelimiter { tag: Tag(452) }]
            );
            let [instance] = walk.instances() else {
                panic!("expected one instance");
            };
            assert_eq!(
                (text(instance, 452), text(instance, 448)),
                (Some("1"), Some("AL"))
            );
        }

        #[test]
        fn a_repeated_tag_starts_a_new_instance() {
            let message = message("35=D|453=2|448=AL|452=1|452=3|54=1|");
            let walk = walk_parties(&message);

            assert_eq!(
                walk.anomalies(),
                [Anomaly::MissingDelimiter { tag: Tag(452) }]
            );
            let [first, second] = walk.instances() else {
                panic!("expected two instances");
            };
            assert_eq!(text(first, 452), Some("1"));
            assert_eq!((text(second, 452), text(second, 448)), (Some("3"), None));
        }
    }

    mod tables {
        use super::*;

        #[test]
        #[should_panic(expected = "group members must be sorted and unique")]
        fn unsorted_members_are_rejected() {
            GroupTable::new(Tag(453), Tag(448), &[Tag(452), Tag(448)], &[]);
        }

        #[test]
        #[should_panic(expected = "group members must include the delimiter")]
        fn members_without_the_delimiter_are_rejected() {
            GroupTable::new(Tag(453), Tag(448), &[Tag(452)], &[]);
        }

        #[test]
        #[should_panic(expected = "group members must include every nested count tag")]
        fn members_without_a_nested_count_tag_are_rejected() {
            GroupTable::new(Tag(453), Tag(448), &[Tag(448)], &[NO_PARTY_SUB_IDS]);
        }

        #[test]
        #[should_panic(expected = "known tags must be sorted and unique")]
        fn duplicate_known_tags_are_rejected() {
            KnownTags::new(&[Tag(35), Tag(35)]);
        }
    }
}
