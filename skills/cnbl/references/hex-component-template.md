# Hexagonal Component Template

Canonical structure for a new hexagonal Rust component generated during the
port phase. Follow the writing-solid-rust skill for detailed conventions.

## Directory Layout

```
src/
  domain/
    <name>.rs       # Pure domain type — no I/O, no external deps
  ports/
    <name>.rs       # Trait defining the port (interface)
  adapters/
    <name>_<impl>.rs  # Concrete adapter implementing the port
```

## Domain Type

```rust
// src/domain/<name>.rs
// No external crate imports. Only std and your own domain types.

#[derive(Debug, Clone, PartialEq)]
pub struct <Name> {
    pub id: <NameId>,
    // fields...
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct <Name>Id(String);
```

## Port Trait

```rust
// src/ports/<name>.rs
use crate::domain::<Name>;

pub trait <Name>Repository {
    type Error: std::error::Error + Send + Sync + 'static;

    fn find_by_id(&self, id: &<Name>Id) -> Result<Option<<Name>>, Self::Error>;
    fn save(&self, item: &<Name>) -> Result<(), Self::Error>;
    fn delete(&self, id: &<Name>Id) -> Result<(), Self::Error>;
}
```

## Adapter

```rust
// src/adapters/<name>_<impl>.rs
use crate::domain::<Name>;
use crate::ports::<Name>Repository;

pub struct <Name><Impl>Adapter {
    // infrastructure handle (db connection, HTTP client, etc.)
}

impl <Name>Repository for <Name><Impl>Adapter {
    type Error = <ImplError>;

    fn find_by_id(&self, id: &<Name>Id) -> Result<Option<<Name>>, Self::Error> {
        // implementation
    }
    // ...
}
```

## Rules

- Domain types must not import infrastructure crates (sqlx, reqwest, serde, etc.)
- Ports are traits only — no implementation
- Adapters implement exactly one port each
- Use `thiserror` for adapter error types
- Use `#[cfg(test)]` + `mockall` for port mocks in unit tests
