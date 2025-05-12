use anyhow::{anyhow, Result};
use chumsky::container::Container;
use chumsky::prelude::*;
use log::debug;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::model::{Api, EntityId, Namespace, NamespaceChild, Type, TypeRef, UNDEFINED_NAMESPACE};
use crate::parser::rust::import::Import;
use crate::parser::{error, Config};
use crate::{model, rust_util, Input, Parser as ApyxlParser};

mod attributes;
mod comment;
mod dto;
mod en;
mod expr_block;
mod import;
mod namespace;
mod rpc;
mod ty;
mod ty_alias;
mod visibility;

#[derive(Default)]
pub struct Rust {}

impl ApyxlParser for Rust {
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

            let imports = comment::multi()
                .ignore_then(import::parser().padded())
                .repeated()
                .collect::<Vec<_>>();

            let (imports, children) = imports
                .then(
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

            let api = Api {
                name: Cow::Borrowed(UNDEFINED_NAMESPACE),
                children,
                attributes: Default::default(),
                is_virtual: false,
            };

            // Keep track of all EntityIds in this chunk for use in blanket imports.
            let chunk_entity_id = chunk
                .relative_file_path
                .as_ref()
                .map(|file_path| rust_util::path_to_entity_id(file_path))
                .unwrap_or(EntityId::default());
            collect_entity_ids(&api, chunk_entity_id, &mut all_entity_ids);

            chunked_apis.push((chunk, api, imports));
        }

        // Necessary to separate API parsing from merging to builder so that we have the complete
        // API available to qualify imported types. Otherwise, we wouldn't be able to resolve
        // blanket imports like `use a::b::*`.

        for (chunk, mut api, mut imports) in chunked_apis {
            if let Some(file_path) = &chunk.relative_file_path {
                for component in rust_util::path_to_entity_id(file_path).component_names() {
                    builder.enter_namespace(component)
                }
            }

            debug!(
                "applying imports to chunk {:?}...",
                chunk.relative_file_path
            );

            let mut local_entity_ids = HashSet::new();
            collect_entity_ids(&api, EntityId::default(), &mut local_entity_ids);

            sort_imports(&mut imports);
            apply_imports(
                &all_entity_ids,
                &local_entity_ids,
                EntityId::default(),
                &mut api,
                &imports,
            )?;

            debug!("merging chunk {:?}...", chunk.relative_file_path);
            builder.merge_from_chunk(api, chunk);
            builder.clear_namespace();
        }

        Ok(())
    }
}

fn sort_imports(imports: &mut [Import]) {
    // Sort single/multi before blanket as they take priority.
    imports.sort_by(|a, b| match (a, b) {
        (Import::Single(_), Import::Blanket(_)) | (Import::Multi(_), Import::Blanket(_)) => {
            Ordering::Less
        }
        (Import::Blanket(_), Import::Single(_)) | (Import::Blanket(_), Import::Multi(_)) => {
            Ordering::Greater
        }
        _ => Ordering::Equal,
    });
}

fn collect_entity_ids(ns: &Namespace, id: EntityId, set: &mut HashSet<EntityId>) {
    for child in &ns.children {
        let id = id.child_unqualified(child.name());
        set.push(id.clone());
        if let NamespaceChild::Namespace(ns) = child {
            collect_entity_ids(ns, id, set);
        }
    }
}

fn apply_imports(
    all_entity_ids: &HashSet<EntityId>,
    local_entity_ids: &HashSet<EntityId>,
    namespace_id: EntityId,
    namespace: &mut Namespace,
    imports: &[Import],
) -> Result<()> {
    // Also add ids from this portion of the hierarchy tree in order to catch multi nested ids
    // referencing less nested types, for example:
    // mod a {
    //   type Id = u32;
    //   mod b {
    //     struct Entity {
    //       id: Id; // this references the Id inside 'a' even though it's not qualified.
    //     }
    //   }
    // }
    let mut local_entity_ids = local_entity_ids.clone();
    collect_entity_ids(namespace, EntityId::default(), &mut local_entity_ids);

    for dto in namespace.dtos_mut() {
        for field in &mut dto.fields {
            apply_imports_to_type(
                all_entity_ids,
                &local_entity_ids,
                &namespace_id,
                &mut field.ty,
                imports,
            )?;
        }
    }

    for rpc in namespace.rpcs_mut() {
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
    }

    for alias in namespace.ty_aliases_mut() {
        apply_imports_to_type(
            all_entity_ids,
            &local_entity_ids,
            &namespace_id,
            &mut alias.target_ty,
            imports,
        )?;
    }

    // note: enums have no type refs.

    for ns in namespace.namespaces_mut() {
        apply_imports(
            all_entity_ids,
            &local_entity_ids,
            namespace_id.child_unqualified(&ns.name),
            ns,
            imports,
        )?;
    }

    Ok(())
}

fn apply_imports_to_type(
    all_entity_ids: &HashSet<EntityId>,
    local_entity_ids: &HashSet<EntityId>,
    namespace_id: &EntityId,
    ty: &mut TypeRef,
    imports: &[Import],
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
            if !local_entity_ids.contains(id) {
                for import in imports {
                    if let Some(qualified) = qualify_by_import(all_entity_ids, import, id)? {
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
    import: &Import,
    id: &mut EntityId,
) -> Result<Option<EntityId>> {
    match import {
        Import::Single(import) => {
            if let Some(qualified) = qualify_by_import_id(import, id)? {
                return Ok(Some(qualified));
            }
        }
        Import::Multi(imports) => {
            for import in imports {
                if let Some(qualified) = qualify_by_import_id(import, id)? {
                    return Ok(Some(qualified));
                }
            }
        }
        Import::Blanket(import) => {
            let qualified = import.concat(id)?;
            if all_entity_ids.contains(&qualified) {
                return Ok(Some(qualified));
            }
        }
    }
    Ok(None)
}

fn qualify_by_import_id(import: &EntityId, id: &EntityId) -> Result<Option<EntityId>> {
    match (import.component_names().last(), id.component_names().next()) {
        (Some(import_end), Some(id_start)) if import_end == id_start => {
            let qualified = import.parent().unwrap().concat(id)?;
            return Ok(Some(qualified));
        }
        _ => {}
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::model::{Builder, Comment, UNDEFINED_NAMESPACE};
    use crate::parser::Config;
    use crate::test_util::executor::TEST_CONFIG;
    use crate::{input, parser, Parser as ApyxlParser};

    #[test]
    fn root_namespace() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        // comment
        use asdf;
        // comment
        // comment
        pub use asdf;
        // rpc comment
        pub fn rpc() {}
        // zzz
        pub const field: &str = "blah";
        // alias comment
        type private_alias = u32;
        // alias comment
        pub type alias = u32;
        fn private_rpc() {}
        const private_field: u32 = 5;
        pub enum en {}
        enum private_en {}
        pub struct dto {}
        impl dto {
            fn method() {}
        }
        struct private_dto {}
        pub mod namespace {}
        mod private_namespace {}
        // end comment ignored
        "#,
        );
        let mut builder = Builder::default();
        parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some(), "dto");
        assert!(model.api().rpc("rpc").is_some(), "rpc");
        assert!(model.api().en("en").is_some(), "en");
        assert!(model.api().field("field").is_some(), "field");
        assert!(model.api().ty_alias("alias").is_some(), "alias");
        assert!(model.api().namespace("namespace").is_some(), "namespace");
        assert!(model.api().dto("private_dto").is_some(), "private_dto");
        assert!(model.api().rpc("private_rpc").is_some(), "private_rpc");
        assert!(model.api().en("private_en").is_some(), "private_en");
        assert!(
            model.api().field("private_field").is_some(),
            "private_field"
        );
        assert!(
            model.api().ty_alias("private_alias").is_some(),
            "private_alias"
        );
        assert!(
            model.api().namespace("private_namespace").is_some(),
            "private_namespace"
        );
        assert_eq!(
            model.api().rpc("rpc").unwrap().attributes.comments,
            vec![Comment::unowned(&["rpc comment"])],
            "comment after 'use' attributed to rpc"
        );
        assert!(
            model.api().dto("dto").unwrap().namespace.is_some(),
            "dto impl block ns"
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
            "impl block rpc"
        );
        Ok(())
    }

    #[test]
    fn disabled_parse_private() -> Result<()> {
        let mut input = input::Buffer::new(
            r#"
        // comment
        use asdf;
        // comment
        // comment
        pub use asdf;
        // rpc comment
        pub fn rpc() {}
        fn ignored_rpc() {}
        pub enum en {}
        enum ignored_en {}
        pub struct dto {}
        struct ignored_dto {}
        type ignored_alias = u32;
        pub type alias = u32;
        pub mod namespace {}
        mod ignored_namespace {}
        "#,
        );
        let mut builder = Builder::default();
        let config = Config {
            enable_parse_private: false,
            ..Default::default()
        };
        parser::Rust::default().parse(&config, &mut input, &mut builder)?;
        let model = builder.build().unwrap();
        assert_eq!(model.api().name, UNDEFINED_NAMESPACE);
        assert!(model.api().dto("dto").is_some());
        assert!(model.api().rpc("rpc").is_some());
        assert!(model.api().en("en").is_some());
        assert!(model.api().ty_alias("alias").is_some());
        assert!(model.api().namespace("namespace").is_some());
        assert!(model.api().dto("ignored_dto").is_none());
        assert!(model.api().rpc("ignored_rpc").is_none());
        assert!(model.api().en("ignored_en").is_none());
        assert!(model.api().ty_alias("ignored_alias").is_none());
        assert!(model.api().namespace("ignored_namespace").is_none());
        Ok(())
    }

    mod file_path_to_mod {
        use anyhow::Result;

        use crate::model::{Builder, Chunk, EntityId};
        use crate::test_util::executor::TEST_CONFIG;
        use crate::{input, parser, Parser};

        #[test]
        fn file_path_including_name_without_ext() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/c.rs"),
                "pub struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model
                .api()
                .find_namespace(&EntityId::new_unqualified("a.b.c"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }

        #[test]
        fn ignore_mod_rs() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/mod.rs"),
                "pub struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model
                .api()
                .find_namespace(&EntityId::new_unqualified("a.b"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }

        #[test]
        fn ignore_lib_rs() -> Result<()> {
            let mut input = input::ChunkBuffer::new();
            input.add_chunk(
                Chunk::with_relative_file_path("a/b/lib.rs"),
                "pub struct dto {}",
            );
            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let namespace = model
                .api()
                .find_namespace(&EntityId::new_unqualified("a.b"));
            assert!(namespace.is_some());
            assert!(namespace.unwrap().dto("dto").is_some());
            Ok(())
        }
    }

    mod imports {
        use crate::model::{Builder, Chunk, EntityId, Model};
        use crate::test_util::executor::TEST_CONFIG;
        use crate::{input, parser, Parser};
        use anyhow::{anyhow, Result};

        #[test]
        fn dto_field() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::Id;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "test.Entity", "ns:a.a:Id")
        }

        #[test]
        fn rpc_param() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::Id;
            fn rpc(id: Id) {}
            "#;

            run_chunked_test(&[("a", a), ("test", test)], |model| {
                let actual = model
                    .api()
                    .find_rpc(&EntityId::new_unqualified("test.rpc"))
                    .unwrap()
                    .params[0]
                    .ty
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from("ns:a.a:Id").unwrap();
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
            let a = "type Id = u32;";
            let test = r#"
            use a::Id;
            fn rpc() -> Id {}
            "#;

            run_chunked_test(&[("a", a), ("test", test)], |model| {
                let actual = model
                    .api()
                    .find_rpc(&EntityId::new_unqualified("test.rpc"))
                    .unwrap()
                    .return_type
                    .as_ref()
                    .ok_or(anyhow!("no return type"))?
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from("ns:a.a:Id").unwrap();
                assert_eq!(
                    expected, *actual,
                    "expected: {}, actual: {}",
                    expected, actual
                );
                Ok(())
            })
        }

        #[test]
        fn ty_alias() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::Id;
            type MyId = Id;
            "#;

            run_chunked_test(&[("a", a), ("test", test)], |model| {
                let actual = model
                    .api()
                    .find_ty_alias(&EntityId::new_unqualified("test.MyId"))
                    .unwrap()
                    .target_ty
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from("ns:a.a:Id").unwrap();
                assert_eq!(
                    expected, *actual,
                    "expected: {}, actual: {}",
                    expected, actual
                );
                Ok(())
            })
        }

        #[test]
        fn private() -> Result<()> {
            let root = "type Id = u32;";
            let test = r#"
            use crate::Id;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(&[("mod.rs", root), ("test", test)], "test.Entity", "a:Id")
        }

        #[test]
        fn public() -> Result<()> {
            let root = "pub type Id = u32;";
            let test = r#"
            pub use crate::Id;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(&[("mod.rs", root), ("test", test)], "test.Entity", "a:Id")
        }

        #[test]
        fn namespaced_full() -> Result<()> {
            let a = "mod b { mod c { type Id = u32; } }";
            let test = r#"
            use crate::a::b::c::Id;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("test", test)],
                "test.Entity",
                "ns:a.ns:b.ns:c.a:Id",
            )
        }

        #[test]
        fn namespace() -> Result<()> {
            let a = "mod b { mod c { type Id = u32; } }";
            let test = r#"
            use crate::a::b::c;
            struct Entity {
                id: c::Id
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("test", test)],
                "test.Entity",
                "ns:a.ns:b.ns:c.a:Id",
            )
        }

        #[test]
        fn multi() -> Result<()> {
            let a = r#"
            mod b {
                struct Xyz {}
                mod c {
                    type Id = u32;
                    struct Abc {}
                }
            }
            "#;
            let test = r#"
            use a::b::c::{Abc, Id};
            use a::b::{Xyz};
            struct Entity {
                abc: Abc,
                id: Id,
                xyz: Xyz
            }
            "#;

            let mut input = input::ChunkBuffer::new();
            input.add_chunk(Chunk::with_relative_file_path("a"), a);
            input.add_chunk(Chunk::with_relative_file_path("test"), test);

            let mut builder = Builder::default();
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
            let model = builder.build().unwrap();

            let assert = |i: usize, expected: &str| {
                let actual = model
                    .api()
                    .find_dto(&EntityId::new_unqualified("test.Entity"))
                    .unwrap()
                    .fields[i]
                    .ty
                    .value
                    .api()
                    .unwrap();

                let expected = EntityId::try_from(expected).unwrap();
                assert_eq!(
                    expected, *actual,
                    "expected: {}, actual {}",
                    expected, actual
                );
            };

            assert(0, "ns:a.ns:b.ns:c.d:Abc");
            assert(1, "ns:a.ns:b.ns:c.a:Id");
            assert(2, "ns:a.ns:b.d:Xyz");
            Ok(())
        }

        #[test]
        fn blanket() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::*;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "test.Entity", "ns:a.a:Id")
        }

        #[test]
        fn blanket_with_namespace() -> Result<()> {
            let a = "";
            let b = "mod c { mod d { type Id = u32; } }";
            let test = r#"
            use a::*;
            use b::c::*;
            struct Entity {
                id: d::Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("b", b), ("test", test)],
                "test.Entity",
                "ns:b.ns:c.ns:d.a:Id",
            )
        }

        #[test]
        fn explicit_override_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let b = "type Id = u32;";
            let test = r#"
            use a::*;
            use b;
            struct Entity {
                id: b::Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("b", b), ("test", test)],
                "test.Entity",
                "ns:b.a:Id",
            )
        }

        #[test]
        fn local_override_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::*;
            type Id = u32;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "test.Entity", "ns:test.a:Id")
        }

        #[test]
        fn nested_local_override_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::*;
            type Id = u32;
            mod b {
                struct Entity {
                    id: Id,
                }
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "test.b.Entity", "ns:test.a:Id")
        }

        #[test]
        fn extra_nested_local_override_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::*;
            mod b {
                type Id = u32;
                mod c {
                    struct Entity {
                        id: Id,
                    }
                }
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("test", test)],
                "test.b.c.Entity",
                "ns:test.ns:b.a:Id",
            )
        }

        #[test]
        fn single_always_before_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let b = "type Id = u32;";

            // Try with both orders. If one fails we're not handling single before blanket.

            let test = r#"
            use b::Id;
            use a::*;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("b", b), ("test", test)],
                "test.Entity",
                "ns:b.a:Id",
            )?;

            let test = r#"
            use a::*;
            use b::Id;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("b", b), ("test", test)],
                "test.Entity",
                "ns:b.a:Id",
            )
        }

        #[test]
        fn multi_always_before_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let b = "type Id = u32;";

            // Try with both orders. If one fails we're not handling multi before blanket.

            let test = r#"
            use b::{Id};
            use a::*;
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("b", b), ("test", test)],
                "test.Entity",
                "ns:b.a:Id",
            )?;

            let test = r#"
            use a::*;
            use b::{Id};
            struct Entity {
                id: Id,
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("b", b), ("test", test)],
                "test.Entity",
                "ns:b.a:Id",
            )
        }

        #[test]
        fn local_sibling_nested_no_override_blanket() -> Result<()> {
            let a = "type Id = u32;";
            let test = r#"
            use a::*;
            mod b {
                type Id = u32;
            }
            mod c {
                struct Entity {
                    id: Id,
                }
            }
            "#;
            run_dto_chunked_test(&[("a", a), ("test", test)], "test.c.Entity", "ns:a.a:Id")
        }

        #[test]
        fn explicit_sibling_nested_override_blanket() -> Result<()> {
            let a = r#"
            mod b {
                type Id = u32;
            }
            "#;
            let test = r#"
            use a::*;
            mod b {
                type Id = u32;
            }
            mod c {
                struct Entity {
                    id: b::Id,
                }
            }
            "#;
            run_dto_chunked_test(
                &[("a", a), ("test", test)],
                "test.c.Entity",
                "ns:test.ns:b.a:Id",
            )
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
            parser::Rust::default().parse(&TEST_CONFIG, &mut input, &mut builder)?;
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
