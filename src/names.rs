use crate::error::{ParseError, err};
use indexmap::IndexSet;
use rustc_hash::FxBuildHasher;
use std::borrow::Cow;

/// `Lookup` is a string interner for parameter and property names. It starts out
#[derive(Default, Debug)]
pub struct Lookup {
    parms: NameIds,
    props: NameIds,
}
impl Lookup {
    pub fn new() -> Self {
        Lookup {
            parms: NameIds::known_ids(crate::parameters::NAMES),
            props: NameIds::known_ids(crate::properties::NAMES),
        }
    }
    #[inline]
    pub fn known_parameter(&mut self, name: &'static str) -> NameResult<ParameterId> {
        self.parms.known_id(name).map(|u| ParameterId(u))
    }
    #[inline]
    pub fn parameter_id(&mut self, name: &str) -> NameResult<ParameterId> {
        self.parms.id(name).map(|u| ParameterId(u))
    }
    #[inline]
    pub fn parameter_name(&self, id: ParameterId) -> Option<&Key> {
        self.parms.name(id.0)
    }
    #[inline]
    pub fn known_property(&mut self, name: &'static str) -> NameResult<PropertyId> {
        self.parms.known_id(name).map(|u| PropertyId(u))
    }
    #[inline]
    pub fn property_id(&mut self, name: &str) -> NameResult<PropertyId> {
        self.parms.id(name).map(|u| PropertyId(u))
    }
    #[inline]
    pub fn property_name(&self, id: PropertyId) -> Option<&Key> {
        self.parms.name(id.0)
    }
}
pub type NameResult<T> = Result<T, ParseError>;
pub struct ParameterId(pub(crate) usize);
pub struct PropertyId(pub(crate) usize);

type Key = Cow<'static, str>;
type NameSet = IndexSet<Key, FxBuildHasher>;
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NameIds(NameSet);
impl NameIds {
    #[must_use]
    fn known_ids<const N: usize>(names: [&'static str; N]) -> Self {
        let mut set = NameSet::with_capacity_and_hasher(N + 50, FxBuildHasher);
        for name in names {
            set.insert(Cow::Borrowed(name));
        }
        NameIds(set)
    }
    pub fn known_id(&mut self, name: &'static str) -> Result<usize, ParseError> {
        match well_formed(name) {
            WellFormed::Uppercase => {
                let (id, _) = self.0.insert_full(Cow::Borrowed(name));
                Ok(id)
            }
            WellFormed::Lowercase => {
                Err(err!("Known names must be uppercase, but '{name}' isn't."))
            }
            WellFormed::No => Err(err!("Not a valid name: '{name}'")),
        }
    }
    pub fn id(&mut self, name: &str) -> Result<usize, ParseError> {
        if let Some((id_found, _)) = self.0.get_full(dbg!(name)) {
            Ok(id_found)
        } else {
            let key = match well_formed(name) {
                WellFormed::Uppercase => Cow::from(name.to_string()),
                WellFormed::Lowercase => Cow::from(name.to_ascii_uppercase()),
                WellFormed::No => return Err(err!("Not a valid name: '{name}'")),
            };
            let (id_new, _) = self.0.insert_full(key);
            Ok(id_new)
        }
    }
    #[must_use]
    pub fn name(&self, id: usize) -> Option<&Key> {
        self.0.get_index(id)
    }
}
enum WellFormed {
    Uppercase,
    Lowercase,
    No,
}
fn well_formed(nym: &str) -> WellFormed {
    if nym.is_empty() {
        return WellFormed::No;
    }
    let mut ok = WellFormed::Uppercase;
    for b in nym.as_bytes() {
        match b {
            b'a'..=b'z' => ok = WellFormed::Lowercase,
            b'A'..=b'Z' | b'0'..=b'9' | b'-' => {}
            _ => return WellFormed::No,
        }
    }
    ok
}

#[cfg(test)]
mod test {
    use super::*;

    fn empty() -> NameIds {
        NameIds::known_ids([])
    }

    #[test]
    fn known() {
        assert!(empty().known_id("foo").unwrap_err().0.contains("upper"));
        assert!(empty().known_id("").unwrap_err().0.contains("valid"));
        assert!(empty().known_id("f o o").unwrap_err().0.contains("valid"));
        assert_eq!(empty().known_id("FOO").unwrap(), 0);
    }
    #[test]
    fn fresh_upper() {
        let mut names = empty();
        let id = names.id("FOO").unwrap();
        assert_eq!(names.name(id).unwrap(), "FOO");
    }
    #[test]
    fn fresh_lower() {
        let mut names = empty();
        let id = names.id("foo").unwrap();
        assert_eq!(names.name(id).unwrap(), "FOO");
    }
    #[test]
    fn fresh_invalid() {
        let mut names = empty();
        let orig = names.clone();
        assert!(names.id("").is_err());
        assert_eq!(names, orig);
    }
}
