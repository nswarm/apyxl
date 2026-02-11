// fn validate(parser: impl Parser, feature: Feature, input: &str) -> Result<()> {
//     // let model = builder.build().map_err(|errs| {
//     //     let mut message = String::from("Validation errors:\n\n");
//     //     for err in &errs {
//     //         write!(&mut message, "{}\n\n", err).unwrap();
//     //     }
//     //     anyhow!(message)
//     // })?;
//
//     let config = Config::default();
//     let pyx_builder = builder(Pyx::default(), &config, feature.to_pyx_str());
//     let target_builder = builder(parser, &config, input);
//
//     insta::assert_yaml_snapshot!();
//
//     Ok(())
// }

mod tests {
    use crate::feature::Feature;
    use crate::parser::pyx::Pyx;
    use crate::parser::Config;
    use crate::{input, model, Parser};
    use anyhow::Result;
    // macro_rules! validate {
    //     ($name:ident, $test_content:literal) => {
    //         #[test]
    //         fn $name() -> Result<()> {
    //             insta::assert_yaml_snapshot();
    //         }
    //     };
    // }

    #[test]
    fn namespace() -> Result<()> {
        let feature = Feature::Namespace;
        let input = feature.to_pyx_str();
        let parser = Pyx::default();
        let config = Config::default();
        let mut input = input::Buffer::new(input);
        let mut builder = model::Builder::default();
        parser.parse(&config, &mut input, &mut builder)?;

        insta::assert_yaml_snapshot!(builder.into_api(), @"
        name: _
        children:
          - Namespace:
              name: ns
              children: []
              attributes:
                chunk: ~
                entity_id:
                  components: []
                comments: []
                user: []
              is_virtual: false
        attributes:
          chunk: ~
          entity_id:
            components: []
          comments: []
          user: []
        is_virtual: false
        ");

        Ok(())
    }
}
