# stackfmt
Creates formatted string from `format_args!()` like `alloc::fmt::format()` but 
without allocation:

```rust
let mut buf = [0u8; 64];
let formatted: &str = stackfmt::fmt_truncate(&mut buf, format_args!("Hello{}", 42));
assert_eq!(formatted, "Hello42");
```

Implemented based on this SO answer [https://stackoverflow.com/a/50201632/601298](https://stackoverflow.com/a/50201632/601298)

