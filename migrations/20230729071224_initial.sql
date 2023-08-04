CREATE TABLE todonts (
	id          INTEGER PRIMARY KEY,
	description TEXT NOT NULL,
	done        BOOLEAN NOT NULL DEFAULT FALSE
);
