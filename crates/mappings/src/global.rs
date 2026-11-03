//! A global member map for namespaces whose member names are unique on their own, such as
//! intermediary (`field_1234`, `method_5678`, `comp_90`).
//!
//! In such a namespace a member can be remapped from its name alone, without its owner or
//! descriptor, which is what a remapper needs for references it cannot resolve to a declaring
//! class (reflection strings, mixin targets). The map records every name that maps to more
//! than one target as a [`MemberConflict`] and never answers for it.

use std::collections::HashMap;

use crate::error::Result;
use crate::model::MappingSet;

/// Fields or methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberKind {
    /// A field.
    Field,
    /// A method.
    Method,
}

/// One target of a conflicting name, with the first owner seen giving it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictTarget {
    /// The owner class, in the `from` namespace (the primary name if it has none there).
    pub owner: String,
    /// The target name.
    pub target: String,
}

/// A member name that maps to two or more different names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberConflict {
    /// Field or method.
    pub kind: MemberKind,
    /// The name in the `from` namespace.
    pub name: String,
    /// Every distinct target, in the order first seen.
    pub targets: Vec<ConflictTarget>,
}

/// What a global lookup found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalLookup<'a> {
    /// Every occurrence maps to this one name.
    Unique(&'a str),
    /// Occurrences disagree; nothing is answered.
    Conflict(&'a MemberConflict),
    /// The name is not in the map.
    Missing,
}

enum Slot {
    Unique { target: String, owner: String },
    Conflict(usize),
}

/// Member name to member name, without owners or descriptors.
pub struct GlobalMemberMap {
    from: String,
    to: String,
    fields: HashMap<String, Slot>,
    methods: HashMap<String, Slot>,
    conflicts: Vec<MemberConflict>,
}

/// Whether `name` has the shape of an intermediary member name: `field_<n>`, `method_<n>` or
/// `comp_<n>` (record components).
pub fn is_intermediary_member_name(name: &str) -> bool {
    ["field_", "method_", "comp_"].iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

impl GlobalMemberMap {
    /// Builds the map over every field and method that has a name in both namespaces.
    pub fn build(set: &MappingSet, from: &str, to: &str) -> Result<Self> {
        Self::build_filtered(set, from, to, |_| true)
    }

    /// Builds the map over the members whose `from` name `keep` accepts, for example
    /// [`is_intermediary_member_name`]. Members without a name in `to` are skipped: they are
    /// not a second target, only a missing one.
    pub fn build_filtered(
        set: &MappingSet,
        from: &str,
        to: &str,
        mut keep: impl FnMut(&str) -> bool,
    ) -> Result<Self> {
        let fi = set.require_namespace(from)?;
        let ti = set.require_namespace(to)?;
        let mut map = GlobalMemberMap {
            from: from.to_owned(),
            to: to.to_owned(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            conflicts: Vec::new(),
        };
        for class in set.classes() {
            let owner = class.name(fi).unwrap_or(class.primary());
            for f in class.fields() {
                if let (Some(name), Some(target)) = (f.name(fi), f.name(ti))
                    && keep(name)
                {
                    map.insert(MemberKind::Field, name, target, owner);
                }
            }
            for m in class.methods() {
                if let (Some(name), Some(target)) = (m.name(fi), m.name(ti))
                    && keep(name)
                {
                    map.insert(MemberKind::Method, name, target, owner);
                }
            }
        }
        Ok(map)
    }

    fn insert(&mut self, kind: MemberKind, name: &str, target: &str, owner: &str) {
        let slots = match kind {
            MemberKind::Field => &mut self.fields,
            MemberKind::Method => &mut self.methods,
        };
        let Some(slot) = slots.get_mut(name) else {
            slots.insert(
                name.to_owned(),
                Slot::Unique {
                    target: target.to_owned(),
                    owner: owner.to_owned(),
                },
            );
            return;
        };
        match slot {
            Slot::Unique {
                target: existing,
                owner: first_owner,
            } => {
                if existing != target {
                    self.conflicts.push(MemberConflict {
                        kind,
                        name: name.to_owned(),
                        targets: vec![
                            ConflictTarget {
                                owner: std::mem::take(first_owner),
                                target: std::mem::take(existing),
                            },
                            ConflictTarget {
                                owner: owner.to_owned(),
                                target: target.to_owned(),
                            },
                        ],
                    });
                    *slot = Slot::Conflict(self.conflicts.len() - 1);
                }
            }
            Slot::Conflict(i) => {
                let conflict = &mut self.conflicts[*i];
                if !conflict.targets.iter().any(|t| t.target == target) {
                    conflict.targets.push(ConflictTarget {
                        owner: owner.to_owned(),
                        target: target.to_owned(),
                    });
                }
            }
        }
    }

    /// The namespace names are taken from.
    pub fn from_namespace(&self) -> &str {
        &self.from
    }

    /// The namespace names are mapped to.
    pub fn to_namespace(&self) -> &str {
        &self.to
    }

    fn lookup<'s>(&'s self, slots: &'s HashMap<String, Slot>, name: &str) -> GlobalLookup<'s> {
        match slots.get(name) {
            None => GlobalLookup::Missing,
            Some(Slot::Unique { target, .. }) => GlobalLookup::Unique(target),
            Some(Slot::Conflict(i)) => GlobalLookup::Conflict(&self.conflicts[*i]),
        }
    }

    /// Looks a field name up.
    pub fn field(&self, name: &str) -> GlobalLookup<'_> {
        self.lookup(&self.fields, name)
    }

    /// Looks a method name up.
    pub fn method(&self, name: &str) -> GlobalLookup<'_> {
        self.lookup(&self.methods, name)
    }

    /// The field's target name, only when it is unique.
    pub fn map_field(&self, name: &str) -> Option<&str> {
        match self.field(name) {
            GlobalLookup::Unique(t) => Some(t),
            _ => None,
        }
    }

    /// The method's target name, only when it is unique.
    pub fn map_method(&self, name: &str) -> Option<&str> {
        match self.method(name) {
            GlobalLookup::Unique(t) => Some(t),
            _ => None,
        }
    }

    /// Every conflicting name, in the order found.
    pub fn conflicts(&self) -> &[MemberConflict] {
        &self.conflicts
    }

    /// How many field names the map knows (conflicting ones included).
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// How many method names the map knows (conflicting ones included).
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldMapping, MethodMapping, names};

    #[test]
    fn intermediary_names() {
        for good in ["field_1", "method_12345", "comp_0"] {
            assert!(is_intermediary_member_name(good), "{good}");
        }
        for bad in ["field_", "method_x", "a", "class_1", "field_1a", "Method_1"] {
            assert!(!is_intermediary_member_name(bad), "{bad}");
        }
    }

    #[test]
    fn overrides_agree_and_conflicts_are_reported() {
        let mut set = MappingSet::new(["intermediary", "mojang"]).unwrap();
        let a = set.add_class(names(["class_1", "pkg/Base"])).unwrap();
        let b = set.add_class(names(["class_2", "pkg/Sub"])).unwrap();
        let c = set.add_class(names(["class_3", "pkg/Odd"])).unwrap();
        for class in [a, b] {
            set.add_method(
                class,
                MethodMapping::new(names(["method_1", "tick"]), "()V"),
            )
            .unwrap();
        }
        set.add_method(c, MethodMapping::new(names(["method_2", "first"]), "()V"))
            .unwrap();
        set.add_method(b, MethodMapping::new(names(["method_2", "second"]), "()V"))
            .unwrap();
        set.add_method(a, MethodMapping::new(names(["method_2", "third"]), "()V"))
            .unwrap();
        set.add_method(a, MethodMapping::new(names(["method_3", ""]), "()V"))
            .unwrap();
        set.add_field(
            c,
            FieldMapping::new(names(["field_9", "value"]), Some("I".into())),
        )
        .unwrap();
        set.add_method(a, MethodMapping::new(names(["helper", "helper"]), "()V"))
            .unwrap();

        let map = GlobalMemberMap::build_filtered(
            &set,
            "intermediary",
            "mojang",
            is_intermediary_member_name,
        )
        .unwrap();
        assert_eq!(map.from_namespace(), "intermediary");
        assert_eq!(map.to_namespace(), "mojang");
        assert_eq!(map.map_method("method_1"), Some("tick"));
        assert_eq!(map.map_field("field_9"), Some("value"));
        assert_eq!(map.map_method("method_2"), None);
        assert_eq!(map.method("method_3"), GlobalLookup::Missing);
        assert_eq!(map.method("helper"), GlobalLookup::Missing);
        assert_eq!(map.field("method_1"), GlobalLookup::Missing);
        let GlobalLookup::Conflict(conflict) = map.method("method_2") else {
            panic!("method_2 should conflict");
        };
        assert_eq!(conflict.kind, MemberKind::Method);
        let targets: Vec<_> = conflict
            .targets
            .iter()
            .map(|t| (t.owner.as_str(), t.target.as_str()))
            .collect();
        assert_eq!(
            targets,
            [
                ("class_1", "third"),
                ("class_2", "second"),
                ("class_3", "first")
            ]
        );
        assert_eq!(map.conflicts().len(), 1);
        assert_eq!(map.method_count(), 2);
        assert_eq!(map.field_count(), 1);
    }
}
