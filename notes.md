# Notes
> ## Deps
>> ### sqlx-cli
>>> Install
>>> ```bash
>>> cargo install sqlx-cli --no-default-features --features sqlite
>>> ```
>>
> ## Temp Database
>>
>> ### Change Directory
>>> ```bash
>>> cd backend/crates/pm-db
>>> ```
>>
>> ### Environment Variable 
>>> ```bash
>>> export DATABASE_URL="sqlite:/Users/tony/git/blazor-agile-board/backend/crates/pm-db/.sqlx-test/test.db"
>>> ```
>>
>> ### Create Database
>>> ```bash
>>> sqlx database create
>>> ```
>>
>> ### Migrate
>>> ```bash
>>> sqlx migrate run
>>> ```
>>
>> ### Prepare the query cache
>>> ```bash
>>> cargo sqlx prepare
>>> ```
>>> 
>>> This generates .sqlx/query-*.json files that let SQLx verify queries at compile time WITHOUT needing the database.