# pyx

Pyx in an Interface Definition Language (IDL) built with the explicit purpose of supporting every feature
that apyxl supports. Originally apyxl was using the rust parser within tests to quickly set up APIs, but
as development on other languages and use cases continued it became clear there are many things rust does
_not_ support. Pyx grew out of that need.

As such, pyx is extremely close to rust. It differs only where it needs to for the sake of apyxl features.
These differences are iterated in this document.

## Development Checklist

- [ ] compare rust/c#
    - consider if impl blocks and partial classes are both covered here...?
- [ ] ty alias in dto
- [ ] field inside dto
- [ ] rpc inside dto
- [ ] enum inside dto
- [ ] enum inside impl block
- [ ] nested dtos
- [ ] nested dtos in impl block
- [ ] event type???
- [ ] what can be shared?

## Differences from Rust

The biggest difference is that pyx is not a programming language, and does not support any sort of logic or
execution.

### DTO children can be defined inside DTO or impl block

```rust
#![rustfmt::skip]

struct Dto {
    type Alias = u32;
    const static_field: u32 = 0;
    fn static_rpc() {}
    fn rpc(self) {}

    normal_field: u32;
}

impl Dto {
    // still works
    fn rpc2() {}
}
```

### DTOs or enums inside DTOs (incl nested DTOs)

```rust
#![rustfmt::skip]

struct Dto {
    struct Nested {
        struct Nested2 {
            field: u32;
        }
        enum En {}
    }
}

impl Dto {
    struct Nested3 {
        struct Nested4 {
            field: u32;
        }
    
        enum En2 {}
    }
}
```

### First class event type

```rust
#![rustfmt::skip]

event<> ???;
```
