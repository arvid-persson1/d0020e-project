-- Your SQL goes here
CREATE TYPE book_format_type AS ENUM ('Pdf', 'Docx', 'Epub', 'Hardcover', 'Paperback');

CREATE TABLE books (
  title VARCHAR NOT NULL,
  author VARCHAR NOT NULL,
  format book_format_type NOT NULL,
  isbn VARCHAR PRIMARY KEY
);

INSERT INTO books (isbn, title, author, format) VALUES
    ('9788316497541', 'The Last Wish: Introducing the Witcher', 'Andrzej Sapkowski', 'Hardcover'),
    ('0000000000001', 'Blood of Elves', 'Andrzej Sapkowski', 'Paperback'),
    ('0000000000002', 'Time of Contempt', 'Andrzej Sapkowski', 'Epub');

CREATE ROLE mock_reader WITH LOGIN PASSWORD 'mock';

GRANT USAGE ON SCHEMA public TO mock_reader;

GRANT SELECT ON books TO mock_reader;
