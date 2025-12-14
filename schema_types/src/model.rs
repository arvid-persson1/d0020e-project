use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct SchemaClass {
    label: String,
    // NOTE: Only labels should be stored, makes instantiation easier.
    subclass_of: Box<[String]>,
    comment: String,
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
    label: String,
    comment: String,
    range: String,
    // A tuple (class label, property label).
    inverse_of: Option<(String, String)>,
    // Superseded items should be marked as deprecated and link to their successor.
    superseded_by: Option<String>,
    /* TODO: More data available? */
}

impl PartialEq for SchemaClass {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self {
            label,
            subclass_of,
            comment,
            properties,
        } = self;
        *label == other.label && {
            // Labels should uniquely identify classes. Hence, if the labels are equal, the classes
            // should be entirely equal.
            debug_assert!(*subclass_of == other.subclass_of);
            debug_assert!(*comment == other.comment);
            debug_assert!(*properties == other.properties);
            true
        }
    }
}

impl Eq for SchemaClass {}

impl Hash for SchemaClass {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.label.hash(state);
    }
}

impl PartialEq for Property {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let Self {
            label,
            comment,
            range,
            inverse_of,
            superseded_by,
        } = self;
        *label == other.label && {
            // Labels should uniquely identify properties. Hence, if the labels are equal, the
            // properties should be entirely equal.
            debug_assert!(*comment == other.comment);
            debug_assert!(*range == other.range);
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
        self.label.hash(state);
    }
}
