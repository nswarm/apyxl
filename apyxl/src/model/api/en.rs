use crate::model::Attributes;

/// A single enum type in the within an [Api].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Enum<'a> {
    pub name: &'a str,
    pub values: Vec<EnumValue<'a>>,
    pub attributes: Attributes,
}

pub type EnumValueNumber = i64;

/// A single value within an [Enum].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct EnumValue<'a> {
    pub name: &'a str,
    pub number: EnumValueNumber,
    pub attributes: Attributes,
}

impl<'a> Enum<'a> {
    pub fn value(&self, name: &str) -> Option<&EnumValue<'a>> {
        self.values.iter().find(|value| value.name == name)
    }

    pub fn value_mut(&mut self, name: &str) -> Option<&mut EnumValue<'a>> {
        self.values.iter_mut().find(|value| value.name == name)
    }
}
