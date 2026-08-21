<?php

declare(strict_types=1);

namespace Tests\Fixtures\Filament\Resources;

use Filament\Resources\Resource;
use Tests\Fixtures\Models\Post;

class PostResource extends Resource
{
    protected static ?string $model = Post::class;
}
