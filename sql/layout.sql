CREATE SCHEMA "global";
CREATE SCHEMA users;

CREATE TABLE "global".activity_data (
	id serial,
	"name" varchar NOT NULL,
	alias _text NULL,
	CONSTRAINT activity_data_pkey PRIMARY KEY (id, "name")
);

CREATE TABLE "global".activity_history (
	user_id varchar NOT NULL,
	activity int8 NOT NULL,
	time_played float8 DEFAULT 0 NOT NULL,
	CONSTRAINT activity_history_pkey PRIMARY KEY (user_id, activity)
);

CREATE TABLE "global".activity_time_history (
	user_id varchar NOT NULL,
	"date" date NOT NULL,
	time_played float8 NOT NULL,
	CONSTRAINT activity_time_history_pkey PRIMARY KEY (user_id, date)
);

CREATE TABLE "global".activity_user_data (
	user_id varchar NOT NULL,
	last_played_activity int8 NULL,
	last_played_time_played float8 NULL,
	current_activity int8 NULL,
	current_activity_start_time timestamp NULL,
	CONSTRAINT activity_user_data_pkey PRIMARY KEY (user_id)
);

CREATE TABLE "global".guild_data (
	guild_id varchar NOT NULL,
	"name" varchar NOT NULL,
	icon varchar NULL,
	banner varchar NULL,
	stat_exclusion_channels _varchar NOT NULL,
	CONSTRAINT guild_data_pkey PRIMARY KEY (guild_id)
);

CREATE TABLE "global".roles (
	role_id varchar NOT NULL,
	guild_id varchar NOT NULL,
	creator_id varchar NULL,
	"name" varchar NULL,
	color varchar NOT NULL,
	is_admin bool NOT NULL,
	CONSTRAINT roles_pkey PRIMARY KEY (role_id)
);

CREATE TABLE users.user_assets (
	user_id varchar NOT NULL,
	guild_id varchar NOT NULL,
	asset text NOT NULL,
	CONSTRAINT user_assets_pkey PRIMARY KEY (user_id, guild_id)
);

CREATE TABLE users.user_data (
	user_id varchar NOT NULL,
	guild_id varchar NOT NULL,
	username varchar NOT NULL,
	display_name varchar NULL,
	global_name varchar NULL,
	nickname varchar NULL,
	avatar varchar NULL,
	banner varchar NULL,
	message_count int8 NOT NULL,
	voice_time float8 NOT NULL,
	total float8 NOT NULL,
	voice_channel_id varchar NULL,
	voice_channel_join_time varchar NULL,
	join_date timestamptz NULL,
	creation_date timestamptz NULL,
	user_left bool DEFAULT false NOT NULL,
	leave_date timestamptz NULL,
	leave_deletion_duration numeric DEFAULT 14 NOT NULL,
	message_data bool DEFAULT true NOT NULL,
	voice_data bool DEFAULT true NOT NULL,
	game_data bool DEFAULT true NOT NULL,
	CONSTRAINT user_data_pkey PRIMARY KEY (user_id, guild_id)
);

CREATE TABLE users.voice_message_history (
	user_id varchar NOT NULL,
	guild_id varchar NOT NULL,
	"date" date NOT NULL,
	message_count int8 NOT NULL,
	voice_time float8 NOT NULL,
	CONSTRAINT voice_message_history_pk PRIMARY KEY (user_id, guild_id, date)
);

ALTER TABLE "global".activity_data OWNER TO postgres;
ALTER TABLE "global".activity_history OWNER TO postgres;
ALTER TABLE "global".activity_time_history OWNER TO postgres;
ALTER TABLE "global".activity_user_data OWNER TO postgres;
ALTER TABLE "global".guild_data OWNER TO postgres;
ALTER TABLE "global".roles OWNER TO postgres;
ALTER TABLE users.user_assets OWNER TO postgres;
ALTER TABLE users.user_data OWNER TO postgres;
ALTER TABLE users.voice_message_history OWNER TO postgres;
