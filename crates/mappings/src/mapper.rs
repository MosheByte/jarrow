//! Lookups from any namespace to any other.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::desc;
use crate::error::Result;
use crate::model::{ClassMapping, FieldMapping, MappingSet, MethodMapping};

/// An index over a [`MappingSet`] that answers lookups keyed by names in one namespace
/// (`from`) and gives names in another (`to`).
///
/// Built once with [`MappingSet::mapper`]; every lookup is a hash-map access. Member
/// descriptors passed to lookups are in the `from` namespace.
pub struct Mapper<'a> {
    set: &'a MappingSet,
    from: usize,
    to: usize,
    classes: HashMap<&'a str, usize>,
    members: Vec<MemberIndex<'a>>,
}

/// Fields of one class by `from` name: (descriptor in `from`, position in the class).
type FieldCandidates<'a> = Vec<(Option<Cow<'a, str>>, usize)>;
/// Methods of one class by `from` name: (descriptor in `from`, position in the class).
type MethodCandidates<'a> = Vec<(Cow<'a, str>, usize)>;

#[derive(Default)]
struct MemberIndex<'a> {
    fields: HashMap<&'a str, FieldCandidates<'a>>,
    methods: HashMap<&'a str, MethodCandidates<'a>>,
}

impl MappingSet {
    /// Builds a [`Mapper`] from namespace `from` to namespace `to`.
    pub fn mapper(&self, from: &str, to: &str) -> Result<Mapper<'_>> {
        Mapper::new(self, from, to)
    }
}

impl<'a> Mapper<'a> {
    fn new(set: &'a MappingSet, from: &str, to: &str) -> Result<Self> {
        let from = set.require_namespace(from)?;
        let to = set.require_namespace(to)?;
        // Descriptors are stored in the primary namespace; lookups take them in `from`.
        let to_from: HashMap<&str, &str> = if from == 0 {
            HashMap::new()
        } else {
            set.classes()
                .iter()
                .filter_map(|c| Some((c.primary(), c.name(from)?)))
                .collect()
        };
        let convert = |d: &'a str, method: bool| -> Result<Cow<'a, str>> {
            if from == 0 {
                return Ok(Cow::Borrowed(d));
            }
            let remap = |n: &str| to_from.get(n).copied();
            Ok(Cow::Owned(if method {
                desc::remap_method_desc(d, remap)?
            } else {
                desc::remap_field_desc(d, remap)?
            }))
        };
        let mut classes = HashMap::with_capacity(set.classes().len());
        let mut members = Vec::with_capacity(set.classes().len());
        for (ci, c) in set.classes().iter().enumerate() {
            if let Some(name) = c.name(from) {
                classes.entry(name).or_insert(ci);
            }
            let mut index = MemberIndex::default();
            for (fi, f) in c.fields().iter().enumerate() {
                if let Some(name) = f.name(from) {
                    let d = f.desc().map(|d| convert(d, false)).transpose()?;
                    index.fields.entry(name).or_default().push((d, fi));
                }
            }
            for (mi, m) in c.methods().iter().enumerate() {
                if let Some(name) = m.name(from) {
                    let d = convert(m.desc(), true)?;
                    index.methods.entry(name).or_default().push((d, mi));
                }
            }
            members.push(index);
        }
        Ok(Self {
            set,
            from,
            to,
            classes,
            members,
        })
    }

    /// The namespace lookups are keyed by.
    pub fn from_namespace(&self) -> &'a str {
        &self.set.namespaces()[self.from]
    }

    /// The namespace lookups answer in.
    pub fn to_namespace(&self) -> &'a str {
        &self.set.namespaces()[self.to]
    }

    /// The class whose name in the `from` namespace is `name`.
    pub fn class(&self, name: &str) -> Option<&'a ClassMapping> {
        let i = *self.classes.get(name)?;
        self.set.classes().get(i)
    }

    /// The class name in the `to` namespace, or `None` when the class is unknown or has no
    /// name there.
    pub fn map_class(&self, name: &str) -> Option<&'a str> {
        self.class(name)?.name(self.to)
    }

    /// The field `owner.name`, all in the `from` namespace. With `Some(desc)`, the field with
    /// that descriptor (or one recorded without a descriptor); with `None`, the first field of
    /// that name.
    pub fn field(&self, owner: &str, name: &str, desc: Option<&str>) -> Option<&'a FieldMapping> {
        let ci = *self.classes.get(owner)?;
        let candidates = self.members[ci].fields.get(name)?;
        let fi = match desc {
            Some(d) => {
                candidates
                    .iter()
                    .find(|(cd, _)| cd.as_deref() == Some(d))
                    .or_else(|| candidates.iter().find(|(cd, _)| cd.is_none()))?
                    .1
            }
            None => candidates.first()?.1,
        };
        self.set.classes()[ci].fields().get(fi)
    }

    /// The field's name in the `to` namespace.
    pub fn map_field(&self, owner: &str, name: &str, desc: Option<&str>) -> Option<&'a str> {
        self.field(owner, name, desc)?.name(self.to)
    }

    /// The method `owner.name desc`, all in the `from` namespace.
    pub fn method(&self, owner: &str, name: &str, desc: &str) -> Option<&'a MethodMapping> {
        let ci = *self.classes.get(owner)?;
        let mi = self.members[ci]
            .methods
            .get(name)?
            .iter()
            .find(|(cd, _)| cd == desc)?
            .1;
        self.set.classes()[ci].methods().get(mi)
    }

    /// The method's name in the `to` namespace.
    pub fn map_method(&self, owner: &str, name: &str, desc: &str) -> Option<&'a str> {
        self.method(owner, name, desc)?.name(self.to)
    }

    /// Rewrites a field descriptor; unmapped classes keep their names.
    pub fn map_field_desc(&self, desc: &str) -> Result<String> {
        desc::remap_field_desc(desc, |n| self.map_class(n))
    }

    /// Rewrites a method descriptor; unmapped classes keep their names.
    pub fn map_method_desc(&self, desc: &str) -> Result<String> {
        desc::remap_method_desc(desc, |n| self.map_class(n))
    }

    /// Rewrites a generic signature; unmapped classes keep their names.
    pub fn map_signature(&self, signature: &str) -> Result<String> {
        desc::remap_signature(signature, |n| self.map_class(n))
    }

    /// Rewrites a `CONSTANT_Class` name, which may be an array descriptor.
    pub fn map_class_or_array(&self, name: &str) -> Result<String> {
        desc::remap_class_or_array(name, |n| self.map_class(n))
    }
}

#[cfg(test)]
mod tests {
    use crate::{FieldMapping, MappingSet, MethodMapping, names};

    #[test]
    fn lookups_in_every_direction() {
        let mut set = MappingSet::new(["obf", "intermediary", "named"]).unwrap();
        let a = set
            .add_class(names(["a", "net/minecraft/class_1", "pkg/Alpha"]))
            .unwrap();
        set.add_class(names(["b", "net/minecraft/class_2", "pkg/Beta"]))
            .unwrap();
        set.add_field(
            a,
            FieldMapping::new(names(["c", "field_3", "beta"]), Some("Lb;".into())),
        )
        .unwrap();
        set.add_method(
            a,
            MethodMapping::new(names(["d", "method_4", "make"]), "(Lb;)La;"),
        )
        .unwrap();

        let m = set.mapper("intermediary", "named").unwrap();
        assert_eq!(m.from_namespace(), "intermediary");
        assert_eq!(m.to_namespace(), "named");
        assert_eq!(m.map_class("net/minecraft/class_1"), Some("pkg/Alpha"));
        assert_eq!(m.map_class("a"), None);
        assert_eq!(
            m.map_field(
                "net/minecraft/class_1",
                "field_3",
                Some("Lnet/minecraft/class_2;")
            ),
            Some("beta")
        );
        assert_eq!(
            m.map_field("net/minecraft/class_1", "field_3", None),
            Some("beta")
        );
        assert_eq!(
            m.map_field("net/minecraft/class_1", "field_3", Some("Lb;")),
            None
        );
        assert_eq!(
            m.map_method(
                "net/minecraft/class_1",
                "method_4",
                "(Lnet/minecraft/class_2;)Lnet/minecraft/class_1;"
            ),
            Some("make")
        );
        assert_eq!(
            m.map_method_desc("(Lnet/minecraft/class_2;Ljava/lang/String;)V")
                .unwrap(),
            "(Lpkg/Beta;Ljava/lang/String;)V"
        );
        assert_eq!(
            m.map_signature("Ljava/util/List<Lnet/minecraft/class_1;>;")
                .unwrap(),
            "Ljava/util/List<Lpkg/Alpha;>;"
        );
        assert_eq!(
            m.map_class_or_array("[Lnet/minecraft/class_2;").unwrap(),
            "[Lpkg/Beta;"
        );
        assert!(m.map_field_desc("L;").is_err());

        let back = set.mapper("named", "obf").unwrap();
        assert_eq!(
            back.map_method("pkg/Alpha", "make", "(Lpkg/Beta;)Lpkg/Alpha;"),
            Some("d")
        );
        assert!(set.mapper("obf", "yarn").is_err());
    }
}
