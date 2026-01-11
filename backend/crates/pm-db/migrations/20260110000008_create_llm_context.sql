-- LLM Context
CREATE TABLE pm_llm_context (
    id TEXT PRIMARY KEY,
    context_type TEXT NOT NULL CHECK(context_type IN ('schema_doc', 'query_pattern', 'business_rule', 'example', 'instruction')),

    category TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,

    example_sql TEXT,
    example_description TEXT,

    priority INTEGER NOT NULL DEFAULT 0,

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
);

-- Indexes
CREATE INDEX idx_pm_llm_context_type ON pm_llm_context(context_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_llm_context_category ON pm_llm_context(category) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_llm_context_priority ON pm_llm_context(priority DESC) WHERE deleted_at IS NULL;