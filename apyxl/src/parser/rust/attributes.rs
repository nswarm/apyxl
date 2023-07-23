use crate::model::attribute;
use crate::parser::rust::Error;
use chumsky::prelude::{just, none_of, skip_then_retry_until};
use chumsky::{text, IterParser, Parser};

pub fn attributes<'a>() -> impl Parser<'a, &'a str, Vec<attribute::User<'a>>, Error<'a>> {
    let name = text::ident();
    let data = text::ident()
        .then(just('=').padded().ignore_then(text::ident()).or_not())
        .map(|(lhs, rhs)| match rhs {
            None => attribute::UserData::new(None, lhs),
            Some(rhs) => attribute::UserData::new(Some(lhs), rhs),
        });
    let data_list = data
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded())
        .or_not();
    name.then(data_list)
        .map(|(name, data)| attribute::User {
            name,
            data: data.unwrap_or(vec![]),
        })
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just("#[").padded(), just(']').padded())
        .recover_with(skip_then_retry_until(
            none_of(",]").ignored(),
            just(']').ignored(),
        ))
        .or_not()
        .map(|opt| opt.unwrap_or(vec![]))
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::model::attribute;
    use crate::model::attribute::UserData;
    use crate::parser::rust::dto;
    use crate::test_util::executor::TEST_CONFIG;

    #[test]
    fn flags() {
        run_test(
            r#"
                    #[flag1, flag2, flag3]
                    struct dto {}
                    "#,
            vec![
                attribute::User::new_flag("flag1"),
                attribute::User::new_flag("flag2"),
                attribute::User::new_flag("flag3"),
            ],
        )
    }

    #[test]
    fn lists() {
        run_test(
            r#"
                    #[attr0(a_one), attr1(a_two, b_two, c_two)]
                    struct dto {}
                    "#,
            vec![
                attribute::User::new("attr0", vec![UserData::new(None, "a_one")]),
                attribute::User::new(
                    "attr1",
                    vec![
                        UserData::new(None, "a_two"),
                        UserData::new(None, "b_two"),
                        UserData::new(None, "c_two"),
                    ],
                ),
            ],
        )
    }

    #[test]
    fn maps() {
        run_test(
            r#"
                    #[attr0(k0 = v0, k1 = v1), attr1(k00 = v00)]
                    struct dto {}
                    "#,
            vec![
                attribute::User::new(
                    "attr0",
                    vec![
                        UserData::new(Some("k0"), "v0"),
                        UserData::new(Some("k1"), "v1"),
                    ],
                ),
                attribute::User::new("attr1", vec![UserData::new(Some("k00"), "v00")]),
            ],
        )
    }

    #[test]
    fn mixed() {
        run_test(
            r#"
                    #[attr0(k0 = v0, k1 = v1), attr1, attr2(one, two, three)]
                    struct dto {}
                    "#,
            vec![
                attribute::User::new(
                    "attr0",
                    vec![
                        UserData::new(Some("k0"), "v0"),
                        UserData::new(Some("k1"), "v1"),
                    ],
                ),
                attribute::User::new_flag("attr1"),
                attribute::User::new(
                    "attr2",
                    vec![
                        UserData::new(None, "one"),
                        UserData::new(None, "two"),
                        UserData::new(None, "three"),
                    ],
                ),
            ],
        )
    }

    fn run_test(content: &str, expected: Vec<attribute::User>) {
        let (dto, _) = dto::parser(&TEST_CONFIG).parse(content).into_result().unwrap();
        assert_eq!(dto.attributes.user, expected);
    }
}
