# laravel-model-schema

Report drift between Laravel migrations, Eloquent models, and Filament resource forms.

Analysis runs in a native Rust binary (`model-schema`) using [Mago](https://mago.carthage.software) to parse PHP. The Composer package is a thin `vendor/bin` stub (same shape as Mago): the first run downloads the matching GitHub release archive and caches it. Later runs reuse the cache. There is no Artisan command and no npm package.

```bash
composer require --dev michael4d45/laravel-model-schema
vendor/bin/model-schema check
```

If `model-schema.toml` exists in the working directory it is loaded automatically.

If GitHub's anonymous rate limit blocks the first download (common on shared CI), pass a token:

```yaml
- run: vendor/bin/model-schema check
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Command

```bash
vendor/bin/model-schema check
vendor/bin/model-schema check --config model-schema.toml
vendor/bin/model-schema check --json-only
vendor/bin/model-schema check --output reports/model_schema_report.json
```

- `--config` / `-c` TOML or JSON config (defaults to `./model-schema.toml` when present)
- `--output` JSON report path (default `reports/model_schema_report.json`)
- `--json-only` print the actionable report JSON only
- `--schema-json` optional live schema dump (skips migration parse)

v1 is report-only. Automatic `--fix` flags are not included yet.

## Configuration

Copy `model-schema.example.toml` to the project root as `model-schema.toml`.

## Releases

Tag a Cargo version with a `v` prefix. `Cargo.toml` `0.1.0` → `git tag v0.1.0`.

CI publishes archives named like Mago's rustc triples:

```text
model-schema-0.1.0-x86_64-unknown-linux-musl.tar.gz
model-schema-0.1.0-x86_64-unknown-linux-gnu.tar.gz
model-schema-0.1.0-aarch64-unknown-linux-musl.tar.gz
model-schema-0.1.0-aarch64-unknown-linux-gnu.tar.gz
model-schema-0.1.0-aarch64-apple-darwin.tar.gz
model-schema-0.1.0-x86_64-apple-darwin.tar.gz
```

`vendor/bin/model-schema` detects arch + libc (musl vs gnu, so Lerd/Alpine gets musl), downloads that archive from `releases/download/v{version}/`, and extracts it next to the stub.

Local path installs also look at `target/<triple>/release` from `cargo build`, so you do not need a GitHub release while developing.

```bash
cargo build --release --target x86_64-unknown-linux-musl   # Lerd / Alpine
cargo build --release                                      # host glibc
```

Override with `MODEL_SCHEMA_BIN` if you want a specific executable.

## License

MIT
