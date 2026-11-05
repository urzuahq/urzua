---
default: minor
closes: []
---

`urzua new` no longer reuses a number already taken on another branch. Besides the working tree, it
now looks at the type's directory on every local branch and remote-tracking ref, takes the next
number above the highest found, and prints to stderr which ref(s) already used each number it
skipped. This uses git only, with no network, so a branch that was never fetched into your clone is
still invisible: run `git fetch` first. If git cannot be read, `new` numbers from the working tree
as before and says so on stderr.
