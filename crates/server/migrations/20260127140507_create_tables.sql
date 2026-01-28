-- Themes table
CREATE TABLE IF NOT EXISTS themes (
    id SERIAL PRIMARY KEY,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Votes table
CREATE TABLE IF NOT EXISTS votes (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    theme_id INTEGER NOT NULL REFERENCES themes(id),
    vote_type TEXT NOT NULL CHECK (vote_type IN ('yes', 'no', 'skip')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, theme_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_votes_user_id ON votes(user_id);
CREATE INDEX IF NOT EXISTS idx_votes_theme_id ON votes(theme_id);
CREATE INDEX IF NOT EXISTS idx_votes_vote_type ON votes(vote_type);