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
