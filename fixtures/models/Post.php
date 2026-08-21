<?php

declare(strict_types=1);

namespace Tests\Fixtures\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\Pivot;
use Illuminate\Notifications\DatabaseNotification as BaseDatabaseNotification;

class User extends Model
{
}

class Visibility
{
}

class Post extends Model
{
    protected $fillable = ['title'];

    public function user(): BelongsTo
    {
        return $this->belongsTo(User::class);
    }

    public function getStatusAttribute(): string
    {
        return 'draft';
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'published_at' => 'datetime',
            'visibility' => Visibility::class,
        ];
    }
}

class Comment extends Model
{
    protected $casts = [
        'edited_at' => 'datetime',
    ];
}

class PostTag extends Pivot
{
    protected $table = 'post_tag';

    protected $fillable = ['post_id', 'tag_id'];
}

class DatabaseNotification extends BaseDatabaseNotification
{
}
