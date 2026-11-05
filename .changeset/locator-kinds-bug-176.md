---
default: minor
---

`embodiment.locator-exists` takes two options, `locator_kinds` and `pinnable`, each a list of `file`, `directory` and `glob` that defaults to `[file]`. A repository that opts in may cite a directory or glob in `Realized-by`: it is legal when it covers at least one tracked file, its drift uses the newest commit among those files, a pin on it hashes the sorted list of each file's path and content hash, and `urzua explain` and `urzua fix --rebaseline` show the files a pin covers. A locator of a kind the repository has not declared is reported, naming the option.
