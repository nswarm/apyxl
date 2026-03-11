#[derive(Debug, Copy, Clone, macros::ValidateFeatures)]
pub enum Feature {
    Namespace,
    NestedNamespace,
    NamespaceAttributeFlag,
    NamespaceAttributeList,
    NamespaceAttributeMap,
    NamespaceAttributeMultiple,
    NamespaceCommentLine,
    NamespaceCommentMultiLine,
    NamespaceCommentBlock,
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
            Feature::NamespaceCommentLine => {
                r#"
                /// comment
                namespace ns {}
                "#
            }
            Feature::NamespaceCommentMultiLine => {
                r#"
                /// line 1
                /// line 2
                namespace ns {}
                "#
            }
            Feature::NamespaceCommentBlock => {
                r#"
                /*
                multiline
                  block
                comment
                */
                namespace ns {}
                "#
            }
        }
    }
}
