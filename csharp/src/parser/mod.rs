use std::borrow::Cow;

use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use itertools::Itertools;
use log::debug;

use apyxl::model::entity::{EntityMut, FindEntity};
use apyxl::model::{
    entity, Api, EntityId, EntityType, Field, Namespace, NamespaceChild, Rpc, Type,
    TypeRef, UNDEFINED_NAMESPACE,
};
use apyxl::parser::error::Error;
use apyxl::parser::{error, util, Config};
use apyxl::{model, Input};

mod attributes;
mod comment;
mod dto;
mod en;
mod expr_block;
mod field;
mod is_static;
mod namespace;
mod rpc;
mod ty;
mod ty_alias;
mod visibility;

#[derive(Default)]
pub struct CSharpParser {}

impl apyxl::Parser for CSharpParser {
    fn parse<'a, I: Input + 'a>(
        &self,
        config: &'a Config,
        input: &'a mut I,
        builder: &mut model::Builder<'a>,
    ) -> Result<()> {
        for (chunk, data) in input.chunks() {
            debug!("parsing chunk {:?}", chunk.relative_file_path);

            // These must come before asmdefs, but still be appended to the top level
            // namespace aliases.
            let file_aliases = ty_alias::parser(config)
                .map(|alias| alias.map(NamespaceChild::TypeAlias))
                .padded()
                .repeated()
                .collect::<Vec<Option<NamespaceChild>>>();

            let children = file_aliases
                .then_ignore(assembly_definitions())
                .then(
                    namespace::children(config, namespace::parser(config), end().ignored())
                        .padded(),
                )
                .then_ignore(end())
                .map(|(aliases, mut children)| {
                    let mut aliases = aliases.into_iter().flatten().collect_vec();
                    children.append(&mut aliases);
                    children
                })
                .parse(data)
                .into_result()
                .map_err(|errs| {
                    let return_err = anyhow!("errors encountered while parsing: {:?}", &errs);
                    error::report_errors(chunk, data, errs.clone());
                    return_err
                })?;

            let mut api = Api {
                name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                children,
                attributes: Default::default(),
                is_virtual: false,
            };

            apply_local_ty_aliases(&mut api)?;

            builder.merge_from_chunk(api, chunk);
        }

        Ok(())
    }
}

/// Replace types in this chunk referred to by local aliases like `using X = A.B.C;`. This will
/// update the type `X` everywhere in the `api` to be `A.B.C`.
///
/// This is necessary because C# doesn't have 'real' typedefs, it only has file-local using
/// statements.
fn apply_local_ty_aliases(api: &mut Api) -> Result<()> {
    let mut type_ids = Vec::new();
    collect_type_ids(api, EntityId::default(), &mut type_ids);

    // Each pass replaces one level of indirection, so if an alias refers to another alias,
    // you need two passes. So we just loop until we stop making replacements.
    loop {
        let mut made_replacements = false;
        for type_id in &type_ids {
            let ty_parent = type_id.namespace().unwrap_or_default();
            let ty = type_ref(api, type_id.clone()).value.clone();
            let replace_ty = match find_alias_target_ty(api, ty_parent, ty) {
                None => {
                    continue;
                }
                Some(ty) => ty,
            };

            let actual_type_ref = type_ref(api, type_id.clone());
            actual_type_ref.value = replace_ty;
            made_replacements = true;
        }
        if !made_replacements {
            break;
        }
    }

    // Now remove all ty aliases because we don't want them polluting the merged API.
    remove_all_ty_aliases(api);

    Ok(())
}

fn type_ref<'a>(api: &'a mut Api, alias_id: EntityId) -> &'a mut TypeRef {
    let type_ref = api
        .find_entity_mut(alias_id)
        .expect("entity id should exist since we just collected it");
    if let EntityMut::Type(type_ref) = type_ref {
        type_ref
    } else {
        unreachable!("type checked in collect_type_ids");
    }
}

fn find_alias_target_ty(
    api: &mut Api,
    mut from_ty_parent: EntityId,
    from_ty: Type,
) -> Option<Type> {
    let rel_alias_id = if let Type::Api(alias_id) = from_ty {
        alias_id
    } else {
        return None;
    };

    loop {
        let namespace = api.find_namespace_mut(&from_ty_parent).unwrap();
        if let Some(alias) = namespace.find_ty_alias(&rel_alias_id) {
            return Some(alias.target_ty.value.clone());
        }
        from_ty_parent = from_ty_parent.parent()?
    }
}

fn remove_all_ty_aliases(namespace: &mut Namespace) {
    let is_not_ty_alias = |child: &NamespaceChild| !matches!(child, NamespaceChild::TypeAlias(_));
    namespace.children.retain(is_not_ty_alias);

    for namespace in namespace.namespaces_mut() {
        remove_all_ty_aliases(namespace);
    }
}

// I don't love this since it's duplicating iteration code that is very similar elsewhere,
// but I have a plan for a much improved hierarchy iteration refactor in the future.
fn collect_type_ids(namespace: &Namespace, namespace_id: EntityId, type_ids: &mut Vec<EntityId>) {
    let handle_field = |parent_id: &EntityId, field: &Field, type_ids: &mut Vec<EntityId>| {
        let field_id = parent_id.child(EntityType::Field, field.name).unwrap();
        type_ids.push(
            field_id
                .child(EntityType::Type, entity::subtype::TY)
                .unwrap(),
        )
    };

    let handle_rpc = |parent_id: &EntityId, rpc: &Rpc, type_ids: &mut Vec<EntityId>| {
        let rpc_id = parent_id.child(EntityType::Rpc, rpc.name).unwrap();
        for param in &rpc.params {
            handle_field(&rpc_id, param, type_ids);
        }
        if rpc.return_type.is_some() {
            type_ids.push(
                rpc_id
                    .child(EntityType::Type, entity::subtype::RETURN_TY)
                    .unwrap(),
            )
        }
    };

    for child in &namespace.children {
        match child {
            NamespaceChild::Field(field) => handle_field(&namespace_id, field, type_ids),
            NamespaceChild::Dto(dto) => {
                let dto_id = namespace_id.child(EntityType::Dto, dto.name).unwrap();
                for field in &dto.fields {
                    handle_field(&dto_id, field, type_ids);
                }
                for rpc in &dto.rpcs {
                    handle_rpc(&dto_id, rpc, type_ids);
                }
                if let Some(namespace) = dto.namespace.as_ref() {
                    collect_type_ids(namespace, dto_id, type_ids);
                }
            }
            NamespaceChild::Rpc(rpc) => handle_rpc(&namespace_id, rpc, type_ids),
            NamespaceChild::Enum(_) => { /* none */ }
            NamespaceChild::TypeAlias(alias) => {
                let alias_id = namespace_id
                    .child(EntityType::TypeAlias, alias.name)
                    .unwrap();
                type_ids.push(
                    alias_id
                        .child(EntityType::Type, entity::subtype::TY_ALIAS_TARGET)
                        .unwrap(),
                )
            }
            NamespaceChild::Namespace(namespace) => {
                let namespace_id = namespace_id
                    .child(EntityType::Namespace, &namespace.name)
                    .unwrap();
                collect_type_ids(namespace, namespace_id, type_ids)
            }
        }
    }
}

fn assembly_definitions<'a>() -> impl Parser<'a, &'a str, (), Error<'a>> {
    let asmdef = util::keyword_ex("assembly")
        .then(just(":").padded())
        .then(any().and_is(just("]").not()).repeated().slice())
        .delimited_by(just("[").padded(), just("]").padded());
    comment::multi().then(asmdef).repeated().ignored()
}

#[cfg(test)]
mod tests {
    use crate::parser::{assembly_definitions, CSharpParser};
    use anyhow::Result;
    use apyxl::model::{Builder, UNDEFINED_NAMESPACE};
    use apyxl::parser::Config;
    use apyxl::test_util::executor::TEST_CONFIG;
    use apyxl::{input, Parser};
    use chumsky::Parser as ChumskyParser;

    #[test]
    fn root_namespace() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        // comment
        using asdf;
        // comment
        // comment
        using asdf.x.y.z;
        // alias comment
        using alias = uint;
        public class dto {
            public static void method() {}
            alias AliasedField = 5;
        }
        private struct private_dto {}
        namespace SomeNamespace {}
        namespace Some.Other.Namespace {}
        enum private_en {}
        public enum en {}
        // end comment ignored
        "#,
        );
        let mut builder = Builder::default();
        CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some(), "dto");
        assert!(model.api().en("en").is_some(), "en");
        assert!(
            model.api().namespace("SomeNamespace").is_some(),
            "SomeNamespace"
        );
        assert!(model.api().dto("private_dto").is_some(), "private_dto");
        assert!(model.api().en("private_en").is_some(), "private_en");
        assert!(
            model.api().dto("dto").unwrap().namespace.is_some(),
            "dto scope namespace"
        );
        assert!(
            model
                .api()
                .dto("dto")
                .unwrap()
                .namespace
                .as_ref()
                .unwrap()
                .rpc("method")
                .is_some(),
            "dto scope rpc"
        );
        assert!(model.api().ty_alias("alias").is_none(), "alias not in api");
        Ok(())
    }

    #[test]
    fn disabled_parse_private() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        public enum en {}
        enum ignored_en {}
        public struct dto {
            // rpc comment
            public void rpc() {}
            private void ignored_rpc() {}
        }
        struct ignored_dto {
            public void ignored_rpc() {}
        }
        "#,
        );
        let mut builder = Builder::default();
        let config = Config {
            enable_parse_private: false,
            ..Default::default()
        };
        CSharpParser::default().parse(&config, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        let dto = model.api().dto("dto");
        assert!(dto.is_some());
        let dto = dto.unwrap();
        assert!(dto.rpc("rpc").is_some());
        assert!(dto.rpc("ignored_rpc").is_none());
        assert!(model.api().en("en").is_some());
        assert!(model.api().dto("ignored_dto").is_none());
        assert!(model.api().en("ignored_en").is_none());
        Ok(())
    }

    #[test]
    fn assembly_definition_parser() {
        let result = assembly_definitions()
            .parse(
                r#"
            // comment
            [assembly: AssemblyVersion(123.123.123.123)]
            "#,
            )
            .into_result();
        if result.is_err() {
            // println!("{}", result.unwrap_err().iter().join(","));
            panic!("error parsing");
        }
    }

    #[test]
    fn ignore_assembly_definitions() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        [assembly: blah blahsd () <>.sd.as=++]
        // comment
        [assembly: AssemblyVersion(123.123.123.123)]
        // comment
        // comment
        [assembly: "zzz"]
        "#,
        );
        let mut builder = Builder::default();
        CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)
    }

    mod local_ty_alias {
        use crate::parser::CSharpParser;
        use anyhow::Result;
        use apyxl::model::{Api, Builder, EntityId, Type};
        use apyxl::test_util::executor::TEST_CONFIG;
        use apyxl::{input, Parser};

        #[test]
        fn basic() -> Result<()> {
            let input = r#"
            using Alias = string;
            struct Dto {
                Alias field;
            }
            "#;
            run_test(input, |api| {
                let entity_id = EntityId::try_from("d:Dto.f:field").unwrap();
                let field = api.find_field(&entity_id).unwrap();
                assert_eq!(field.ty.value, Type::String);
            })
        }

        #[test]
        fn nested_inside() -> Result<()> {
            let input = r#"
            namespace a {
                using Alias = string;
                struct Dto {
                    Alias field;
                }
            }
            "#;
            run_alias_test(input)
        }

        #[test]
        fn nested_outside() -> Result<()> {
            let input = r#"
            using Alias = string;
            namespace a {
                struct Dto {
                    Alias field;
                }
            }
            "#;
            run_alias_test(input)
        }

        #[test]
        fn nested_sibling() -> Result<()> {
            let input = r#"
            namespace a {
                struct Dto {
                    b.Alias field;
                }
            }
            namespace b {
                using Alias = string;
            }
            "#;
            run_alias_test(input)
        }

        #[test]
        fn nested_child() -> Result<()> {
            let input = r#"
            namespace a {
                struct Dto {
                    b.Alias field;
                }
                namespace b {
                    using Alias = string;
                }
            }
            "#;
            run_alias_test(input)
        }

        #[test]
        fn multi_redirect_aliases() -> Result<()> {
            let input = r#"
            using Alias4 = a.x.Alias5;
            namespace a {
                struct Dto {
                    b.Alias1 field;
                }
                namespace x {
                    using Alias5 = string;
                }
                namespace b {
                    using Alias1 = c.Alias2;
                }
                namespace c {
                    using Alias2 = d.Alias3;
                }
            }
            namespace d {
                using Alias3 = Alias4;
            }
            "#;
            run_alias_test(input)
        }

        #[test]
        fn removes_type_aliases() -> Result<()> {
            let input = r#"
            using Alias1 = string;
            namespace a {
                using Alias2 = string;
            }
            "#;
            run_test(input, |api| {
                assert_eq!(api.ty_aliases().count(), 0);
                assert_eq!(api.namespace("a").unwrap().ty_aliases().count(), 0);
            })
        }

        fn run_alias_test(input: &'static str) -> Result<()> {
            run_test(input, |api| {
                let entity_id = EntityId::try_from("a.d:Dto.f:field").unwrap();
                let field = api.find_field(&entity_id).unwrap();
                assert_eq!(field.ty.value, Type::String);
            })
        }

        fn run_test(input: &'static str, assertions: impl FnOnce(&Api)) -> Result<()> {
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();
            assertions(model.api());
            Ok(())
        }
    }
}
