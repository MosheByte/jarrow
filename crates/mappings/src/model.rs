//! The mapping model: a [`MappingSet`] with named namespaces.
//!
//! Every class, field, method and package carries one name per namespace. The first namespace
//! is the *primary* one: every class, field, method and package has a name there, member
//! descriptors are written in it, and the set is indexed by it. Names in the other namespaces
//! may be missing (`None`): the entry has no name there.
//!
//! Insertion order is kept everywhere, so writers and transformations are deterministic. Hash
//! maps are used only for lookups, never iterated to produce output.

use std::collections::HashMap;
use std::fmt;

use crate::desc;
use crate::error::{Error, Result};

/// One name per namespace. `None` (or an empty string, which constructors normalise to `None`)
/// means the entry has no name in that namespace.
pub type Names = Vec<Option<String>>;

/// Builds [`Names`] from strings; an empty string becomes `None`.
///
/// ```
/// let n = jarrow_mappings::names(["a", "", "Foo"]);
/// assert_eq!(n, vec![Some("a".to_owned()), None, Some("Foo".to_owned())]);
/// ```
pub fn names<I, S>(items: I) -> Names
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    items
        .into_iter()
        .map(|s| {
            let s = s.as_ref();
            (!s.is_empty()).then(|| s.to_owned())
        })
        .collect()
}

fn normalise(names: Names) -> Names {
    names
        .into_iter()
        .map(|n| n.filter(|s| !s.is_empty()))
        .collect()
}

fn name_at(names: &[Option<String>], ns: usize) -> Option<&str> {
    names.get(ns).and_then(|n| n.as_deref())
}

/// A ProGuard line-number range: the obfuscated lines `start..=end`, and optionally the
/// original lines they came from (`:original_start[:original_end]` in the file).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LineRange {
    /// First obfuscated line.
    pub start: u32,
    /// Last obfuscated line.
    pub end: u32,
    /// First original line, when the file gives one.
    pub original_start: Option<u32>,
    /// Last original line, when the file gives one (only together with `original_start`).
    pub original_end: Option<u32>,
}

/// A ProGuard method line whose original name is qualified with another class: code of
/// `owner.name` that was inlined into the class it is listed under. It is kept so a ProGuard
/// file reads and writes back, but it is not a member of the class and takes no part in
/// lookups, composition or projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineFrame {
    /// The obfuscated line range.
    pub lines: LineRange,
    /// The original owner, as an internal name in the original namespace.
    pub owner: String,
    /// The original method name.
    pub name: String,
    /// The method descriptor in the original namespace.
    pub desc: String,
    /// The obfuscated name the line maps to.
    pub obfuscated: String,
}

/// A package mapping (TSRG and TSRG2 package lines). Names have no trailing `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMapping {
    names: Names,
}

impl PackageMapping {
    /// The package name in namespace `ns` (by index).
    pub fn name(&self, ns: usize) -> Option<&str> {
        name_at(&self.names, ns)
    }

    /// The package name in the primary namespace.
    pub fn primary(&self) -> &str {
        self.name(0).unwrap_or_default()
    }

    /// All names, one per namespace.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }
}

/// A field: names per namespace and, when the format has one, its descriptor in the primary
/// namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMapping {
    names: Names,
    desc: Option<String>,
    comment: Option<String>,
}

impl FieldMapping {
    /// A field with the given names and descriptor (in the primary namespace).
    pub fn new(names: Names, desc: Option<String>) -> Self {
        Self {
            names: normalise(names),
            desc,
            comment: None,
        }
    }

    /// The same field with a comment.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// The name in namespace `ns` (by index).
    pub fn name(&self, ns: usize) -> Option<&str> {
        name_at(&self.names, ns)
    }

    /// The name in the primary namespace.
    pub fn primary(&self) -> &str {
        self.name(0).unwrap_or_default()
    }

    /// All names, one per namespace.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }

    /// The field descriptor in the primary namespace, if the source format had one.
    pub fn desc(&self) -> Option<&str> {
        self.desc.as_deref()
    }

    /// The documentation comment (tiny v2).
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Replaces the comment.
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }
}

/// A method parameter, keyed by its local-variable index. Any name may be missing, the primary
/// one included: obfuscated code has no parameter names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamMapping {
    lv_index: u32,
    names: Names,
    comment: Option<String>,
}

impl ParamMapping {
    /// A parameter at local-variable index `lv_index`.
    pub fn new(lv_index: u32, names: Names) -> Self {
        Self {
            lv_index,
            names: normalise(names),
            comment: None,
        }
    }

    /// The same parameter with a comment.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// The local-variable index (0 is `this` in an instance method).
    pub fn lv_index(&self) -> u32 {
        self.lv_index
    }

    /// The name in namespace `ns` (by index).
    pub fn name(&self, ns: usize) -> Option<&str> {
        name_at(&self.names, ns)
    }

    /// All names, one per namespace.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }

    /// The documentation comment (tiny v2).
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Replaces the comment.
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }
}

/// A local variable (tiny v2 `v` lines), keyed by index, start offset and LVT row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarMapping {
    lv_index: u32,
    start_offset: u32,
    lvt_index: Option<u32>,
    names: Names,
    comment: Option<String>,
}

impl VarMapping {
    /// A local variable. `lvt_index` is `None` when the file gives `-1`.
    pub fn new(lv_index: u32, start_offset: u32, lvt_index: Option<u32>, names: Names) -> Self {
        Self {
            lv_index,
            start_offset,
            lvt_index,
            names: normalise(names),
            comment: None,
        }
    }

    /// The same variable with a comment.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// The local-variable index.
    pub fn lv_index(&self) -> u32 {
        self.lv_index
    }

    /// The bytecode offset where the variable's scope starts.
    pub fn start_offset(&self) -> u32 {
        self.start_offset
    }

    /// The row in the local variable table, when known.
    pub fn lvt_index(&self) -> Option<u32> {
        self.lvt_index
    }

    /// The name in namespace `ns` (by index).
    pub fn name(&self, ns: usize) -> Option<&str> {
        name_at(&self.names, ns)
    }

    /// All names, one per namespace.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }

    /// The documentation comment (tiny v2).
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Replaces the comment.
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }

    fn key(&self) -> (u32, u32, Option<u32>) {
        (self.lv_index, self.start_offset, self.lvt_index)
    }
}

/// A method: names per namespace, the descriptor in the primary namespace, and its
/// parameters, local variables, ProGuard line ranges and TSRG2 static marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodMapping {
    names: Names,
    desc: String,
    comment: Option<String>,
    params: Vec<ParamMapping>,
    vars: Vec<VarMapping>,
    lines: Vec<LineRange>,
    is_static: bool,
}

impl MethodMapping {
    /// A method with the given names and descriptor (in the primary namespace).
    pub fn new(names: Names, desc: impl Into<String>) -> Self {
        Self {
            names: normalise(names),
            desc: desc.into(),
            comment: None,
            params: Vec::new(),
            vars: Vec::new(),
            lines: Vec::new(),
            is_static: false,
        }
    }

    /// The same method with a comment.
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// The same method with one more parameter (merged by index when added to a set).
    pub fn with_param(mut self, param: ParamMapping) -> Self {
        self.params.push(param);
        self
    }

    /// The same method with one more local variable.
    pub fn with_var(mut self, var: VarMapping) -> Self {
        self.vars.push(var);
        self
    }

    /// The same method with one more ProGuard line range.
    pub fn with_lines(mut self, lines: LineRange) -> Self {
        self.lines.push(lines);
        self
    }

    /// The same method marked static (TSRG2 `static` line).
    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// The name in namespace `ns` (by index).
    pub fn name(&self, ns: usize) -> Option<&str> {
        name_at(&self.names, ns)
    }

    /// The name in the primary namespace.
    pub fn primary(&self) -> &str {
        self.name(0).unwrap_or_default()
    }

    /// All names, one per namespace.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }

    /// The method descriptor in the primary namespace.
    pub fn desc(&self) -> &str {
        &self.desc
    }

    /// The documentation comment (tiny v2).
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Replaces the comment.
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }

    /// Parameters, in insertion order.
    pub fn params(&self) -> &[ParamMapping] {
        &self.params
    }

    /// The parameter at local-variable index `lv_index`.
    pub fn param(&self, lv_index: u32) -> Option<&ParamMapping> {
        self.params.iter().find(|p| p.lv_index == lv_index)
    }

    /// Mutable access to a parameter by position, for setting its comment.
    pub fn param_at_mut(&mut self, index: usize) -> Option<&mut ParamMapping> {
        self.params.get_mut(index)
    }

    /// Local variables, in insertion order.
    pub fn vars(&self) -> &[VarMapping] {
        &self.vars
    }

    /// Mutable access to a local variable by position, for setting its comment.
    pub fn var_at_mut(&mut self, index: usize) -> Option<&mut VarMapping> {
        self.vars.get_mut(index)
    }

    /// ProGuard line ranges, in file order.
    pub fn lines(&self) -> &[LineRange] {
        &self.lines
    }

    /// Whether a TSRG2 file marked the method static.
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Merges a parameter by local-variable index; returns its position.
    fn merge_param(
        &mut self,
        param: ParamMapping,
        namespaces: &[String],
        key: &str,
    ) -> Result<usize> {
        check_count(param.names.len(), namespaces.len())?;
        match self
            .params
            .iter()
            .position(|p| p.lv_index == param.lv_index)
        {
            Some(i) => {
                let existing = &mut self.params[i];
                merge_names(
                    &mut existing.names,
                    param.names,
                    "parameter",
                    key,
                    namespaces,
                )?;
                fill(&mut existing.comment, param.comment);
                Ok(i)
            }
            None => {
                self.params.push(param);
                Ok(self.params.len() - 1)
            }
        }
    }

    fn merge_var(&mut self, var: VarMapping, namespaces: &[String], key: &str) -> Result<usize> {
        check_count(var.names.len(), namespaces.len())?;
        match self.vars.iter().position(|v| v.key() == var.key()) {
            Some(i) => {
                let existing = &mut self.vars[i];
                merge_names(
                    &mut existing.names,
                    var.names,
                    "local variable",
                    key,
                    namespaces,
                )?;
                fill(&mut existing.comment, var.comment);
                Ok(i)
            }
            None => {
                self.vars.push(var);
                Ok(self.vars.len() - 1)
            }
        }
    }
}

/// A class with its fields, methods and (ProGuard only) inline frames.
#[derive(Clone)]
pub struct ClassMapping {
    names: Names,
    comment: Option<String>,
    fields: Vec<FieldMapping>,
    methods: Vec<MethodMapping>,
    inline_frames: Vec<InlineFrame>,
    field_index: HashMap<String, Vec<usize>>,
    method_index: HashMap<String, Vec<usize>>,
}

impl PartialEq for ClassMapping {
    fn eq(&self, other: &Self) -> bool {
        self.names == other.names
            && self.comment == other.comment
            && self.fields == other.fields
            && self.methods == other.methods
            && self.inline_frames == other.inline_frames
    }
}

impl Eq for ClassMapping {}

impl fmt::Debug for ClassMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassMapping")
            .field("names", &self.names)
            .field("comment", &self.comment)
            .field("fields", &self.fields)
            .field("methods", &self.methods)
            .field("inline_frames", &self.inline_frames)
            .finish()
    }
}

impl ClassMapping {
    fn new(names: Names) -> Self {
        Self {
            names,
            comment: None,
            fields: Vec::new(),
            methods: Vec::new(),
            inline_frames: Vec::new(),
            field_index: HashMap::new(),
            method_index: HashMap::new(),
        }
    }

    /// The class name (internal form) in namespace `ns` (by index).
    pub fn name(&self, ns: usize) -> Option<&str> {
        name_at(&self.names, ns)
    }

    /// The class name in the primary namespace.
    pub fn primary(&self) -> &str {
        self.name(0).unwrap_or_default()
    }

    /// All names, one per namespace.
    pub fn names(&self) -> &[Option<String>] {
        &self.names
    }

    /// The documentation comment (tiny v2).
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// Replaces the comment.
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }

    /// Fields, in insertion order.
    pub fn fields(&self) -> &[FieldMapping] {
        &self.fields
    }

    /// Methods, in insertion order.
    pub fn methods(&self) -> &[MethodMapping] {
        &self.methods
    }

    /// ProGuard inline frames listed under this class.
    pub fn inline_frames(&self) -> &[InlineFrame] {
        &self.inline_frames
    }

    /// Mutable access to a field by position, for setting its comment.
    pub fn field_at_mut(&mut self, index: usize) -> Option<&mut FieldMapping> {
        self.fields.get_mut(index)
    }

    /// Mutable access to a method by position, for comments, parameters and variables.
    pub fn method_at_mut(&mut self, index: usize) -> Option<&mut MethodMapping> {
        self.methods.get_mut(index)
    }

    /// Looks a field up by primary name. With `Some(desc)`, the field with that descriptor
    /// (or, failing that, one recorded without a descriptor); with `None`, the first field of
    /// that name in insertion order.
    pub fn field(&self, name: &str, desc: Option<&str>) -> Option<&FieldMapping> {
        self.field_position(name, desc).map(|i| &self.fields[i])
    }

    /// Every field with this primary name (several when obfuscation reused a name with
    /// different types).
    pub fn fields_named<'s>(&'s self, name: &str) -> impl Iterator<Item = &'s FieldMapping> + 's {
        self.field_index
            .get(name)
            .into_iter()
            .flatten()
            .map(|&i| &self.fields[i])
    }

    /// Looks a method up by primary name and descriptor.
    pub fn method(&self, name: &str, desc: &str) -> Option<&MethodMapping> {
        self.method_position(name, desc).map(|i| &self.methods[i])
    }

    /// Every method with this primary name (its overloads).
    pub fn methods_named<'s>(&'s self, name: &str) -> impl Iterator<Item = &'s MethodMapping> + 's {
        self.method_index
            .get(name)
            .into_iter()
            .flatten()
            .map(|&i| &self.methods[i])
    }

    pub(crate) fn field_position(&self, name: &str, desc: Option<&str>) -> Option<usize> {
        let candidates = self.field_index.get(name)?;
        match desc {
            Some(d) => candidates
                .iter()
                .copied()
                .find(|&i| self.fields[i].desc.as_deref() == Some(d))
                .or_else(|| {
                    candidates
                        .iter()
                        .copied()
                        .find(|&i| self.fields[i].desc.is_none())
                }),
            None => candidates.first().copied(),
        }
    }

    pub(crate) fn method_position(&self, name: &str, desc: &str) -> Option<usize> {
        self.method_index
            .get(name)?
            .iter()
            .copied()
            .find(|&i| self.methods[i].desc == desc)
    }

    fn rebuild_index(&mut self) {
        self.field_index.clear();
        self.method_index.clear();
        for (i, f) in self.fields.iter().enumerate() {
            self.field_index
                .entry(f.primary().to_owned())
                .or_default()
                .push(i);
        }
        for (i, m) in self.methods.iter().enumerate() {
            self.method_index
                .entry(m.primary().to_owned())
                .or_default()
                .push(i);
        }
    }
}

/// What [`MappingSet::filter`] asks about.
#[derive(Debug, Clone, Copy)]
pub enum Entry<'a> {
    /// A package mapping.
    Package(&'a PackageMapping),
    /// A class. Dropping it drops its members.
    Class(&'a ClassMapping),
    /// A field of the given class.
    Field(&'a ClassMapping, &'a FieldMapping),
    /// A method of the given class (with its parameters and variables).
    Method(&'a ClassMapping, &'a MethodMapping),
}

/// How many entries a set holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Packages.
    pub packages: usize,
    /// Classes.
    pub classes: usize,
    /// Fields.
    pub fields: usize,
    /// Methods.
    pub methods: usize,
    /// Method parameters.
    pub params: usize,
    /// Local variables.
    pub vars: usize,
}

/// A set of mappings between named namespaces.
///
/// ```
/// use jarrow_mappings::{names, FieldMapping, MappingSet};
///
/// let mut set = MappingSet::new(["obf", "intermediary"])?;
/// let class = set.add_class(names(["a", "net/minecraft/class_1"]))?;
/// set.add_field(class, FieldMapping::new(names(["b", "field_2"]), Some("I".into())))?;
///
/// let mapper = set.mapper("obf", "intermediary")?;
/// assert_eq!(mapper.map_class("a"), Some("net/minecraft/class_1"));
/// assert_eq!(mapper.map_field("a", "b", Some("I")), Some("field_2"));
/// # Ok::<(), jarrow_mappings::Error>(())
/// ```
#[derive(Clone)]
pub struct MappingSet {
    namespaces: Vec<String>,
    classes: Vec<ClassMapping>,
    packages: Vec<PackageMapping>,
    properties: Vec<(String, Option<String>)>,
    class_index: HashMap<String, usize>,
}

impl PartialEq for MappingSet {
    fn eq(&self, other: &Self) -> bool {
        self.namespaces == other.namespaces
            && self.classes == other.classes
            && self.packages == other.packages
            && self.properties == other.properties
    }
}

impl Eq for MappingSet {}

impl fmt::Debug for MappingSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MappingSet")
            .field("namespaces", &self.namespaces)
            .field("properties", &self.properties)
            .field("packages", &self.packages)
            .field("classes", &self.classes)
            .finish()
    }
}

impl MappingSet {
    /// An empty set over these namespaces. The first one is primary. Namespace names must be
    /// non-empty, unique and free of whitespace.
    pub fn new<I, S>(namespaces: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let namespaces: Vec<String> = namespaces
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        if namespaces.is_empty() {
            return Err(Error::NamespaceCount {
                expected: 1,
                found: 0,
            });
        }
        for (i, ns) in namespaces.iter().enumerate() {
            if ns.is_empty() || ns.chars().any(char::is_whitespace) {
                return Err(Error::UnknownNamespace(ns.clone()));
            }
            if namespaces[..i].contains(ns) {
                return Err(Error::DuplicateNamespace(ns.clone()));
            }
        }
        Ok(Self {
            namespaces,
            classes: Vec::new(),
            packages: Vec::new(),
            properties: Vec::new(),
            class_index: HashMap::new(),
        })
    }

    /// The namespaces, primary first.
    pub fn namespaces(&self) -> &[String] {
        &self.namespaces
    }

    /// The primary namespace.
    pub fn primary_namespace(&self) -> &str {
        &self.namespaces[0]
    }

    /// The index of a namespace.
    pub fn namespace_index(&self, ns: &str) -> Option<usize> {
        self.namespaces.iter().position(|n| n == ns)
    }

    pub(crate) fn require_namespace(&self, ns: &str) -> Result<usize> {
        self.namespace_index(ns)
            .ok_or_else(|| Error::UnknownNamespace(ns.to_owned()))
    }

    /// Renames a namespace, for example intermediary's `official` column to `obf`.
    pub fn rename_namespace(&mut self, from: &str, to: &str) -> Result<()> {
        let i = self.require_namespace(from)?;
        if from == to {
            return Ok(());
        }
        if to.is_empty() || to.chars().any(char::is_whitespace) {
            return Err(Error::UnknownNamespace(to.to_owned()));
        }
        if self.namespace_index(to).is_some() {
            return Err(Error::DuplicateNamespace(to.to_owned()));
        }
        self.namespaces[i] = to.to_owned();
        Ok(())
    }

    /// File-level properties (tiny v2 header properties other than `escaped-names`).
    pub fn properties(&self) -> &[(String, Option<String>)] {
        &self.properties
    }

    /// Adds or replaces a file-level property.
    pub fn set_property(&mut self, key: impl Into<String>, value: Option<String>) {
        let key = key.into();
        match self.properties.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = value,
            None => self.properties.push((key, value)),
        }
    }

    /// Classes, in insertion order.
    pub fn classes(&self) -> &[ClassMapping] {
        &self.classes
    }

    /// Packages, in insertion order.
    pub fn packages(&self) -> &[PackageMapping] {
        &self.packages
    }

    /// A class by primary name.
    pub fn class(&self, name: &str) -> Option<&ClassMapping> {
        self.class_position(name).map(|i| &self.classes[i])
    }

    /// The position of a class (by primary name) in [`MappingSet::classes`].
    pub fn class_position(&self, name: &str) -> Option<usize> {
        self.class_index.get(name).copied()
    }

    /// A class by position.
    pub fn class_at(&self, index: usize) -> Option<&ClassMapping> {
        self.classes.get(index)
    }

    /// Mutable access to a class by position, for comments and member comments.
    pub fn class_at_mut(&mut self, index: usize) -> Option<&mut ClassMapping> {
        self.classes.get_mut(index)
    }

    /// A field by primary owner, name and optional descriptor (see [`ClassMapping::field`]).
    pub fn field(&self, owner: &str, name: &str, desc: Option<&str>) -> Option<&FieldMapping> {
        self.class(owner)?.field(name, desc)
    }

    /// A method by primary owner, name and descriptor.
    pub fn method(&self, owner: &str, name: &str, desc: &str) -> Option<&MethodMapping> {
        self.class(owner)?.method(name, desc)
    }

    /// Counts the entries.
    pub fn stats(&self) -> Stats {
        let mut s = Stats {
            packages: self.packages.len(),
            classes: self.classes.len(),
            ..Stats::default()
        };
        for c in &self.classes {
            s.fields += c.fields.len();
            s.methods += c.methods.len();
            for m in &c.methods {
                s.params += m.params.len();
                s.vars += m.vars.len();
            }
        }
        s
    }

    /// Adds a class, or merges it into the class with the same primary name. Returns its
    /// position. Merging fills missing names and fails with [`Error::Conflict`] when two
    /// names differ.
    pub fn add_class(&mut self, names: Names) -> Result<usize> {
        let names = normalise(names);
        check_count(names.len(), self.namespaces.len())?;
        let Some(primary) = names[0].clone() else {
            return Err(Error::MissingPrimaryName { what: "class" });
        };
        match self.class_index.get(&primary) {
            Some(&i) => {
                merge_names(
                    &mut self.classes[i].names,
                    names,
                    "class",
                    &primary,
                    &self.namespaces,
                )?;
                Ok(i)
            }
            None => {
                self.classes.push(ClassMapping::new(names));
                let i = self.classes.len() - 1;
                self.class_index.insert(primary, i);
                Ok(i)
            }
        }
    }

    /// Adds a field to the class at `class`, or merges it into the field with the same name
    /// and descriptor. Returns the field's position in the class.
    pub fn add_field(&mut self, class: usize, field: FieldMapping) -> Result<usize> {
        check_count(field.names.len(), self.namespaces.len())?;
        if field.names[0].is_none() {
            return Err(Error::MissingPrimaryName { what: "field" });
        }
        if let Some(d) = &field.desc {
            desc::validate_field_desc(d)?;
        }
        let namespaces = &self.namespaces;
        let c = self.classes.get_mut(class).ok_or(Error::OutOfRange {
            what: "class",
            index: class,
        })?;
        let key = format!("{}.{}", c.primary(), field.primary());
        let existing = match &field.desc {
            Some(_) => c.field_position(field.primary(), field.desc.as_deref()),
            None => match c.field_index.get(field.primary()).map(Vec::as_slice) {
                None | Some([]) => None,
                Some([one]) => Some(*one),
                Some(_) => {
                    return Err(Error::Conflict {
                        what: "field",
                        key,
                        namespace: namespaces[0].clone(),
                        existing: "several descriptors".to_owned(),
                        other: "no descriptor".to_owned(),
                    });
                }
            },
        };
        match existing {
            Some(i) => {
                let target = &mut c.fields[i];
                merge_names(&mut target.names, field.names, "field", &key, namespaces)?;
                fill(&mut target.desc, field.desc);
                fill(&mut target.comment, field.comment);
                Ok(i)
            }
            None => {
                let name = field.primary().to_owned();
                c.fields.push(field);
                let i = c.fields.len() - 1;
                c.field_index.entry(name).or_default().push(i);
                Ok(i)
            }
        }
    }

    /// Adds a method to the class at `class`, or merges it into the method with the same
    /// name and descriptor (parameters by index, variables by slot, line ranges appended).
    /// Returns the method's position in the class.
    pub fn add_method(&mut self, class: usize, method: MethodMapping) -> Result<usize> {
        check_count(method.names.len(), self.namespaces.len())?;
        if method.names[0].is_none() {
            return Err(Error::MissingPrimaryName { what: "method" });
        }
        desc::validate_method_desc(&method.desc)?;
        let namespaces = &self.namespaces;
        let c = self.classes.get_mut(class).ok_or(Error::OutOfRange {
            what: "class",
            index: class,
        })?;
        let key = format!("{}.{}{}", c.primary(), method.primary(), method.desc);
        let MethodMapping {
            names,
            desc,
            comment,
            params,
            vars,
            lines,
            is_static,
        } = method;
        let i = match c.method_position(name_at(&names, 0).unwrap_or_default(), &desc) {
            Some(i) => {
                merge_names(&mut c.methods[i].names, names, "method", &key, namespaces)?;
                i
            }
            None => {
                let name = name_at(&names, 0).unwrap_or_default().to_owned();
                c.methods.push(MethodMapping::new(names, desc));
                let i = c.methods.len() - 1;
                c.method_index.entry(name).or_default().push(i);
                i
            }
        };
        let target = &mut c.methods[i];
        fill(&mut target.comment, comment);
        target.is_static |= is_static;
        for line in lines {
            if !target.lines.contains(&line) {
                target.lines.push(line);
            }
        }
        for p in params {
            target.merge_param(p, namespaces, &key)?;
        }
        for v in vars {
            target.merge_var(v, namespaces, &key)?;
        }
        Ok(i)
    }

    /// Adds a parameter to a method (merged by local-variable index); returns its position.
    pub fn add_param(&mut self, class: usize, method: usize, param: ParamMapping) -> Result<usize> {
        let namespaces = &self.namespaces;
        let c = self.classes.get_mut(class).ok_or(Error::OutOfRange {
            what: "class",
            index: class,
        })?;
        let key = format!("{}#{method}", c.primary());
        let m = c.methods.get_mut(method).ok_or(Error::OutOfRange {
            what: "method",
            index: method,
        })?;
        m.merge_param(param, namespaces, &key)
    }

    /// Adds a local variable to a method (merged by slot); returns its position.
    pub fn add_var(&mut self, class: usize, method: usize, var: VarMapping) -> Result<usize> {
        let namespaces = &self.namespaces;
        let c = self.classes.get_mut(class).ok_or(Error::OutOfRange {
            what: "class",
            index: class,
        })?;
        let key = format!("{}#{method}", c.primary());
        let m = c.methods.get_mut(method).ok_or(Error::OutOfRange {
            what: "method",
            index: method,
        })?;
        m.merge_var(var, namespaces, &key)
    }

    /// Marks a method static (TSRG2).
    pub(crate) fn set_static(&mut self, class: usize, method: usize) {
        if let Some(m) = self
            .classes
            .get_mut(class)
            .and_then(|c| c.methods.get_mut(method))
        {
            m.is_static = true;
        }
    }

    /// Records a ProGuard inline frame under a class.
    pub fn add_inline_frame(&mut self, class: usize, frame: InlineFrame) -> Result<()> {
        desc::validate_method_desc(&frame.desc)?;
        let c = self.classes.get_mut(class).ok_or(Error::OutOfRange {
            what: "class",
            index: class,
        })?;
        if !c.inline_frames.contains(&frame) {
            c.inline_frames.push(frame);
        }
        Ok(())
    }

    /// Adds a package mapping, or merges it into the package with the same primary name.
    pub fn add_package(&mut self, names: Names) -> Result<usize> {
        let names = normalise(names);
        check_count(names.len(), self.namespaces.len())?;
        let Some(primary) = names[0].clone() else {
            return Err(Error::MissingPrimaryName { what: "package" });
        };
        match self.packages.iter().position(|p| p.primary() == primary) {
            Some(i) => {
                merge_names(
                    &mut self.packages[i].names,
                    names,
                    "package",
                    &primary,
                    &self.namespaces,
                )?;
                Ok(i)
            }
            None => {
                self.packages.push(PackageMapping { names });
                Ok(self.packages.len() - 1)
            }
        }
    }

    /// Merges every entry of `other`, which must have the same namespaces in the same order
    /// (for example Mojang's client and server mappings of one version).
    pub fn merge(&mut self, other: &MappingSet) -> Result<()> {
        if self.namespaces != other.namespaces {
            return Err(Error::UnknownNamespace(other.namespaces.join(",")));
        }
        for p in &other.packages {
            self.add_package(p.names.clone())?;
        }
        for c in &other.classes {
            let ci = self.add_class(c.names.clone())?;
            if let Some(comment) = &c.comment
                && self.classes[ci].comment.is_none()
            {
                self.classes[ci].comment = Some(comment.clone());
            }
            for f in &c.fields {
                self.add_field(ci, f.clone())?;
            }
            for m in &c.methods {
                self.add_method(ci, m.clone())?;
            }
            for frame in &c.inline_frames {
                self.add_inline_frame(ci, frame.clone())?;
            }
        }
        Ok(())
    }

    /// A copy that keeps only the entries `keep` accepts. Dropping a class drops its members;
    /// a method keeps its parameters and variables.
    pub fn filter(&self, mut keep: impl FnMut(Entry<'_>) -> bool) -> MappingSet {
        let mut out = MappingSet {
            namespaces: self.namespaces.clone(),
            classes: Vec::new(),
            packages: self
                .packages
                .iter()
                .filter(|p| keep(Entry::Package(p)))
                .cloned()
                .collect(),
            properties: self.properties.clone(),
            class_index: HashMap::new(),
        };
        for c in &self.classes {
            if !keep(Entry::Class(c)) {
                continue;
            }
            let fields = c
                .fields
                .iter()
                .filter(|f| keep(Entry::Field(c, f)))
                .cloned()
                .collect();
            let methods = c
                .methods
                .iter()
                .filter(|m| keep(Entry::Method(c, m)))
                .cloned()
                .collect();
            let mut copy = ClassMapping {
                names: c.names.clone(),
                comment: c.comment.clone(),
                fields,
                methods,
                inline_frames: c.inline_frames.clone(),
                field_index: HashMap::new(),
                method_index: HashMap::new(),
            };
            copy.rebuild_index();
            out.class_index
                .insert(copy.primary().to_owned(), out.classes.len());
            out.classes.push(copy);
        }
        out
    }

    /// A copy over a subset or reordering of the namespaces; the first one becomes primary.
    ///
    /// Member descriptors are rewritten into the new primary namespace through the class
    /// names of this set (classes without a name there, such as JDK classes, keep theirs).
    /// Classes, fields, methods and packages without a name in the new primary namespace are
    /// left out, because they cannot be looked up from it. ProGuard inline frames are kept
    /// only when the namespaces stay exactly as they are.
    pub fn project(&self, namespaces: &[&str]) -> Result<MappingSet> {
        let mut picks = Vec::with_capacity(namespaces.len());
        for ns in namespaces {
            picks.push(self.require_namespace(ns)?);
        }
        let mut out = MappingSet::new(namespaces)?;
        out.properties = self.properties.clone();
        if picks.iter().copied().eq(0..self.namespaces.len()) {
            out.classes = self.classes.clone();
            out.packages = self.packages.clone();
            out.class_index = self.class_index.clone();
            return Ok(out);
        }
        let primary = picks[0];
        let class_map: HashMap<&str, &str> = if primary == 0 {
            HashMap::new()
        } else {
            self.classes
                .iter()
                .filter_map(|c| Some((c.primary(), c.name(primary)?)))
                .collect()
        };
        let remap = |name: &str| class_map.get(name).copied();
        let pick = |names: &[Option<String>]| -> Names {
            picks.iter().map(|&i| names[i].clone()).collect()
        };
        for p in &self.packages {
            if p.name(primary).is_some() {
                out.add_package(pick(&p.names))?;
            }
        }
        for c in &self.classes {
            if c.name(primary).is_none() {
                continue;
            }
            let ci = out.add_class(pick(&c.names))?;
            if c.comment.is_some() && out.classes[ci].comment.is_none() {
                out.classes[ci].comment = c.comment.clone();
            }
            for f in &c.fields {
                if f.name(primary).is_none() {
                    continue;
                }
                let desc = match &f.desc {
                    Some(d) if primary != 0 => Some(desc::remap_field_desc(d, remap)?),
                    other => other.clone(),
                };
                let mut field = FieldMapping::new(pick(&f.names), desc);
                field.comment = f.comment.clone();
                out.add_field(ci, field)?;
            }
            for m in &c.methods {
                if m.name(primary).is_none() {
                    continue;
                }
                let desc = if primary == 0 {
                    m.desc.clone()
                } else {
                    desc::remap_method_desc(&m.desc, remap)?
                };
                let mut method = MethodMapping::new(pick(&m.names), desc);
                method.comment = m.comment.clone();
                method.lines = m.lines.clone();
                method.is_static = m.is_static;
                method.params = m
                    .params
                    .iter()
                    .map(|p| ParamMapping {
                        lv_index: p.lv_index,
                        names: pick(&p.names),
                        comment: p.comment.clone(),
                    })
                    .collect();
                method.vars = m
                    .vars
                    .iter()
                    .map(|v| VarMapping {
                        lv_index: v.lv_index,
                        start_offset: v.start_offset,
                        lvt_index: v.lvt_index,
                        names: pick(&v.names),
                        comment: v.comment.clone(),
                    })
                    .collect();
                out.add_method(ci, method)?;
            }
        }
        Ok(out)
    }

    /// A copy with `ns` as the primary namespace; the others keep their order.
    pub fn with_primary(&self, ns: &str) -> Result<MappingSet> {
        let i = self.require_namespace(ns)?;
        let mut order: Vec<&str> = vec![ns];
        order.extend(
            self.namespaces
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, n)| n.as_str()),
        );
        self.project(&order)
    }

    /// The inverse: the namespaces in reverse order, so `a -> b` becomes `b -> a`. Entries
    /// with no name in the last namespace are left out (see [`MappingSet::project`]).
    pub fn invert(&self) -> Result<MappingSet> {
        let order: Vec<&str> = self.namespaces.iter().rev().map(String::as_str).collect();
        self.project(&order)
    }
}

fn check_count(found: usize, expected: usize) -> Result<()> {
    if found == expected {
        Ok(())
    } else {
        Err(Error::NamespaceCount { expected, found })
    }
}

fn fill<T>(slot: &mut Option<T>, value: Option<T>) {
    if slot.is_none() {
        *slot = value;
    }
}

fn merge_names(
    existing: &mut Names,
    incoming: Names,
    what: &'static str,
    key: &str,
    namespaces: &[String],
) -> Result<()> {
    for (i, name) in incoming.into_iter().enumerate() {
        match (&existing[i], name) {
            (Some(a), Some(b)) if *a != b => {
                return Err(Error::Conflict {
                    what,
                    key: key.to_owned(),
                    namespace: namespaces[i].clone(),
                    existing: a.clone(),
                    other: b,
                });
            }
            (None, Some(b)) => existing[i] = Some(b),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MappingSet {
        let mut set = MappingSet::new(["obf", "named"]).unwrap();
        let a = set.add_class(names(["a", "pkg/Alpha"])).unwrap();
        let b = set.add_class(names(["b", "pkg/Beta"])).unwrap();
        set.add_field(
            a,
            FieldMapping::new(names(["x", "count"]), Some("I".into())),
        )
        .unwrap();
        set.add_field(
            a,
            FieldMapping::new(names(["y", "beta"]), Some("Lb;".into())),
        )
        .unwrap();
        set.add_method(a, MethodMapping::new(names(["m", "make"]), "(Lb;)La;"))
            .unwrap();
        set.add_method(b, MethodMapping::new(names(["m", "run"]), "()V"))
            .unwrap();
        set
    }

    #[test]
    fn new_rejects_bad_namespaces() {
        assert!(MappingSet::new(Vec::<String>::new()).is_err());
        assert!(MappingSet::new(["a", "a"]).is_err());
        assert!(MappingSet::new(["a", ""]).is_err());
        assert!(MappingSet::new(["a b"]).is_err());
    }

    #[test]
    fn lookups() {
        let set = sample();
        assert_eq!(set.class("a").unwrap().name(1), Some("pkg/Alpha"));
        assert_eq!(set.field("a", "x", None).unwrap().name(1), Some("count"));
        assert_eq!(
            set.field("a", "x", Some("I")).unwrap().name(1),
            Some("count")
        );
        assert!(set.field("a", "x", Some("J")).is_none());
        assert_eq!(
            set.method("a", "m", "(Lb;)La;").unwrap().name(1),
            Some("make")
        );
        assert!(set.method("a", "m", "()V").is_none());
        assert_eq!(set.stats().methods, 2);
    }

    #[test]
    fn merging_fills_and_detects_conflicts() {
        let mut set = MappingSet::new(["obf", "named"]).unwrap();
        let a = set.add_class(names(["a", ""])).unwrap();
        assert_eq!(set.add_class(names(["a", "pkg/A"])).unwrap(), a);
        assert_eq!(set.class("a").unwrap().name(1), Some("pkg/A"));
        let err = set.add_class(names(["a", "pkg/Other"])).unwrap_err();
        assert!(
            matches!(err, Error::Conflict { what: "class", .. }),
            "{err}"
        );
        assert!(set.add_class(names(["", "x"])).is_err());
        assert!(set.add_class(names(["only-one"])).is_err());

        set.add_field(a, FieldMapping::new(names(["f", ""]), None))
            .unwrap();
        set.add_field(
            a,
            FieldMapping::new(names(["f", "field"]), Some("I".into())),
        )
        .unwrap();
        assert_eq!(set.class("a").unwrap().fields().len(), 1);
        assert_eq!(
            set.field("a", "f", Some("I")).unwrap().name(1),
            Some("field")
        );
        set.add_field(
            a,
            FieldMapping::new(names(["f", "other"]), Some("J".into())),
        )
        .unwrap();
        assert!(
            set.add_field(a, FieldMapping::new(names(["f", ""]), None))
                .is_err()
        );
        assert!(
            set.add_field(a, FieldMapping::new(names(["g", "g"]), Some("Q".into())))
                .is_err()
        );
        assert!(
            set.add_method(a, MethodMapping::new(names(["m", "m"]), "I"))
                .is_err()
        );
    }

    #[test]
    fn projection_rewrites_descriptors() {
        let set = sample();
        let inv = set.invert().unwrap();
        assert_eq!(inv.namespaces(), ["named", "obf"]);
        let m = inv
            .method("pkg/Alpha", "make", "(Lpkg/Beta;)Lpkg/Alpha;")
            .unwrap();
        assert_eq!(m.name(1), Some("m"));
        assert_eq!(
            inv.field("pkg/Alpha", "beta", Some("Lpkg/Beta;"))
                .unwrap()
                .name(1),
            Some("y")
        );
        assert_eq!(inv.invert().unwrap(), set);
        assert_eq!(set.project(&["obf", "named"]).unwrap(), set);
        assert!(set.project(&["obf", "obf"]).is_err());
        assert!(set.project(&["nope"]).is_err());
        let only = set.project(&["named"]).unwrap();
        assert_eq!(only.namespaces(), ["named"]);
        assert_eq!(only.stats().classes, 2);
    }

    #[test]
    fn projection_drops_unnamed_entries() {
        let mut set = MappingSet::new(["obf", "named"]).unwrap();
        let a = set.add_class(names(["a", "pkg/A"])).unwrap();
        set.add_class(names(["b", ""])).unwrap();
        set.add_method(a, MethodMapping::new(names(["<init>", ""]), "()V"))
            .unwrap();
        let inv = set.invert().unwrap();
        assert_eq!(inv.stats().classes, 1);
        assert_eq!(inv.stats().methods, 0);
    }

    #[test]
    fn filter_and_rename() {
        let mut set = sample();
        let kept = set.filter(|e| !matches!(e, Entry::Field(_, f) if f.primary() == "x"));
        assert!(kept.field("a", "x", None).is_none());
        assert!(kept.field("a", "y", None).is_some());
        let no_b = set.filter(|e| !matches!(e, Entry::Class(c) if c.primary() == "b"));
        assert!(no_b.class("b").is_none());
        set.rename_namespace("obf", "official").unwrap();
        assert_eq!(set.namespaces(), ["official", "named"]);
        assert!(set.rename_namespace("official", "named").is_err());
        assert!(set.rename_namespace("missing", "x").is_err());
    }

    #[test]
    fn merge_sets() {
        let mut a = sample();
        let mut b = MappingSet::new(["obf", "named"]).unwrap();
        let c = b.add_class(names(["c", "pkg/Gamma"])).unwrap();
        b.add_method(
            c,
            MethodMapping::new(names(["n", "go"]), "()V").with_lines(LineRange {
                start: 1,
                end: 2,
                original_start: None,
                original_end: None,
            }),
        )
        .unwrap();
        a.merge(&b).unwrap();
        a.merge(&b).unwrap();
        assert_eq!(a.stats().classes, 3);
        assert_eq!(a.method("c", "n", "()V").unwrap().lines().len(), 1);
        let other = MappingSet::new(["named", "obf"]).unwrap();
        assert!(a.merge(&other).is_err());
    }
}
