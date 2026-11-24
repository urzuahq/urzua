---
default: minor
closes: [MILE-367]
---

Every command's JSON now carries `next_action_argv`, the next action as an array of words with no quoting (`null` exactly when `next_action` is). `next_action` is those words joined and quoted for a POSIX shell, so a placeholder is single-quoted everywhere (`urzua explain --type '<name>'`, and the `--reason` of `fix --pin`, which was double-quoted). Each `next_action_inputs` entry gains `index`, the position of its placeholder in the array. A `doctor` check's `fix` is printed beside `fix_argv` and is always one command: a hook with two blocks is fixed by `urzua hooks install`, and a hook without the execute bit by `chmod +x`, not by a `&&` chain. The heading that `fix` offers to append to a record is an `sh -c` command rather than a shell redirection.
