use {
    super::errors::MediaError,
    aws_sdk_s3::Client as S3Client,
    aws_sdk_s3::primitives::ByteStream,
    aws_sdk_s3::types::ObjectCannedAcl,
    bytes::BytesMut,
    std::{fs, fs::File, io::Write},
};

pub struct Ts {
    ts_number: u32,
    live_path: String,
    s3_client: Option<S3Client>,
    s3_bucket: Option<String>,
    s3_prefix: String,
}

impl Ts {
    pub fn new(path: String, s3_client: Option<S3Client>, s3_bucket: Option<String>, s3_prefix: String) -> Self {
        fs::create_dir_all(path.clone()).unwrap();

        Self {
            ts_number: 0,
            live_path: path,
            s3_client,
            s3_bucket,
            s3_prefix,
        }
    }
    pub async fn write(&mut self, data: BytesMut) -> Result<(String, String), MediaError> {
        let ts_file_path;
        let ts_file_name = format!("{}.ts", self.ts_number);
        self.ts_number += 1;

        if let (Some(client), Some(bucket)) = (&self.s3_client, &self.s3_bucket) {
            let body = ByteStream::from(data.to_vec());
            ts_file_path = format!("{}/{}", self.s3_prefix, ts_file_name);
                
            let _result = client
                .put_object()
                .bucket(bucket)
                .key(&ts_file_path)
                .acl(ObjectCannedAcl::PublicRead)
                .body(body)
                .send()
                .await
                .map_err(|e| MediaError {
                    value: super::errors::MediaErrorValue::IOError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("S3 upload failed: {}", e),
                    )),
                })?;
        } else {
            ts_file_path = format!("{}/{}", self.live_path, ts_file_name);
            let mut ts_file_handler = File::create(ts_file_path.clone())?;
            ts_file_handler.write_all(&data[..])?;
        }

        Ok((ts_file_name, ts_file_path))
    }

    pub async fn delete(&mut self, ts_file_path: String) {
        if let (Some(client), Some(bucket)) = (&self.s3_client, &self.s3_bucket) {
            let _result = client
                .delete_object()
                .bucket(bucket)
                .key(&ts_file_path)
                .send()
                .await
                .map_err(|e| MediaError {
                    value: super::errors::MediaErrorValue::IOError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("S3 delete failed: {}", e),
                    )),
                });
        } else {
            fs::remove_file(ts_file_path).unwrap();
        }
    }
}
