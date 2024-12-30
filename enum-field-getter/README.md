# `enum-field-getter`

A simple derive macro used to implement methods to access fields which are of the same type across every tuple/struct enum variant.

## Usage

Derive `EnumFieldGetter`. For tuple enum variants, produces `get_n(&self) -> Option<&_>` and `get_n_mut(&mut self) -> Option<&mut _>` methods for each tuple member (starting from 0); for struct variants, getters are of the form `prop(&self) -> Option<&_>` and `prop_mut(&mut self) -> Option<&mut _>`. Getters are only produced so long as that member is the same type across all enum variants - if they are of different types, no getter will be produced for that member. These methods are produced even if that member doesn't exist for all enum variants - in the case that it doesn't exist, the getter will return `None`.

## Examples

```rust
use enum_field_getter::EnumFieldGetter;

#[derive(EnumFieldGetter)]
enum Foo {
    Bar(u32),
    Baz(u32, u32),
}

let foo = Foo::Bar(16);
let foo0 = foo.get_0();
assert_eq!(foo0, Some(&16));
let foo1 = foo.get_1();
assert!(foo1.is_none());
```

```rust
use enum_field_getter::EnumFieldGetter;

#[derive(EnumFieldGetter)]
enum Boop {
    Moo {
        a: i32,
        b: i32,
    },
    Baa {
        a: i32,
        b: i32,
        c: i32,
    }
}

let mut boop = Boop::Baa { a: 0, b: 42, c: 180 };
let boop_a = boop.a();
assert_eq!(boop_a, Some(&0));
let boop_c = boop.c();
assert_eq!(boop_c, Some(&180));
*boop.b_mut().unwrap() = 43;
assert_eq!(boop.b(), Some(&43));
```