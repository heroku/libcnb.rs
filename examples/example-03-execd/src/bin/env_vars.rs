use libcnb::execd_binary_main;
use std::collections::HashMap;

execd_binary_main!(HashMap::from([
    ("FOO", "bar"),
    ("BAR", "baz"),
    ("HELLO", "world"),
]));
