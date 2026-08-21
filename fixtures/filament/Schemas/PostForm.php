<?php

declare(strict_types=1);

namespace Tests\Fixtures\Filament\Resources\Schemas;

use Filament\Forms\Components\TextInput;
use Filament\Schemas\Schema;

class PostForm
{
    public static function configure(Schema $schema): Schema
    {
        return $schema->components([
            TextInput::make('title')->required(),
            TextInput::make('legacy_field'),
        ]);
    }
}
