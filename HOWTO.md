## Adding a NamespaceChild

List of stuff to consider when adding a new NamespaceChild:
- model type
  - need `name` and `attributes`
  - add to `NamespaceChild` enum
  - add find/get/iterate methods to `Namespace`
  - add to `EntityType` enum
  - add to `Entity` and `EntityMut` enums
  - bunch of places where you'll need to handle the new enum values
  - implement `ToEntity`
  - implement `FindEntity`
  - add `subtype` names in `api/entity.rs`
  - add subtype to `try_from` for `EntityType`
  - consider valid subtypes, add to doc comment in `api/entity_id`
- view type
  - add to `NamespaceChild` enum
  - add find/get/iterate methods to `Namespace`
  - add `Transform` type
  - add xform to `Transforms`
  - handle transform filtering in `NamespaceTransform`
  - bunch of places where you'll need to handle the new enum values
- api validation
  - add relevant errors to `ValidationError` enum
  - add additional validation steps to `api/validate` and hook up in `model/builder` build method
  - make sure to run `qualify_type` on any `Type`s during validation
  - add support to `model::Api::find_qualified_type_relative` if necessary
- parser
  - add support to the builtin rust parser
- add tests to support everything ^
- add docs to explain everything ^
- add examples at least to the "fake platform" example

## Parsing with `chumsky`

### `recursive`

This method is a bit hard to wrap the brain around. I find it helpful to try to think of your `nested` parser as
just-another-parser rather than trying to consider how the recursion actually works. You can also try manually writing out
one or two recursions by copy-pasting the main block to make sure it makes sense.

**_You should really use a helper enum_**.

This makes a world of difference by creating a clear layer between each recurse.

```rust
// Helper enum! Use one!
enum Section<'a> {
    Text(&'a str),
    Number(i32),
    Nested(Vec<Section<'a>>),
}

// Parses recursive lists of numbers and/or idents
// e.g. [123, hello, [4, 5, hi], [6], 7, 8]
fn my_parser<'a>() -> impl Parser<'a, &'a str, Vec<Section<'a>>, Error<'a>> {
    recursive(|nested| {
        choice((
            nested.map(Section::Nested), // <------- make sure to map your nested parser into the enum.
            text::ident().padded().map(Section::Text),
            text::digits(10)
                .padded()
                .slice()
                .map(|s: &str| Section::Number(s.parse().unwrap())),
        ))  // <------- make sure everything in your `choice` returns the same type so you can `collect` them.
            //          if you always map into a helper enum, it becomes very clear.
            .separated_by(just(','))
            .collect::<Vec<_>>() // <--------------- this is the type that the `nested` parser returns.
            .boxed()             // <------- `recursive` parsers can hit the type length limit fairly easily.
                                 //          if you can't figure out what's wrong, try boxing.
            .delimited_by(just('{'), just('}'))
    })
}
```
