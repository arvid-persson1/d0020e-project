#![expect(clippy::empty_structs_with_brackets)]

// Define a schema class as one of the following:
// - A Rust mapping (type alias) of one of Schema.org's primitive types.
// - A class described by Schema.org comprised of other schema classes.
// - An externally incldued alternative, e.g. `isbn::Isbn` replacing `Text` when applicable.
// All fields should be of one of these types:
// `Option<T>`, where `T` is another schema class that does not reference this class.
// `Option<Box<T>>`, where `T` is `Self` or another schema class that also references this one.
// `Option<Box<[T]>`, as above, but in special cases. TODO: When should properties be plural?

// NOTE: Primitive data types should map to native Rust types.
pub type Text = String;

// Example of a schema class. This one is of note as it inherits from *two* other schema classes.
// TODO: Can comparison traits and `Hash` be derived as external types may not implement them? Is
// it possible to derive these traits only on classes where applicable?
#[derive(Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EducationalOrganization {
    // TODO: This property should clearly be plural. How to handle plural properties?
    // NOTE: Inverse property of `Person::alumni_of`. Hence, It has to be boxed to limit struct
    // size (avoid recursive definitions).
    pub alumni: Option<Box<Person>>,
    // NOTE: Camel cased names should be renamed using snake case.
    pub opening_hours: Option<Text>,
    pub address: Option<Address>,
    pub nonprofit_status: Option<NonprofitType>,
    /* TODO */
}

#[derive(Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Person {/* TODO */}
#[derive(Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PostalAddress {/* TODO */}

// TODO: Does there ever exist two properties of the same name resident on different classes, but
// with the properties of different types? I.e. could a class `A` have a property `foo` of type
// `FooA` while a class `B` has a property `foo` of type `FooB`? If this is the case, these will
// likely have to include the class name as prefix, e.g. `EducationalOrganizationAddress`, possibly
// with `Into` implementations between those that actually are identical.
// NOTE: Enumeration types cannot be `Default`.
#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Address {
    Text(Text),
    PostalAddress(PostalAddress),
}

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum NonprofitType {
    // NOTE: This is against the Rust naming conventions for acronyms (should be
    // `NlNonprofitType`). Should this be corrected aggressively or allowed?
    NLNonprofitType,
    UKNonprofitType,
    USNonprofitType,
}

// NOTE: The supertrait of all object traits.
// TODO: Seal object traits so they are only implemented on the proper class and superclasses.
pub trait ThingObj {
    /* TODO */
}

pub trait PlaceObj: ThingObj {
    // NOTE: Immutable getters should return `Option<&T>` instead of `&Option<T>`.
    fn address(&self) -> Option<&Address>;
    // NOTE: Mutable getters, however, should return `&mut Option<T>` to allow for writing to the
    // fields or e.g. `Option::take`.
    fn address_mut(&mut self) -> &mut Option<Address>;
    /* TODO */
}

pub trait CivicStructureObj: PlaceObj {
    // NOTE: `&Text` should not be turned to `&str` to allow calling e.g. `String::capacity`.
    fn opening_hours(&self) -> Option<&Text>;
    fn opening_hours_mut(&mut self) -> &mut Option<Text>;
    /* TODO */
}

pub trait OrganizationObj: ThingObj {
    fn nonprofit_status(&self) -> Option<&NonprofitType>;
    fn nonprofit_status_mut(&mut self) -> &mut Option<NonprofitType>;
    /* TODO */
}

pub trait EducationalOrganizationObj: CivicStructureObj + OrganizationObj {
    // NOTE: `Box` should be turned to reference for immutable getters.
    fn alumni(&self) -> Option<&Person>;
    // NOTE: Mutable getters should retain their "actual" type.
    fn alumni_mut(&mut self) -> &mut Option<Box<Person>>;
    /* TODO */
}

// TODO: Implement `EducationalOrganizationObj` for `EducationalOrganization`.

// NOTE: A schema class does not have to know about any of its subclasses, e.g.
// `EducationalOrganization` does not depend on `School` even though it is a superclass of it.
// Similarly, it doesn't matter to it that other schema classes might have properties of its type,
// e.g. `EducationalOrganization` does not necessarily depend on `Person` even though
// `Person::alumniOf` is of type `EducationalOrganization`. However, these properties must be boxed
// if it would otherwise create a cyclic dependency.
// TODO: How should cyclic dependencies be detected?

// TODO: Joins (#18).
