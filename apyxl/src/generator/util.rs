use crate::model::{Dependencies, EntityType};
use crate::view::Namespace;
use crate::{model, Output};
use itertools::Itertools;
use std::path::PathBuf;

pub fn write_joined_str(
    components: &[&str],
    separator: &str,
    o: &mut dyn Output,
) -> anyhow::Result<()> {
    write_joined(components, separator, o, |component, o| o.write(component))
}

/// Writes the `components` joined with `separator` without unnecessary allocations.
pub fn write_joined<T, F>(
    components: &[T],
    separator: &str,
    o: &mut dyn Output,
    write_component: F,
) -> anyhow::Result<()>
where
    F: Fn(&T, &mut dyn Output) -> anyhow::Result<()>,
{
    let mut first = true;
    for component in components {
        if !first {
            o.write(separator)?;
        }
        first = false;
        write_component(component, o)?;
    }
    Ok(())
}

/// Collects relative paths for every chunk referenced by any child (recursively) within `dependent_ns`.
pub fn collect_chunk_dependencies<'v, 'a>(
    root: &'v Namespace<'v, 'a>,
    dependent_id: &model::EntityId,
    dependent_ns: Namespace<'v, 'a>,
    dependencies: &'v Dependencies,
) -> Vec<PathBuf> {
    collect_dependencies_recursively(dependent_id, dependent_ns, dependencies)
        .iter()
        .flat_map(|id| match root.find_child(&id) {
            None => vec![],
            Some(child) => match child.attributes().chunk() {
                None => vec![],
                Some(attr) => attr.relative_file_paths.clone(),
            },
        })
        .dedup()
        .collect_vec()
}

/// Collects all [model::EntityId]s that `dependent` [Namespace] depends on by recursing the
/// hierarchy and collecting all dependents of each [NamespaceChild].
fn collect_dependencies_recursively<'a>(
    dependent_id: &model::EntityId,
    dependent_ns: Namespace,
    dependencies: &'a Dependencies,
) -> Vec<&'a model::EntityId> {
    let child_dependencies = dependent_ns
        .children()
        .map(|child| {
            // unwrap ok: we're iterating over known children.
            dependent_id
                .child(child.entity_type(), child.name())
                .unwrap()
        })
        .flat_map(|id| dependencies.get_for(&id));
    dependent_ns
        .namespaces()
        .flat_map(|ns| {
            // unwrap ok: we're iterating over known children.
            collect_dependencies_recursively(
                &dependent_id
                    .child(EntityType::Namespace, ns.name())
                    .unwrap(),
                ns,
                dependencies,
            )
        })
        .chain(child_dependencies)
        .collect_vec()
}

#[cfg(test)]
#[allow(dead_code)]
pub mod tests {
    use crate::test_util::executor::TestExecutor;
    use crate::{output, Generator};

    pub fn assert_output<F: FnOnce(&mut output::Buffer) -> anyhow::Result<()>>(
        write: F,
        expected: &str,
    ) -> anyhow::Result<()> {
        let mut output = output::Buffer::default();
        write(&mut output)?;
        println!("OUTPUT:\n{}", output.data());
        assert_eq!(output.data(), expected);
        Ok(())
    }

    pub fn assert_output_slice<F: FnOnce(&mut output::Buffer) -> anyhow::Result<()>>(
        write: F,
        expected: &[&str],
    ) -> anyhow::Result<()> {
        assert_output(write, &expected.join("\n"))
    }

    pub fn assert_output_contains<F: FnOnce(&mut output::Buffer) -> anyhow::Result<()>>(
        write: F,
        expected: &str,
    ) -> anyhow::Result<()> {
        let mut output = output::Buffer::default();
        write(&mut output)?;
        println!("OUTPUT:\n{}", output.data());
        assert!(
            &output.data().contains(expected),
            "output did not contain expected sequence"
        );
        Ok(())
    }

    pub fn assert_e2e<T: Generator + Default>(data: &str, expected: &str) -> anyhow::Result<()> {
        let mut exe = TestExecutor::new(data);
        let model = exe.build();
        let view = model.view();
        assert_output(move |o| T::default().generate(view, o), expected)
    }

    pub fn assert_e2e_contains<T: Generator + Default>(
        data: &str,
        expected: &str,
    ) -> anyhow::Result<()> {
        let mut exe = TestExecutor::new(data);
        let model = exe.build();
        let view = model.view();
        assert_output_contains(move |o| T::default().generate(view, o), expected)
    }

    pub fn indent(indent: &str, s: &str) -> String {
        [indent, s].join("")
    }
}
