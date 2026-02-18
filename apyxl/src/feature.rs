#[derive(Debug, Copy, Clone, macros::ValidateFeatures)]
pub enum Feature {
    Namespace,
    NestedNamespace,
}

impl Feature {
    pub fn to_pyx_str(self) -> &'static str {
        match self {
            Feature::Namespace => "namespace ns {}",
            Feature::NestedNamespace => "namespace outer { namespace inner {} }",
        }
    }
}
