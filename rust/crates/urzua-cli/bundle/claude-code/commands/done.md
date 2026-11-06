---
description: Mark delivered work finished by recording its evidence and letting urzua fix compute the state
argument-hint: <record path>
---

# /done

Finish a record whose state the configuration derives from its evidence (a built piece of work, or a bug
fixed in the change you are making). `$ARGUMENTS` is the record's path.

This command never writes a state itself and never takes one from you. Record the evidence first: the
code and test locators the record's evidence field holds, which `urzua explain` and the configuration's
relation fields name, and for a fix the regression test. Then it runs `urzua fix --apply`, so the state
written is the one the engine computes from that evidence. A state an `authority` gate names is a
person's: the engine refuses to write it and so does this command. A type whose state is not derived from
evidence has nothing computed, and the command says so and stops.

## Steps

1. Add the evidence to the record's evidence field, from what this change actually delivered. Do not
   touch any state value.
2. Apply. This runs `urzua fix --apply --ids RECORD --actor agent`, stages the record and runs
   `urzua check` on it with its examined count.

   ```bash step
   .claude/hooks/urzua-record.sh done "$RECORD"
   ```

3. If the command reports the engine computed nothing, the evidence is missing or does not reach the tier
   the configuration requires: add it and run step 2 again. Report the path, what the engine wrote and
   the `check` result.
