use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct SchemaClass {
    name: String,
    // NOTE: Only names should be stored, makes instantiation easier.
    subclass_of: Box<[String]>,
    description: String,
    // NOTE: Should only include those defined on the class directly, not on any of its supertypes.
    // These will be added during instantiation.
    properties: Box<[Property]>,
    /* TODO: More data available? */
}
//
// NOTE: Unlike the schema file, properties here are actually fields of the relevant struct. As
// such, this does not reference the class it belongs to.
#[derive(Clone, Debug)]
pub struct Property {
    name: String,
    description: String,
    // WARN: Must not be empty.
    expected_types: Box<[String]>,
    // A tuple (class name, property name).
    inverse_of: Option<(String, String)>,
    // Superseded items should be marked as deprecated and link to their successor.
    superseded_by: Option<String>,
    /* TODO: More data available? */
}

impl PartialEq for SchemaClass {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self {
            name,
            subclass_of,
            description: comment,
            properties,
        } = self;
        *name == other.name && {
            // Labels should uniquely identify classes. Hence, if the names are equal, the classes
            // should be entirely equal.
            debug_assert!(*subclass_of == other.subclass_of);
            debug_assert!(*comment == other.description);
            debug_assert!(*properties == other.properties);
            true
        }
    }
}

impl Eq for SchemaClass {}

impl Hash for SchemaClass {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Property {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self {
            name,
            description: comment,
            expected_types: range,
            inverse_of,
            superseded_by,
        } = self;
        *name == other.name && {
            // Labels should uniquely identify properties. Hence, if the names are equal, the
            // properties should be entirely equal.
            debug_assert!(*comment == other.description);
            debug_assert!(*range == other.expected_types);
            debug_assert!(*inverse_of == other.inverse_of);
            debug_assert!(*superseded_by == other.superseded_by);
            true
        }
    }
}

impl Eq for Property {}

impl Hash for Property {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
