CREATE TABLE storage_type (
    id serial primary key,
    name text not null
);

CREATE TABLE video_file (
    id serial primary key,
    path text not null,
    name text not null,
    storage_type_id serial not null,
    label text not null,
    ts timestamp not null,
    created timestamp not null,
    modified timestamp not null,
--    duration ??,
    CONSTRAINT fk_storage_type
      FOREIGN KEY(storage_type_id) 
	  REFERENCES storage_type(id)
);

INSERT INTO storage_type (name)
VALUES 
('s3'), 
('local');