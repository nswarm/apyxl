#[derive(Debug, Copy, Clone, macros::ValidateFeatures)]
pub enum Feature {
    Namespace,
    NestedNamespace,
    NamespaceAttributeFlag,
    NamespaceAttributeList,
    NamespaceAttributeMap,
    NamespaceAttributeMultiple,
}

impl Feature {
    pub fn to_pyx_str(self) -> &'static str {
        match self {
            Feature::Namespace => "namespace ns {}",
            Feature::NestedNamespace => "namespace outer { namespace inner {} }",
            Feature::NamespaceAttributeFlag => "#[some_attr] namespace ns {}",
            Feature::NamespaceAttributeList => "#[some_attr(a, b, c)] namespace ns {}",
            Feature::NamespaceAttributeMap => "#[some_attr(a=x, b=y)] namespace ns {}",
            Feature::NamespaceAttributeMultiple => {
                "#[attr1, attr2(a, b), attr3(k=v)] namespace ns {}"
            }
        }
    }
}
