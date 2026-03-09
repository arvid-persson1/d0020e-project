-- This file should undo anything in `up.sql`
DROP OWNED BY mock_reader;
DROP ROLE IF EXISTS mock_reader;

-- Drop the table
DROP TABLE IF EXISTS books;

-- Drop the custom type LAST
DROP TYPE IF EXISTS book_format_type;
