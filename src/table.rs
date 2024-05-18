use {super::*, std::collections::btree_map};

pub(crate) type Table<'src, T> = BTreeMap<&'src str, T>;
