-- public.sipin_sips definition

-- Drop table

-- DROP TABLE public.sipin_sips;

CREATE TABLE public.sipin_sips (
	row_id serial4 NOT NULL,
	correlation_id text NOT NULL, -- The correlation_id of this SIP-delivery.
	bag_name text NULL, -- Filename by which this bag was delivered/uploaded to the ingest host.
	cp_id text NULL, -- The ID for this CP within meemoo. Also known as OR-id.
	local_id varchar(255) NULL, -- The CP-provided main ID of the original carrier or borndigital item in their system of reference.
	md5_hash_sip text NULL, -- MD5 hash from the complete SIP package.
	md5_hash_essence_manifest varchar(32) NULL, -- MD5 hash as calculated by meemoo.
	md5_hash_essence_sidecar text NULL, -- MD5 hash as was delivered in the sidecar. No column constraints since any value can be provided in de sidecar.
	essence_filename text NULL, -- Filename of the main essence in this SIP.
	essence_filesize int8 NULL, -- Filesize of the main essence in this SIP.
	ingest_host text NULL, -- FQDN of the ingest host (scheme + host + domain).
	ingest_bucket text NULL, -- Bucket name in which the SIP was delivered (for S3). NULL for FTP.
	ingest_path_or_key text NULL, -- Fully qualified path (FTP) or key (S3) for the incoming object/SIP.
	bag_filesize int8 NULL, -- Filesize of the zipped bag.
	pid bpchar(10) NULL, -- Persistent Identifier. The main ID of the object within meemoo.
	mh_record_id bpchar(64) NULL, -- MediaHaven record ID.
	first_event_date timestamptz NOT NULL, -- Datetime for the first event for this correlation ID.
	last_event_type text NOT NULL, -- Last seen event type for this correlation ID.
	last_event_date timestamptz NOT NULL, -- Datetime for the last event for this correlation ID.
	status text NOT NULL, -- More human friendly status: correlates one-to-one with the last event type.
	CONSTRAINT sipin_sips_correlation_id_key UNIQUE (correlation_id),
	CONSTRAINT sipin_sips_mh_record_id_key UNIQUE (mh_record_id),
	CONSTRAINT sipin_sips_pid_key UNIQUE (pid),
	CONSTRAINT sipin_sips_pkey PRIMARY KEY (row_id)
);
CREATE INDEX sipin_sips_essence_filename_idx ON public.sipin_sips USING btree (essence_filename);
CREATE INDEX sipin_sips_md5_hash_sip_idx ON public.sipin_sips USING btree (md5_hash_sip);
CREATE INDEX sipin_sips_md5_hash_essence_manifest_idx ON public.sipin_sips USING btree (md5_hash_essence_manifest);

-- Column comments

COMMENT ON COLUMN public.sipin_sips.correlation_id IS 'The correlation_id of this SIP-delivery.';
COMMENT ON COLUMN public.sipin_sips.bag_name IS 'Filename by which this bag was delivered/uploaded to the ingest host.';
COMMENT ON COLUMN public.sipin_sips.cp_id IS 'The ID for this CP within meemoo. Also known as OR-id.';
COMMENT ON COLUMN public.sipin_sips.local_id IS 'The CP-provided main ID of the original carrier or borndigital item in their system of reference.';
COMMENT ON COLUMN public.sipin_sips.md5_hash_sip IS 'MD5 hash from the complete SIP package.';
COMMENT ON COLUMN public.sipin_sips.md5_hash_essence_manifest IS 'MD5 hash as calculated by meemoo.';
COMMENT ON COLUMN public.sipin_sips.md5_hash_essence_sidecar IS 'MD5 hash as was delivered in the sidecar. No column constraints since any value can be provided in de sidecar.';
COMMENT ON COLUMN public.sipin_sips.essence_filename IS 'Filename of the main essence in this SIP.';
COMMENT ON COLUMN public.sipin_sips.essence_filesize IS 'Filesize of the main essence in this SIP.';
COMMENT ON COLUMN public.sipin_sips.ingest_host IS 'FQDN of the ingest host (scheme + host + domain).';
COMMENT ON COLUMN public.sipin_sips.ingest_bucket IS 'Bucket name in which the SIP was delivered (for S3). NULL for FTP.';
COMMENT ON COLUMN public.sipin_sips.ingest_path_or_key IS 'Fully qualified path (FTP) or key (S3) for the incoming object/SIP.';
COMMENT ON COLUMN public.sipin_sips.bag_filesize IS 'Filesize of the zipped bag.';
COMMENT ON COLUMN public.sipin_sips.pid IS 'Persistent Identifier. The main ID of the object within meemoo.';
COMMENT ON COLUMN public.sipin_sips.mh_record_id IS 'MediaHaven record ID.';
COMMENT ON COLUMN public.sipin_sips.first_event_date IS 'Datetime for the first event for this correlation ID.';
COMMENT ON COLUMN public.sipin_sips.last_event_type IS 'Last seen event type for this correlation ID.';
COMMENT ON COLUMN public.sipin_sips.last_event_date IS 'Datetime for the last event for this correlation ID.';
COMMENT ON COLUMN public.sipin_sips.status IS 'More human friendly status: correlates one-to-one with the last event type.';