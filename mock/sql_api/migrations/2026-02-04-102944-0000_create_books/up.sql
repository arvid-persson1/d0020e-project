-- Your SQL goes here
CREATE TYPE book_format_type AS ENUM ('Pdf', 'Docx', 'Epub', 'Hardcover', 'Paperback');

CREATE TABLE books (
  title VARCHAR NOT NULL,
  author VARCHAR NOT NULL,
  format book_format_type NOT NULL,
  isbn VARCHAR PRIMARY KEY
);
