use crate::Tag;

/// Length tags whose value gives the byte count of the immediately following
/// data field.
///
/// Derived from the QuickFIX FIX 4.0 - 5.0 SP2 dictionaries:
/// every LENGTH field paired with a DATA field.
///
/// *Note*: these values must remain ordered.
pub(crate) const STANDARD: &[u32] = &[
    90, 93, 95, 212, 348, 350, 352, 354, 356, 358, 360, 362, 364, 445, 618, 621, 1184, 1277, 1280,
    1282, 1397, 1401, 1403, 1468, 1525, 1578, 1620, 1664, 1678, 1733, 1871, 1874, 2072, 2074, 2111,
    2179, 2287, 2351, 2372, 2481, 2494, 2522, 2637, 2651, 2665, 2715, 2718, 2721, 2797, 2802, 2809,
    2815, 40004, 40008, 40978, 40980, 40982, 40984, 40986, 40988, 41083, 41101, 41107, 41256,
    41320, 41324, 41458, 41476, 41482, 41653, 41710, 41806, 41811, 41873, 41969, 42025, 42171,
    42451, 42652, 42947, 43109, 43110, 43111,
];

/// Length tags below this go in the bitmap, the rest in a sorted list.
const BITMAP_TAGS: u32 = 65_536;
// Whole words, so no tag at or above the cap shares a word with one below it
const _: () = assert!(BITMAP_TAGS.is_multiple_of(64));

/// The length tags a tokenizer recognises.
#[derive(Clone, Debug)]
pub(crate) struct LengthTags {
    bitmap: Vec<u64>,
    above: Vec<Tag>,
}

impl LengthTags {
    pub(crate) fn new(tags: impl IntoIterator<Item = Tag>) -> Self {
        let (below, mut above): (Vec<Tag>, Vec<Tag>) =
            tags.into_iter().partition(|tag| tag.0 < BITMAP_TAGS);
        let words = below.iter().map(|&tag| word(tag) + 1).max().unwrap_or(0);
        let mut bitmap = vec![0; words];
        for tag in below {
            bitmap[word(tag)] |= bit(tag);
        }
        above.sort_unstable();
        above.dedup();
        Self { bitmap, above }
    }

    pub(crate) fn contains(&self, tag: Tag) -> bool {
        match self.bitmap.get(word(tag)) {
            Some(&bits) => bits & bit(tag) != 0,
            None => self.contains_above(tag),
        }
    }

    // The uncommon case, kept out of line so more can be inlined up the call stack
    #[inline(never)]
    fn contains_above(&self, tag: Tag) -> bool {
        self.above.binary_search(&tag).is_ok()
    }
}

fn word(tag: Tag) -> usize {
    tag.0 as usize / 64
}

fn bit(tag: Tag) -> u64 {
    1 << (tag.0 % 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_is_sorted_and_unique() {
        assert!(STANDARD.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn agrees_with_the_standard_list_past_the_cap() {
        let tags = LengthTags::new(STANDARD.iter().copied().map(Tag));
        for tag in 0..BITMAP_TAGS + 64 {
            assert_eq!(
                tags.contains(Tag(tag)),
                STANDARD.contains(&tag),
                "tag {tag}"
            );
        }
    }

    #[test]
    fn tags_either_side_of_the_cap() {
        let edges = [BITMAP_TAGS - 1, BITMAP_TAGS, u32::MAX];
        let tags = LengthTags::new(edges.map(Tag));
        assert!(edges.iter().all(|&tag| tags.contains(Tag(tag))));
        assert!(!tags.contains(Tag(BITMAP_TAGS + 1)));
    }

    #[test]
    fn a_tag_above_the_cap_needs_no_bitmap() {
        let tags = LengthTags::new([Tag(u32::MAX)]);
        assert!(tags.bitmap.is_empty());
    }
}
