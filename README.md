# apyxl
apyxl is a command line tool that uses API defined by an Interface Definition Language (IDL), and generates corresponding API code in many output languages (or other such artifacts).

Example use cases for apyxl include:
- generating API bindings for SDKs in different languages with the same target API
- generating cross-language interop code e.g. between a Rust library and a C#
- generating documentation for an API
- generating a visual representation of class dependencies
- converting from one IDL to another e.g. protobuf to flatbuffers
- converting from programming language definitions to a data schema e.g. kotlin to protobuffers

## Built-in Support

Input:
- Rust

Output:
- Rust (stub methods)

## Usage

apyxl can be used in two ways, and has examples for each:
- Command line interface: [examples](examples)
- Rust library: [examples](apyxl/examples)

# Customizing

apyxl is built to support users writing their own **parsers** and **generators**.

## Writing a Parser

See the [rust parser](apyxl/src/parser/rust.rs) for a complete example using [chumsky](https://github.com/zesterer/chumsky). 

apyxl parsers can be written however you want, as long as they implement the trait [Parser](apyxl/src/parser/mod.rs).
If you're parsing a programming language or IDL, the included library [chumsky](https://github.com/zesterer/chumsky) is
a great option.

### Key Points

This is a list of things to keep in mind when writing a parser.

- Support all relevant [API model structs](apyxl/src/model/api)
  - Namespaces
  - DTOs, fields
  - RPCs, params, return types
  - Enums
  - Imports/includes
  - Comments (see [Attributes](apyxl/src/model/api/attribute.rs))
  - Types including primitives, arrays, maps, optionals
  - [User types](#user-types)
  - [User attributes](#attributes)
- [Chunks](#chunks)

### API Builder

API sources are typically split up into multiple files. Chunks are an abstraction around that idea that leave the door
open to receiving chunks from a source other than files, but you can just think of them as files. :)

The [api::Builder](apyxl/src/model/builder/mod.rs) is a temporary collector for chunks of your API. Each time you
finish parsing a chunk, you merge it into the `Builder`.

Once you have finished parsing the entire API, call `Builder::build()`. This method:
- Dedupes namespaces, i.e. creates a unified view of the entire API without chunk divisions
- [optionally] Prints the full API before validation (See [Debugging Validation Errors](#debugging-validation-errors))
- Performs a host of validations like checking for duplicate definitions and ensuring all types are valid primitives
or exist within the API.
- Fully qualifies all types within the API.

### User Types

When apyxl validates the API, it will error if the type is not a supplied primitive or a DTO/Enum in the API. User
types are a way to override this behavior and supply types that are known to exist but are not primitives,
e.g. built-in language types.

These are supplied via [Parser Config](#parser-config). Example:

```json
{
  "user_types": [
    {
      "parse": "MySpecialType",
      "name": "special"
    }
  ]
}
```

Generators can then look for the type specified by the `name` field (`special` in this example) to generate the
appropriate type on their end.

### User Attributes

Many languages have a way to specify custom attributes or annotations on various things. The `user` field inside
[Attributes](apyxl/src/model/api/attribute.rs) exists to support passing these through the API for use in your
generators.

These support various use cases e.g.:
```rust
#[name_only, list(a, b, c), map(a=1, b=2, c=3)]
struct Dto {}
```

### Parser Config

Parser config is an option set of configuration that parsers need to accept to support certain built-in features.
Parser config can be supplied to the CLI as a json file.

### Debugging Validation Errors

You can enable a debug printout of the entire API in the builder _before it is validated_ by setting the
`PreValidationPrint` option in the builder configuration.

## Writing a Generator

See the [rust generator](apyxl/src/generator/rust.rs) for a complete example.

Generators iterate over [views](#views) into the in-memory API model and write to an [Output](#output).
What you use the view for, and what you send to the `Output` is entirely up to you.

### Views

[Views](apyxl/src/view) are a set of structs that mirror the model, and provide an immutable view into the model. The
most important difference is that views can be [transformed](#transforms) to alter their view of the model, e.g. by filtering or
applying text casing changes, without modifying the model itself for other generators.

A built-in example of views is `view::Model::api_chunked_iter`, which provides an iterator over a set of views mapped
to each [chunk](#chunk) from the parser. This allows generating a file structure similar to the API files.

### Transforms

Transforms are traits that provide ways to alter the information coming the model that only apply to the current
generator's view of the model.

Each transformable type has a Transform trait e.g. `DtoTransform`. Implement this and then apply to the view via a
`Transformer` trait method such as `with_dto_transform(...)`.

Views are trivially cloneable so you can create as many views with different transforms as you need.

See also [Subview](apyxl/src/view/sub_view.rs) for another way of using views & transforms.

### Output

Output is how the generated content is written to a file or other destination. Typically, you'll be outputting to
a [FileSet](apyxl/src/output/file_set.rs), so you can consider `Output::write_chunk` to be your method for starting a
new file.

You can also use the Output [Buffer](apyxl/src/output/buffer.rs) if you are using the library programmatically and want
to do other things with the generated content.

#### Indentation

[Indented](apyxl/src/output/indent.rs) is a helper type that wraps an `Output` and keeps track of applying the current
indentation level to each line before passing it through to the `Output`.

See the [rust generator](apyxl/src/generator/rust.rs) for example usage.

### Debugging

You can use the Output type [StdOut](apyxl/src/output/stdout.rs) to pipe your generated output directly to stdout
instead of a file.

## Planned Feature Support

- RPCs associated with DTOs i.e. methods
- Union/oneof types
- Refactor out common chumsky helpers
- Applying transforms through configuration/cli
