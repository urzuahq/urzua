---
default: patch
closes: [MILE-190, MILE-193, MILE-194, MILE-196, MILE-203, MILE-214]
---

No change to what the CLI prints. A repository that declares none of the opt-in options (`level: info`, `roadmap.work_types`, `scope: changed`, `statuses.delivery.items`) now has a regression test holding its `check`, `check --base`, `roadmap`, `export --format json`, `fix` and `explain` output byte for byte to what those commands printed before the options existed. The same release adds tests that run `urzua audit`, `urzua check --rules` and the key-absent `roadmap` and `export` over this repository's own corpus, show that `scope: changed` leaves a rule's suggested actions as they were, and run `urzua explain` over the Done-means-items acceptance corpus.
