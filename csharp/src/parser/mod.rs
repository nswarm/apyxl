use anyhow::{anyhow, Result};
use chumsky::container::Container;
use chumsky::prelude::*;
use itertools::{Either, Itertools};
use log::debug;
use std::borrow::Cow;
use std::collections::HashSet;

use apyxl::model::entity::{EntityMut, FindEntity};
use apyxl::model::{
    entity, Api, EntityId, EntityType, Field, Namespace, NamespaceChild, Rpc, Type, TypeAlias,
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
mod property;
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
        let mut chunked_apis = Vec::new();
        let mut all_entity_ids = HashSet::<EntityId>::default();
        for (chunk, data) in input.chunks() {
            debug!("parsing chunk {:?}", chunk.relative_file_path);

            // These must come before asmdefs, but still be appended to the top level
            // namespace aliases.
            let imports = choice((
                ty_alias::parser(config).map(Import::Alias),
                import().map(Import::Namespace),
            ))
            .padded()
            .repeated()
            .collect::<Vec<_>>();

            let (children, imports) = imports
                .then_ignore(assembly_definitions())
                .then(
                    namespace::children(config, namespace::parser(config), end().ignored())
                        .padded(),
                )
                .then_ignore(end())
                .map(|(mut imports, mut children)| {
                    let (imports, mut aliases): (Vec<_>, Vec<_>) =
                        imports.into_iter().partition_map(|import| match import {
                            Import::Namespace(id) => Either::Left(id),
                            Import::Alias(alias) => Either::Right(NamespaceChild::TypeAlias(alias)),
                        });
                    children.append(&mut aliases);
                    (children, imports)
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

            // Keep track of all EntityIds in this chunk for use in imports.
            collect_referenceable_entity_ids(&api, EntityId::default(), &mut all_entity_ids);

            chunked_apis.push((chunk, api, imports));
        }

        // Necessary to separate API parsing from merging to builder so that we have the complete
        // API available to qualify imported types. This is because C# only has namespace imports,
        // and we don't know what's in the namespace until we parse it.

        for (chunk, mut api, imports) in chunked_apis {
            debug!(
                "applying imports to chunk {:?}...",
                chunk.relative_file_path
            );

            // Need to know what's in this chunk so it has precedence over those in others.
            let mut local_entity_ids = HashSet::new();
            collect_referenceable_entity_ids(&api, EntityId::default(), &mut local_entity_ids);

            apply_imports(
                &all_entity_ids,
                &local_entity_ids,
                EntityId::default(),
                &mut api,
                &imports,
            )?;

            debug!("merging chunk {:?}...", chunk.relative_file_path);
            builder.merge_from_chunk(api, chunk);
        }

        Ok(())
    }
}

enum Import<'a> {
    Namespace(EntityId),
    Alias(TypeAlias<'a>),
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

fn collect_referenceable_entity_ids(ns: &Namespace, id: EntityId, set: &mut HashSet<EntityId>) {
    for child in &ns.children {
        let id = id.child_unqualified(child.name());
        set.push(id.clone());

        match child {
            NamespaceChild::Dto(dto) => {
                if let Some(ns) = &dto.namespace {
                    collect_referenceable_entity_ids(ns, id, set)
                }
            }
            NamespaceChild::Namespace(ns) => collect_referenceable_entity_ids(ns, id, set),
            _ => {}
        }
    }
}

fn apply_imports(
    all_entity_ids: &HashSet<EntityId>,
    local_entity_ids: &HashSet<EntityId>,
    namespace_id: EntityId,
    namespace: &mut Namespace,
    imports: &[EntityId],
) -> Result<()> {
    // Also add ids from this portion of the hierarchy tree in order to catch multi nested ids
    // referencing less-nested types, for example:
    // namespace a {
    //   using Id = int;
    //   namespace b {
    //     struct Entity {
    //       id: Id; // this references the Id inside 'a' even though it's not qualified.
    //     }
    //   }
    // }
    let mut local_entity_ids = local_entity_ids.clone();
    collect_referenceable_entity_ids(namespace, EntityId::default(), &mut local_entity_ids);

    println!(
        r#"
--- {} ---
all_entity_ids:
  {}
local_entity_ids:
  {}
imports:
  {}
"#,
        namespace_id,
        all_entity_ids.iter().format("\n  "),
        local_entity_ids.iter().format("\n  "),
        imports.iter().format("\n  ")
    );

    // todo this is getting ridiculously convoluted. will the Great Entity Refactor (tm) help?

    let apply_import_to_field = |field: &mut Field,
                                 namespace_id: EntityId,
                                 local_entity_ids: &HashSet<EntityId>|
     -> Result<()> {
        apply_imports_to_type(
            all_entity_ids,
            &local_entity_ids,
            &namespace_id,
            &mut field.ty,
            imports,
        )
    };

    let apply_import_to_rpc = |rpc: &mut Rpc,
                               namespace_id: EntityId,
                               local_entity_ids: &HashSet<EntityId>|
     -> Result<()> {
        for param in &mut rpc.params {
            apply_imports_to_type(
                all_entity_ids,
                &local_entity_ids,
                &namespace_id,
                &mut param.ty,
                imports,
            )?;
        }
        if let Some(return_ty) = &mut rpc.return_type {
            apply_imports_to_type(
                all_entity_ids,
                &local_entity_ids,
                &namespace_id,
                return_ty,
                imports,
            )?;
        }
        Ok(())
    };

    for dto in namespace.dtos_mut() {
        let dto_id = namespace_id.child_unqualified(dto.name);

        let mut dto_entity_ids = local_entity_ids.clone();
        if let Some(dto_ns) = &mut dto.namespace {
            apply_imports(
                all_entity_ids,
                &local_entity_ids,
                dto_id.clone(),
                dto_ns,
                imports,
            )?;

            // Need to do this here so that types inside non-static fields/rpcs can properly
            // reference types nested inside the dto.
            collect_referenceable_entity_ids(dto_ns, EntityId::default(), &mut dto_entity_ids);
        }

        for field in &mut dto.fields {
            apply_import_to_field(field, dto_id.clone(), &dto_entity_ids)?;
        }
        for rpc in &mut dto.rpcs {
            apply_import_to_rpc(rpc, dto_id.clone(), &dto_entity_ids)?;
        }
        // note: enums have no type refs.
    }

    for rpc in namespace.rpcs_mut() {
        apply_import_to_rpc(rpc, namespace_id.clone(), &local_entity_ids)?;
    }

    for field in namespace.fields_mut() {
        apply_import_to_field(field, namespace_id.clone(), &local_entity_ids)?;
    }

    // note: enums have no type refs.

    for ns in namespace.namespaces_mut() {
        let ns_id = namespace_id.child_unqualified(&ns.name);
        apply_imports(all_entity_ids, &local_entity_ids, ns_id, ns, imports)?;
    }

    Ok(())
}

fn apply_imports_to_type(
    all_entity_ids: &HashSet<EntityId>,
    local_entity_ids: &HashSet<EntityId>,
    namespace_id: &EntityId,
    ty: &mut TypeRef,
    imports: &[EntityId],
) -> Result<()> {
    match &mut ty.value {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::USIZE
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::F8
        | Type::F16
        | Type::F32
        | Type::F64
        | Type::F128
        | Type::String
        | Type::StringView
        | Type::Bytes
        | Type::User(_) => {}

        Type::Array(ty) => {
            apply_imports_to_type(all_entity_ids, local_entity_ids, namespace_id, ty, imports)?
        }
        Type::Optional(ty) => {
            apply_imports_to_type(all_entity_ids, local_entity_ids, namespace_id, ty, imports)?
        }
        Type::Map { key, value } => {
            apply_imports_to_type(all_entity_ids, local_entity_ids, namespace_id, key, imports)?;
            apply_imports_to_type(
                all_entity_ids,
                local_entity_ids,
                namespace_id,
                value,
                imports,
            )?;
        }
        Type::Api(id) => {
            println!("testing {}", id);
            if !local_entity_ids.contains(id) {
                println!("no local id for {}", id);
                for import in imports {
                    if let Some(qualified) = qualify_by_import(all_entity_ids, import, id)? {
                        println!("-- {} qualified to {}", id, qualified);
                        *id = qualified;
                        break;
                    }
                }
            }
        }
    };
    Ok(())
}

fn qualify_by_import(
    all_entity_ids: &HashSet<EntityId>,
    import: &EntityId,
    id: &mut EntityId,
) -> Result<Option<EntityId>> {
    let mut import_iter = import.clone();
    loop {
        let qualified = import_iter.concat(id)?;
        if all_entity_ids.contains(&qualified) {
            return Ok(Some(qualified));
        }
        import_iter = match import_iter.parent() {
            None => break,
            Some(parent) => parent,
        }
    }
    Ok(None)
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
        let rpc_id = parent_id.child(EntityType::Rpc, &rpc.name).unwrap();
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

fn import<'a>() -> impl Parser<'a, &'a str, EntityId, Error<'a>> {
    let prefix = util::keyword_ex("using").then(text::whitespace().at_least(1));
    let namespace = text::ident()
        .separated_by(just('.').padded())
        .at_least(1)
        .collect::<Vec<_>>();
    comment::multi()
        .ignore_then(attributes::attributes().padded())
        .ignore_then(prefix)
        .ignore_then(namespace)
        .then_ignore(just(';').padded())
        .map(|namespace| EntityId::new_unqualified_vec(namespace.into_iter()))
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

    mod imports {
        use crate::parser::{import, CSharpParser};
        use anyhow::{anyhow, Result};
        use apyxl::input;
        use apyxl::model::{Builder, Chunk, EntityId, Model};
        use apyxl::parser::test_util::wrap_test_err;
        use apyxl::test_util::executor::TEST_CONFIG;
        use apyxl::Parser as ApyxlParser;
        use chumsky::Parser;

        #[test]
        fn basic_import() -> Result<()> {
            parse_import("using asd;")
        }

        #[test]
        fn namespaced_using() -> Result<()> {
            parse_import("using a.b.c.d;")
        }

        #[test]
        fn using_with_comment() -> Result<()> {
            parse_import(
                r#"
            // comment
            using a.b.c.d;
            "#,
            )
        }

        #[test]
        fn using_with_attrs() -> Result<()> {
            parse_import(
                r#"
            [attr]
            using a.b.c.d;
            "#,
            )
        }

        #[test]
        fn multiple_imports() -> Result<()> {
            let input = r#"
            using a.b;
            using c.d;
            "#;
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            Ok(())
        }

        #[test]
        fn multiple_imports_and_aliases() -> Result<()> {
            let input = r#"
            using a.b;
            using Blah = a.b.xyz;
            using c.d;
            "#;
            let mut input = input::Buffer::new(input);
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            Ok(())
        }

        #[test]
        fn dto_field() -> Result<()> {
            let a = r#"
            namespace a {
                struct Id {}
            }
            "#;
            let test = r#"
            using a;
            struct Entity {
                Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "Entity", "ns:a.d:Id")
        }

        #[test]
        fn rpc_param() -> Result<()> {
            let a = r#"
            namespace a {
                struct Id {}
            }
            "#;
            let test = r#"
            using a;
            struct Entity {
                void rpc(Id id) {}
            }
            "#;

            run_chunked_test(&[("a", a), ("test", test)], |model| {
                let actual = model
                    .api()
                    .find_rpc(&EntityId::new_unqualified("Entity.rpc"))
                    .unwrap()
                    .params[0]
                    .ty
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from("ns:a.d:Id").unwrap();
                assert_eq!(
                    expected, *actual,
                    "expected: {}, actual: {}",
                    expected, actual
                );
                Ok(())
            })
        }

        #[test]
        fn rpc_return_ty() -> Result<()> {
            let a = r#"
            namespace a {
                struct Id {}
            }
            "#;
            let test = r#"
            using a;
            struct Entity {
                Id rpc() {}
            }
            "#;

            run_chunked_test(&[("a", a), ("test", test)], |model| {
                let actual = model
                    .api()
                    .find_rpc(&EntityId::new_unqualified("Entity.rpc"))
                    .unwrap()
                    .return_type
                    .as_ref()
                    .ok_or(anyhow!("no return type"))?
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from("ns:a.d:Id").unwrap();
                assert_eq!(
                    expected, *actual,
                    "expected: {}, actual: {}",
                    expected, actual
                );
                Ok(())
            })
        }

        #[test]
        fn namespace_full() -> Result<()> {
            let a = "namespace a { namespace b { namespace c { struct Id {} } } }";
            let test = r#"
            using a.b;
            struct Entity {
                a.b.c.Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "Entity", "ns:a.ns:b.ns:c.d:Id")
        }

        #[test]
        fn namespace() -> Result<()> {
            let a = "namespace a { namespace b { namespace c { struct Id {} } } }";
            let test = r#"
            using a.b.c;
            struct Entity {
                c.Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "Entity", "ns:a.ns:b.ns:c.d:Id")
        }

        #[test]
        fn explicit_override() -> Result<()> {
            let a = "namespace a { struct Id {} }";
            let b = "namespace b { struct Id {} }";
            let test = r#"
            using a;
            using b;
            struct Entity {
                b.Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("b", b), ("test", test)], "Entity", "ns:b.d:Id")
        }

        #[test]
        fn local_override() -> Result<()> {
            let a = "namespace a { struct Id {} }";
            let test = r#"
            using a;
            struct Id {}
            struct Entity {
                Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "Entity", "d:Id")
        }

        #[test]
        fn nested_dto_override() -> Result<()> {
            let a = "namespace a { struct Id {} }";
            let test = r#"
            using a;
            struct Entity {
                struct Id {}
                Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "Entity", "d:Entity.d:Id")
        }

        #[test]
        fn nested_enum_override() -> Result<()> {
            let a = "namespace a { struct Id {} }";
            let test = r#"
            using a;
            struct Entity {
                enum Id {}
                Id id;
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "Entity", "d:Entity.e:Id")
        }

        #[test]
        fn nested_local_override() -> Result<()> {
            let a = "namespace a {  }";
            let test = r#"
            using a;
            struct Id {}
            namespace b {
                struct Entity {
                    Id id;
                }
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "b.Entity", "d:Id")
        }

        #[test]
        fn extra_nested_local_override() -> Result<()> {
            let a = "namespace a { struct Id {} }";
            let test = r#"
            using a;
            namespace b {
                struct Id {}
                namespace c {
                    struct Entity {
                        Id id;
                    }
                }
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "b.c.Entity", "ns:b.d:Id")
        }

        #[test]
        fn local_sibling_nested_no_override() -> Result<()> {
            let a = "namespace a { struct Id {} }";
            let test = r#"
            using a;
            namespace b {
                struct Id {}
            }
            namespace c {
                struct Entity {
                    Id id;
                }
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "c.Entity", "ns:a.d:Id")
        }

        #[test]
        fn explicit_sibling_nested_override() -> Result<()> {
            let a = r#"
            namespace a {
                namespace b {
                    struct Id {}
                }
            }
            "#;
            let test = r#"
            using a;
            namespace b {
                struct Id {}
            }
            namespace c {
                struct Entity {
                    b.Id id;
                }
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "c.Entity", "ns:b.d:Id")
        }

        fn parse_import(input: &'static str) -> Result<()> {
            let _ = import().parse(input).into_result().map_err(wrap_test_err)?;
            Ok(())
        }

        fn run_chunked_test(
            inputs: &[(&str, &str)],
            assertions: impl FnOnce(&Model) -> Result<()>,
        ) -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            for (path, data) in inputs {
                input.add_chunk(Chunk::with_relative_file_path(path), data);
            }
            let mut builder = Builder::default();
            CSharpParser::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            assertions(&model)?;
            Ok(())
        }

        fn run_dto_chunked_test(
            inputs: &[(&str, &str)],
            dto_id: &str,
            expected_entity_id: &str,
        ) -> Result<()> {
            run_chunked_test(inputs, |model| {
                let actual = model
                    .api()
                    .find_dto(&EntityId::new_unqualified(dto_id))
                    .unwrap()
                    .fields[0]
                    .ty
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from(expected_entity_id)?;
                assert_eq!(
                    expected, *actual,
                    "expected: {}, actual: {}",
                    expected, actual
                );
                Ok(())
            })
        }
    }
}
