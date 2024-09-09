CREATE TABLE
    messages (
        message_id UUID NOT NULL PRIMARY KEY,
        username VARCHAR(20) NOT NULL,
        message_content TEXT NOT NULL,
        publish_date TIMESTAMPTZ NOT NULL
    );
