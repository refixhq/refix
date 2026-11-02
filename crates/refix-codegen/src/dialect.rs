use refix_dictionary::{Dictionary, Tag};

/// Every length tag the dictionary links to a data field, in order.
pub(crate) fn length_tags(dictionary: &Dictionary) -> Vec<Tag> {
    let mut tags: Vec<Tag> = dictionary
        .fields()
        .iter()
        .filter_map(|field| field.length_tag)
        .collect();
    tags.sort_unstable();
    tags
}
