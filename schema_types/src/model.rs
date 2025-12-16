use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub enum Type {
    Class {
        name: String,
        // NOTE: Only names should be stored, makes instantiation easier.
        subclass_of: Box<[String]>,
        description: String,
        // NOTE: Should only include those defined on the class directly, not on any of its
        // superclasses. These will be added during instantiation.
        properties: Box<[Property]>,
        /* TODO: More data available? */
    },
    Enumeration {
        name: String,
        description: String,
        members: Box<[EnumerationMember]>,
        // NOTE: Superclass inferred to be `Enumeration`.
        /* TODO: More data available? */
    },
}

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

#[derive(Clone, Debug)]
pub struct EnumerationMember {
    name: String,
    description: String,
    /* TODO: More data available? */
}

// Uninteresting trait implementations below.

impl PartialEq for Type {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Class {
                    name,
                    subclass_of,
                    description,
                    properties,
                },
                Self::Class {
                    name: other_name,
                    subclass_of: other_subclass_of,
                    description: other_description,
                    properties: other_properties,
                },
            ) => {
                name == other_name && {
                    // Names should uniquely identify classes. Hence, if the names are equal, the types
                    // should be entirely equal.
                    debug_assert!(subclass_of == other_subclass_of);
                    debug_assert!(description == other_description);
                    debug_assert!(properties == other_properties);
                    true
                }
            },

            (
                Self::Enumeration {
                    name,
                    description,
                    members,
                },
                Self::Enumeration {
                    name: other_name,
                    description: other_description,
                    members: other_members,
                },
            ) => {
                name == other_name && {
                    // Names should uniquely identify enumerations. Hence, if the names are equal, the types
                    // should be entirely equal.
                    debug_assert!(description == other_description);
                    debug_assert!(members == other_members);
                    true
                }
            },
            _ => false,
        }
    }
}

impl Eq for Type {}

impl Hash for Type {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (Self::Class { name, .. } | Self::Enumeration { name, .. }) = self;
        name.hash(state);
    }
}

impl PartialEq for Property {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self {
            name,
            description,
            expected_types,
            inverse_of,
            superseded_by,
        } = self;
        *name == other.name && {
            // Names should uniquely identify properties. Hence, if the names are equal, the
            // properties should be entirely equal.
            debug_assert!(*description == other.description);
            debug_assert!(*expected_types == other.expected_types);
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

impl PartialEq for EnumerationMember {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self { name, description } = self;
        *name == other.name && {
            // names should uniquely identify members. Hence, if the names are equal, the members
            // should be entirely equal.
            debug_assert!(*description == other.description);
            true
        }
    }
}

impl Eq for EnumerationMember {}

impl Hash for EnumerationMember {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
