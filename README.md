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
- `--schema-json` read columns from a schema dump instead of parsing migrations
- `--no-live` skip the live database and parse migrations (handled by `vendor/bin/model-schema`)

### Live database (default)

When `vendor/bin/model-schema check` runs in a Laravel app whose database is reachable, the PHP wrapper boots the app, dumps the current connection's tables with `Schema::getColumns()`, and passes the dump to the binary as `--schema-json`. The live schema is the source of truth, so migrations that drop columns, add them in helper methods, or re-add them in `down()` can't skew the report.

If there is no app or the database is unreachable (for example on CI without a database), it prints why and parses migrations instead. Opt out with `--no-live` or `MODEL_SCHEMA_LIVE=0`. Passing your own `--schema-json` also skips the live dump.

### `--schema-json` format

```json
{
  "users": {
    "id": { "name": "id", "type": "bigint", "nullable": false },
    "email": { "name": "email", "type": "varchar", "nullable": false },
    "is_admin": { "name": "is_admin", "type": "tinyint(1)", "nullable": false },
    "created_at": { "name": "created_at", "type": "timestamp", "nullable": true }
  }
}
```

`type` can be the raw database name (Laravel's `Schema::getColumns()` `type_name`, with `tinyint(1)` for MySQL booleans) or an already-normalized type (`int`, `string`, `bool`, `float`, `Carbon`, `array`, `resource`). Raw names go through your `column_type_mappings` first (case-insensitive), then the built-in database names; anything unrecognized is kept as-is.

v1 is report-only. Automatic `--fix` flags are not included yet.

## Configuration

Copy `model-schema.example.toml` to the project root as `model-schema.toml`. Every key is optional: file values overlay Laravel defaults (lists and maps merge; `ignore.defaults = false` replaces built-in ignores).

```toml
[ignore]
tables = ["telescope_entries"]

[ignore.models]
fields = ["uuid"]

[unignore]
tables = ["jobs"]

[[ignore.rules]]
tables = ["users"]
fields = ["remember_token"]
checks = ["filament"]
```

- `[ignore]` / `[ignore.filament|models|phpdoc]` skip tables or fields
- `[unignore]` re-enables a default skip
- `[[ignore.rules]]` scopes a field to specific tables and checks
- `[checks]` can turn off `filament`, `models`, or `phpdoc` entirely
- `[models]`, `[filament]`, `[migrations]`, and `*_mappings` extend parser vocabulary


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
