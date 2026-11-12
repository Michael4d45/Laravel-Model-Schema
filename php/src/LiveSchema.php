<?php

declare(strict_types=1);

namespace LaravelModelSchema;

use Illuminate\Contracts\Console\Kernel;
use Illuminate\Database\Connection;
use RuntimeException;
use Throwable;

/**
 * Dumps the live database schema of the Laravel app in the working directory
 * into the JSON shape `model-schema check --schema-json` reads:
 *
 *     {"table": {"column": {"name": "column", "type": "bigint", "nullable": false}}}
 *
 * Types are the raw database names (`varchar`, `bigint`, `tinyint(1)`); the
 * binary maps them, honouring the project's `column_type_mappings`.
 */
final class LiveSchema
{
    /**
     * Whether `check` should try the live database: on by default, off with
     * `--no-live`, `MODEL_SCHEMA_LIVE=0`, or an explicit `--schema-json`.
     *
     * @param  list<string>  $args  CLI args; `--no-live` is removed in place.
     */
    public static function wanted(array &$args): bool
    {
        if (($args[0] ?? null) !== 'check') {
            return false;
        }

        $noLive = array_search('--no-live', $args, true);
        if ($noLive !== false) {
            array_splice($args, (int) $noLive, 1);

            return false;
        }

        foreach ($args as $arg) {
            if ($arg === '--schema-json' || str_starts_with($arg, '--schema-json=')) {
                return false;
            }
        }

        $env = getenv('MODEL_SCHEMA_LIVE');

        return !(is_string($env) && in_array(strtolower($env), ['0', 'false', 'no', 'off'], true));
    }

    /**
     * Boot the app in $root and write its schema to a temp JSON file.
     *
     * @return array{path: string, connection: string, tables: int}
     *
     * @throws RuntimeException when there is no Laravel app or no reachable database
     */
    public static function dump(string $root): array
    {
        $bootstrap = $root . '/bootstrap/app.php';
        if (!is_file($bootstrap)) {
            throw new RuntimeException('no Laravel app (bootstrap/app.php) in ' . $root);
        }

        try {
            $app = require $bootstrap;
            $app->make(Kernel::class)->bootstrap();
            /** @var Connection $connection */
            $connection = $app->make('db')->connection();
            $connection->getPdo();
        } catch (Throwable $e) {
            throw new RuntimeException('database not reachable: ' . $e->getMessage(), 0, $e);
        }

        $schema = $connection->getSchemaBuilder();
        $tables = [];

        foreach (self::tablesInCurrentConnection($connection) as $table) {
            $columns = [];
            foreach ($schema->getColumns($table) as $column) {
                $name = (string) $column['name'];
                $columns[$name] = [
                    'name' => $name,
                    'type' => self::rawType($column),
                    'nullable' => (bool) $column['nullable'],
                ];
            }
            ksort($columns);
            $tables[$table] = $columns;
        }
        ksort($tables);

        $path = rtrim(sys_get_temp_dir(), '/') . '/model-schema-live-' . md5($root) . '.json';
        $json = json_encode($tables, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        if (file_put_contents($path, $json . "\n") === false) {
            throw new RuntimeException('could not write ' . $path);
        }

        return [
            'path' => $path,
            'connection' => (string) $connection->getName(),
            'tables' => count($tables),
        ];
    }

    /**
     * Raw database type for one column; the binary maps it through
     * `column_type_mappings` and its built-in database names.
     *
     * @param  array<string, mixed>  $column  One row from Schema::getColumns()
     */
    private static function rawType(array $column): string
    {
        $typeName = is_string($column['type_name'] ?? null) ? strtolower($column['type_name']) : '';
        $fullType = is_string($column['type'] ?? null) ? strtolower($column['type']) : '';

        // MySQL booleans are tinyint(1); keep that apart from wider tinyints.
        if ($typeName === 'tinyint' && str_starts_with($fullType, 'tinyint(1)')) {
            return 'tinyint(1)';
        }

        return $typeName !== '' ? $typeName : 'mixed';
    }

    /**
     * Table names in this connection only. MySQL's `getTableListing()` with a
     * null schema lists every database the user can see.
     *
     * @return list<string>
     */
    private static function tablesInCurrentConnection(Connection $connection): array
    {
        $builder = $connection->getSchemaBuilder();
        $schemas = method_exists($builder, 'getCurrentSchemaListing')
            ? array_values(array_filter(
                (array) $builder->getCurrentSchemaListing(),
                static fn (mixed $schema): bool => is_string($schema) && $schema !== '',
            ))
            : [];

        if ($schemas === []) {
            $database = (string) $connection->getDatabaseName();
            if ($database === '') {
                throw new RuntimeException('cannot determine the current database schema');
            }
            $schemas = [$database];
        }

        $allowed = array_fill_keys($schemas, true);
        $listing = method_exists($builder, 'getTableListing')
            ? $builder->getTableListing($schemas, false)
            : array_column($builder->getTables(), 'name');

        $tables = [];
        foreach ($listing as $table) {
            if (!is_string($table) || $table === '') {
                continue;
            }
            if (str_contains($table, '.')) {
                $parts = explode('.', $table);
                $table = (string) end($parts);
                if ($table === '' || !isset($allowed[$parts[0]])) {
                    continue;
                }
            }
            $tables[] = $table;
        }

        return array_values(array_unique($tables));
    }
}
