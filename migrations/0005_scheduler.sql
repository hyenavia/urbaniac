-- Antrean job persisten. Satu scheduler membaca job yang jatuh tempo; tidak ada timer per aktivitas.
CREATE TABLE scheduled_jobs (
    id          BIGSERIAL PRIMARY KEY,
    kind        TEXT        NOT NULL,
    ref_id      BIGINT      NOT NULL,
    run_at      TIMESTAMPTZ NOT NULL,
    status      TEXT        NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'done', 'failed')),
    attempts    INTEGER     NOT NULL DEFAULT 0,
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ
);
CREATE INDEX idx_jobs_due ON scheduled_jobs(run_at) WHERE status = 'pending';
CREATE UNIQUE INDEX uq_jobs_kind_ref ON scheduled_jobs(kind, ref_id);
