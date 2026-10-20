# EXOPTICON

## Development

### Building

A development build can be performed with `make`. This builds everything and produces a dev binary at `target/debug/exopticon`.

```
$ make
```

A release build can be created with:

```
$ make release
```

Any changes must also pass clippy and format checks. These can be run with:

```
$ make clippy
$ make check-format
```

Formatting can be applied with:

```
$ make format
```

The ci pipeline can be tested with:

```
make ci-flow
```

### Writing Tests

#### General Guidelines

Automated tests are an important part of EXOPTICON development. Here
are some general guidelines for creating tests in the project:

1. Keep tests narrow
   Each test should be focused on a particular behavior or concept.
2. Assert observable results
   Check returned values and resulting state; avoid prescribing internal
   method calls.
3. Use arrange-act-assert structure
4. Use helpers with sensible defaults
   Provide every value relevant to the behavior under test.
5. Keep tests deterministic.
   Control external state and time where they affect the result.

#### Specific Conventions

Here are some specific conventions to handle common test scenarios:

1. Name the rule being demonstrated.
   Prefer touching_recording_ranges_are_merged over a name listing
   several operations. Multiple assertions are fine when they establish
   one rule.

2. Make meaningful data visible.
   Hide migrations and irrelevant camera configuration in fixtures.
   Keep camera identity, recording state, and relevant timestamps
   explicit.

3. Express time relationships with offsets.
   Use a fixed base and at(seconds) so touching intervals, overlaps, and
   gaps are immediately apparent. Specify the unit in the helper’s name
   or signature.

4. Choose examples that distinguish correct from incorrect behavior.
   Test boundary exclusion separately when another interval could
   conceal an incorrectly included boundary row.


5. Assert the entire promised effect.
   A cleanup test should verify deletion; a rejection assertion only
   establishes rejection.

6. Keep expected results independent.
    Small timestamp helpers are useful, but expected ranges should be
    written explicitly rather than calculated with another
    implementation of the merging algorithm.

7. Keep real SQLite tests focused and isolated.
    Retain temporary databases and real migrations for persistence
    behavior. Create physical video files only when filesystem
    behavior matters.

Can someone understand the rule, the significant inputs, and the
expected outcome by reading this test alone?


