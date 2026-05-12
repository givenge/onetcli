ALTER TABLE chat_sessions ADD COLUMN session_kind TEXT NOT NULL DEFAULT 'ai';
UPDATE chat_sessions SET session_kind = 'ai' WHERE session_kind IS NULL OR session_kind = '';
CREATE INDEX IF NOT EXISTS idx_chat_sessions_kind ON chat_sessions (session_kind);
