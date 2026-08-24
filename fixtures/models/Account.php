<?php

declare(strict_types=1);

namespace Tests\Fixtures\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;

/**
 * @property-read \Illuminate\Database\Eloquent\Collection<int, \Tests\Fixtures\Models\AccountBalance> $balances
 */
class Account extends Model
{
    public function balances(): HasMany
    {
        return $this->hasMany(AccountBalance::class);
    }
}
