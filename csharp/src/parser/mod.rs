use std::borrow::Cow;

use anyhow::{anyhow, Result};
use chumsky::prelude::*;
use itertools::Itertools;
use log::debug;

use apyxl::model::entity::{EntityMut, FindEntity};
use apyxl::model::{
    entity, Api, Chunk, EntityId, EntityType, Field, Namespace, NamespaceChild, Rpc, Semantics,
    Type, TypeRef, UNDEFINED_NAMESPACE,
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

            let imports = using(config)
                .padded()
                .repeated()
                .collect::<Vec<_>>()
                .ignored();

            let children = imports // Ignore local aliases as part of the api.
                .ignore_then(assembly_definitions())
                .ignore_then(
                    namespace::children(config, namespace::parser(config), end().ignored())
                        .padded(),
                )
                .then_ignore(end())
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

            remove_chunk_local_aliases(config, chunk, data, &mut api)?;

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
fn remove_chunk_local_aliases(
    config: &Config,
    chunk: &Chunk,
    raw_data: &str,
    api: &mut Api,
) -> Result<()> {
    // Using statements must be at the beginning of the file. So just reparse to use here.
    let aliases = comment::multi()
        .ignore_then(using(config))
        .padded()
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(any().repeated())
        .parse(raw_data)
        .into_result()
        .map_err(|errs| {
            let return_err = anyhow!(
                "errors encountered while parsing local aliases: {:?}",
                &errs
            );
            error::report_errors(chunk, raw_data, errs.clone());
            return_err
        })?
        .into_iter()
        .flatten()
        .collect_vec();

    let mut type_ids = Vec::new();
    collect_type_ids(api, EntityId::default(), &mut type_ids);

    for type_id in type_ids {
        let actual_type_ref = api
            .find_entity_mut(type_id)
            .expect("entity id should exist since we just collected it");
        let actual_type_ref = if let EntityMut::Type(type_ref) = actual_type_ref {
            type_ref
        } else {
            unreachable!("we only collect TypeRef entity ids, so this should really be one");
        };
        if let Some(alias) = aliases.iter().find(|x| x.find == *actual_type_ref) {
            actual_type_ref.value = alias.replace.value.clone();
            actual_type_ref.semantics = alias.replace.semantics;
        }
    }

    Ok(())
}

// I don't love this, but I have a plan for a much improved hierarchy iteration refactor in the future.
pub fn collect_type_ids(
    namespace: &Namespace,
    namespace_id: EntityId,
    type_ids: &mut Vec<EntityId>,
) {
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

#[derive(Debug, Clone, Eq, PartialEq)]
struct LocalAlias {
    find: TypeRef,
    replace: TypeRef,
}

// Works for all types of `using`, but returns None for non-LocalAlias.
fn using(config: &Config) -> impl Parser<&str, Option<LocalAlias>, Error> {
    let find = text::ident()
        .map(|s: &str| TypeRef {
            value: Type::Api(EntityId::new_unqualified(s.trim())),
            semantics: Semantics::Value,
        })
        .then_ignore(just("=").padded());
    let replace = ty::parser(config);
    comment::multi()
        .ignore_then(util::keyword_ex("using"))
        .ignore_then(text::whitespace().at_least(1))
        .ignore_then(find.or_not())
        .then(replace)
        .then_ignore(just(';').padded())
        .map(|(find, replace)| find.map(|find| LocalAlias { find, replace }))
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
    use apyxl::model::{Builder, Type, UNDEFINED_NAMESPACE};
    use apyxl::parser::Config;
    use apyxl::test_util::executor::TEST_CONFIG;
    use apyxl::{input, Parser};
    use chumsky::Parser as ChumskyParser;
    use itertools::Itertools;

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
        }
        private struct private_dto {}
        namespace SomeNamespace {}
        namespace Some.Other.Namespace {}
        enum private_en {}
        public enum en {}
        /// rpc comment
        public void rpc() {}
        private void private_rpc() {}
        alias rpc_with_alias_return() {}
        // end comment ignored
        "#,
        );
        let mut builder = Builder::default();
        CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some(), "dto");
        assert!(model.api().rpc("rpc").is_some(), "rpc");
        assert!(model.api().en("en").is_some(), "en");
        assert!(
            model.api().namespace("SomeNamespace").is_some(),
            "SomeNamespace"
        );
        assert!(model.api().dto("private_dto").is_some(), "private_dto");
        assert!(model.api().rpc("private_rpc").is_some(), "private_rpc");
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
        let alias_rpc = model.api().rpc("rpc_with_alias_return");
        assert!(alias_rpc.is_some(), "rpc_with_alias_return");
        assert_eq!(
            alias_rpc.unwrap().return_type.as_ref().unwrap().value,
            Type::U32,
            "alias applied"
        );
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
            println!("{}", result.unwrap_err().iter().join(","));
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

    mod using {
        use crate::parser::{using, CSharpParser, LocalAlias};
        use anyhow::Result;
        use apyxl::model::{Builder, EntityId, Semantics, Type, TypeRef};
        use apyxl::parser::test_util::wrap_test_err;
        use apyxl::test_util::executor::TEST_CONFIG;
        use apyxl::{input, Parser};
        use chumsky::Parser as ChumskyParser;

        #[test]
        fn direct_parse() {
            let input = "using asd;";
            let result = using(&TEST_CONFIG).parse(input).into_result();
            assert!(result.is_ok());
        }

        #[test]
        fn import() -> Result<()> {
            assert_no_parse_errors("using asd;")
        }

        #[test]
        fn namespaced() -> Result<()> {
            assert_no_parse_errors("using a.b.c.d;")
        }

        #[test]
        fn import_with_comment() -> Result<()> {
            assert_no_parse_errors(
                r#"
            // comment
            using a.b.c.d;
            "#,
            )
        }

        #[test]
        fn alias() -> Result<()> {
            assert_local_alias(
                r#"using a = b;"#,
                LocalAlias {
                    find: type_ref("a"),
                    replace: type_ref("b"),
                },
            )
        }

        #[test]
        fn trims_alias() -> Result<()> {
            assert_local_alias(
                r#"using   a    =   b      ;"#,
                LocalAlias {
                    find: type_ref("a"),
                    replace: type_ref("b"),
                },
            )
        }

        #[test]
        fn complex_alias() -> Result<()> {
            assert_local_alias(
                r#"using asdf_asd = b;"#,
                LocalAlias {
                    find: type_ref("asdf_asd"),
                    replace: type_ref("b"),
                },
            )
        }

        #[test]
        fn alias_replaces_types() -> Result<()> {
            let input = r#"
            using BlahType = string;
            class Dto {
                BlahType bt;
                BlahType func(BlahType bt) {}
            }
            "#;
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();
            let api = model.api();

            let dto = api.dto("Dto").unwrap();
            let field = dto.field("bt").unwrap();
            let rpc = dto.rpc("func").unwrap();
            assert_eq!(field.ty.value, Type::String);
            assert_eq!(rpc.return_type.as_ref().unwrap().value, Type::String);
            Ok(())
        }

        fn type_ref(id: &str) -> TypeRef {
            TypeRef {
                value: Type::Api(EntityId::new_unqualified(id)),
                semantics: Semantics::Value,
            }
        }

        fn assert_local_alias(input: &'static str, expected: LocalAlias) -> Result<()> {
            let actual = using(&TEST_CONFIG)
                .parse(input)
                .into_result()
                .map_err(wrap_test_err)?;
            assert_eq!(actual, Some(expected));
            Ok(())
        }

        fn assert_no_parse_errors(input: &str) -> Result<()> {
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)
        }
    }
}
