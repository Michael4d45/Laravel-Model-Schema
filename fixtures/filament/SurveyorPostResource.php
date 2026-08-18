<?php

declare(strict_types=1);

namespace Tests\Fixtures\Filament\Resources;

use Filament\Resources\Resource;
use Tests\Fixtures\Models\SurveyorPost;

class SurveyorPostResource extends Resource
{
    protected static ?string $model = SurveyorPost::class;
}
