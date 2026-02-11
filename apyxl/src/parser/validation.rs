mod tests {
    use crate::feature::Feature;
    use crate::parser::pyx::Pyx;
    use crate::parser::Config;
    use crate::{input, model, Parser};
    use anyhow::Result;

    macro_rules! validate {
        ($name:ident, $feature:expr) => {
            #[test]
            fn $name() -> Result<()> {
                let input = $feature.to_pyx_str();
                let parser = Pyx::default();
                let config = Config::default();
                let mut input = input::Buffer::new(input);
                let mut builder = model::Builder::default();
                parser.parse(&config, &mut input, &mut builder)?;

                let api = builder.into_api();
                insta::assert_yaml_snapshot!(api);

                Ok(())
            }
        };
    }

    validate!(namespace, Feature::Namespace);
}
