---
default: minor
closes: []
---

`test_paths` of `embodiment.back-pointer-resolution` accepts globs as well as prefixes, so test files that sit beside the code they test and are named by suffix can be declared with `test_paths: ["**/*.test.*"]`. An entry with no glob character is a prefix exactly as before; `*` stays inside one path component and `**` crosses them. A glob is read among the files under `back_pointer_paths`, a glob matching none of them is reported, and one that cannot compile is refused at load.
