#[derive(Debug, Clone)]
pub struct HlsRemuxerConfig {
    pub need_record: bool,
    pub path: Option<String>,
    pub fragment: Option<i64>,
    pub aof_ratio: Option<i64>,
    pub live_ts_count: Option<usize>,
    pub prefix: Option<String>,
    pub s3: Option<S3UploadConfig>,
}

#[derive(Debug, Clone)]
pub struct S3UploadConfig {
    pub bucket: String,
    pub prefix: Option<String>,
}
