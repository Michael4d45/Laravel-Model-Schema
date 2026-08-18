<?php

declare(strict_types=1);

namespace LaravelModelSchema;

use Composer\InstalledVersions;
use PharData;
use RuntimeException;
use Throwable;

class NativeBinary
{
    private const GITHUB_REPO = 'Michael4d45/laravel-model-schema';
    private const PACKAGE_NAME = 'michael4d45/laravel-model-schema';

    public static function resolve(string $packageRoot): string
    {
        $fromEnv = getenv('MODEL_SCHEMA_BIN');
        if (is_string($fromEnv) && $fromEnv !== '' && self::isNativeExecutable($fromEnv)) {
            return $fromEnv;
        }

        foreach (self::localCargoCandidates($packageRoot) as $path) {
            if (self::isNativeExecutable($path)) {
                return $path;
            }
        }

        $version = self::installedVersion();
        $triple = self::rustTriple();
        $binDir = $packageRoot . '/php/bin';

        return self::ensureReleaseBinary($version, $triple, $binDir);
    }

    /**
     * @param  list<string>  $args
     */
    public static function execute(string $binary, array $args): never
    {
        if (function_exists('pcntl_exec')) {
            pcntl_exec($binary, $args);
            fwrite(STDERR, "Error: pcntl_exec failed for {$binary}\n");
            exit(1);
        }

        $command = escapeshellarg($binary);
        if ($args !== []) {
            $command .= ' ' . implode(' ', array_map(escapeshellarg(...), $args));
        }

        $pipes = [];
        $process = @proc_open(
            $command,
            [
                0 => ['file', 'php://stdin', 'r'],
                1 => ['file', 'php://stdout', 'w'],
                2 => ['file', 'php://stderr', 'w'],
            ],
            $pipes,
        );
        if (!is_resource($process)) {
            fwrite(STDERR, "Error: unable to start model-schema process.\n");
            exit(1);
        }

        do {
            usleep(10_000);
            $status = proc_get_status($process);
        } while ($status['running']);

        $exitCode = $status['exitcode'];
        if ($status['signaled']) {
            $exitCode = $status['termsig'] + 128;
        }
        proc_close($process);
        exit($exitCode);
    }

    public static function rustTriple(): string
    {
        $arch = match (strtolower(php_uname('m'))) {
            'x86_64', 'amd64' => 'x86_64',
            'aarch64', 'arm64' => 'aarch64',
            default => throw new RuntimeException(
                'Unsupported architecture: ' . php_uname('m')
                . '. Pre-built binaries exist for x86_64 and aarch64.',
            ),
        };

        return match (PHP_OS_FAMILY) {
            'Linux' => "{$arch}-unknown-linux-" . self::linuxLibc(),
            'Darwin' => "{$arch}-apple-darwin",
            default => throw new RuntimeException(
                'Unsupported OS: ' . PHP_OS_FAMILY
                . '. Pre-built binaries exist for Linux and macOS.',
            ),
        };
    }

    /**
     * Composer/Cargo version without a leading `v` (e.g. "0.1.0").
     */
    public static function installedVersion(): string
    {
        if (class_exists(InstalledVersions::class)) {
            try {
                $pretty = InstalledVersions::getPrettyVersion(self::PACKAGE_NAME);
                if (is_string($pretty) && $pretty !== '' && !str_starts_with($pretty, 'dev-')) {
                    return ltrim($pretty, 'v');
                }
            } catch (\OutOfBoundsException) {
            }
        }

        $composerJson = dirname(__DIR__, 2) . '/composer.json';
        if (is_file($composerJson)) {
            $json = json_decode((string) file_get_contents($composerJson), true);
            $version = is_array($json) ? ($json['version'] ?? null) : null;
            if (is_string($version) && $version !== '') {
                return ltrim($version, 'v');
            }
        }

        throw new RuntimeException(
            'Could not determine michael4d45/laravel-model-schema version. Run `composer install`.',
        );
    }

    /**
     * @return list<string>
     */
    private static function localCargoCandidates(string $packageRoot): array
    {
        try {
            $triple = self::rustTriple();
        } catch (RuntimeException) {
            $triple = null;
        }

        $candidates = [];
        if (is_string($triple)) {
            $candidates[] = $packageRoot . '/target/' . $triple . '/release/model-schema';
            $candidates[] = $packageRoot . '/target/' . $triple . '/debug/model-schema';
        }

        // Host `cargo build --release` is glibc. Skip it on musl (Lerd/Alpine)
        // so we never exec a gnu binary inside Alpine.
        if (PHP_OS_FAMILY !== 'Linux' || self::linuxLibc() === 'gnu') {
            $candidates[] = $packageRoot . '/target/release/model-schema';
            $candidates[] = $packageRoot . '/target/debug/model-schema';
        }

        return $candidates;
    }

    private static function linuxLibc(): string
    {
        if (is_file('/lib/ld-musl-x86_64.so.1') || is_file('/lib/ld-musl-aarch64.so.1')) {
            return 'musl';
        }

        $ldd = (string) shell_exec('ldd --version 2>&1');
        if ($ldd !== '' && str_contains($ldd, 'musl')) {
            return 'musl';
        }

        return 'gnu';
    }

    private static function ensureReleaseBinary(string $version, string $triple, string $binDir): string
    {
        $storageDir = "model-schema-{$version}-{$triple}";
        $releaseDir = "{$binDir}/{$version}";
        $executable = "{$releaseDir}/{$storageDir}/model-schema";

        if (self::isNativeExecutable($executable)) {
            return $executable;
        }

        if (!is_dir($releaseDir) && !mkdir($releaseDir, 0o755, true) && !is_dir($releaseDir)) {
            throw new RuntimeException("Unable to create {$releaseDir}");
        }

        $lockFile = "{$releaseDir}/.model-schema-{$triple}.lock";

        return self::locked($lockFile, static function () use (
            $version,
            $triple,
            $storageDir,
            $releaseDir,
            $executable,
        ): string {
            if (self::isNativeExecutable($executable)) {
                return $executable;
            }

            $archive = "{$releaseDir}/{$storageDir}.tar.gz";
            $url = sprintf(
                'https://github.com/%s/releases/download/%s/%s.tar.gz',
                self::GITHUB_REPO,
                'v' . $version,
                $storageDir,
            );

            fwrite(STDERR, "Downloading model-schema {$version} for {$triple}...\n");
            self::downloadToFile($url, $archive);
            self::extractTarGz($archive, $releaseDir);
            @unlink($archive);

            if (!self::isNativeExecutable($executable)) {
                throw new RuntimeException(
                    "Expected binary not found after extraction at {$executable}. "
                    . "For a path install, run `cargo build --release --target {$triple}`.",
                );
            }

            chmod($executable, 0o755);

            return $executable;
        });
    }

    /**
     * @template T
     * @param  callable(): T  $callback
     * @return T
     */
    private static function locked(string $lockFile, callable $callback): mixed
    {
        $handle = fopen($lockFile, 'c');
        if ($handle === false) {
            throw new RuntimeException("Unable to create lock file: {$lockFile}");
        }

        flock($handle, LOCK_EX);
        try {
            return $callback();
        } finally {
            flock($handle, LOCK_UN);
            fclose($handle);
        }
    }

    private static function downloadToFile(string $url, string $destination): void
    {
        if (function_exists('curl_init')) {
            $ch = curl_init($url);
            $fh = fopen($destination, 'w');
            if ($ch === false || $fh === false) {
                throw new RuntimeException("Failed to download {$url}");
            }
            $headers = ['User-Agent: laravel-model-schema'];
            $token = self::githubToken();
            if ($token !== null) {
                $headers[] = 'Authorization: Bearer ' . $token;
            }
            curl_setopt_array($ch, [
                CURLOPT_FOLLOWLOCATION => true,
                CURLOPT_FILE => $fh,
                CURLOPT_HTTPHEADER => $headers,
                CURLOPT_TIMEOUT => 120,
            ]);
            $ok = curl_exec($ch);
            $status = (int) curl_getinfo($ch, CURLINFO_HTTP_CODE);
            $error = curl_error($ch);
            curl_close($ch);
            fclose($fh);
            if ($ok === false || $status >= 400) {
                @unlink($destination);
                throw new RuntimeException("Failed to download {$url} (HTTP {$status}): {$error}");
            }

            return;
        }

        $headers = "User-Agent: laravel-model-schema\r\n";
        $token = self::githubToken();
        if ($token !== null) {
            $headers .= 'Authorization: Bearer ' . $token . "\r\n";
        }
        $context = stream_context_create([
            'http' => [
                'follow_location' => 1,
                'timeout' => 120,
                'header' => $headers,
            ],
        ]);
        $body = @file_get_contents($url, false, $context);
        if (!is_string($body) || $body === '') {
            throw new RuntimeException("Failed to download {$url}");
        }
        if (file_put_contents($destination, $body) === false) {
            throw new RuntimeException("Unable to write {$destination}");
        }
    }

    private static function githubToken(): ?string
    {
        foreach (['GITHUB_TOKEN', 'GH_TOKEN'] as $variable) {
            $value = getenv($variable);
            if (is_string($value) && $value !== '') {
                return $value;
            }
        }

        return null;
    }

    private static function extractTarGz(string $archive, string $destination): void
    {
        try {
            $phar = new PharData($archive);
            $phar->extractTo($destination, null, true);
        } catch (Throwable $e) {
            throw new RuntimeException('Failed to extract release archive: ' . $e->getMessage(), 0, $e);
        }
    }

    private static function isNativeExecutable(string $path): bool
    {
        if (!is_file($path) || !is_readable($path)) {
            return false;
        }
        if (!is_executable($path)) {
            return false;
        }

        $handle = fopen($path, 'rb');
        if ($handle === false) {
            return false;
        }
        $magic = (string) fread($handle, 4);
        fclose($handle);

        return $magic !== '' && !str_starts_with($magic, '#!');
    }
}
