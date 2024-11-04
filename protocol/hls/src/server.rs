use {
    axum::{
        body::Body,
        extract::{Request, State},
        handler::Handler,
        http::StatusCode,
        response::Response,
    },
    commonlib::auth::{Auth, SecretCarrier},
    std::net::SocketAddr,
    tokio::{fs::File, net::TcpListener},
    tokio_util::codec::{BytesCodec, FramedRead},
};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

static NOTFOUND: &[u8] = b"Not Found";
static UNAUTHORIZED: &[u8] = b"Unauthorized";

#[derive(Debug)]
enum HlsFileType {
    Playlist,
    Segment,
}

impl HlsFileType {
    const CONTENT_TYPE_PLAYLIST: &'static str = "application/vnd.apple.mpegurl";
    const CONTENT_TYPE_SEGMENT: &'static str = "video/mp2t";

    fn content_type(&self) -> &str {
        match self {
            Self::Playlist => Self::CONTENT_TYPE_PLAYLIST,
            Self::Segment => Self::CONTENT_TYPE_SEGMENT,
        }
    }
}

#[derive(Debug)]
struct HlsPath {
    app_name: String,
    stream_name: String,
    file_name: String,
    file_type: HlsFileType,
}

impl HlsPath {
    const M3U8_EXT: &'static str = "m3u8";
    const TS_EXT: &'static str = "ts";

    fn parse(path: &str) -> Option<Self> {
        if path.is_empty() || path.contains("..") {
            return None;
        }

        let mut parts = path[1..].split('/');
        let app_name = parts.next()?;
        let stream_name = parts.next()?;
        let file_part = parts.next()?;
        if parts.next().is_some() {
            return None;
        }

        let (file_name, ext) = file_part.rsplit_once('.')?;
        if file_name.is_empty() {
            return None;
        }

        let file_type = match ext {
            Self::M3U8_EXT => HlsFileType::Playlist,
            Self::TS_EXT => HlsFileType::Segment,
            _ => return None,
        };

        Some(Self {
            app_name: app_name.into(),
            stream_name: stream_name.into(),
            file_name: file_name.into(),
            file_type,
        })
    }

    fn to_file_path(&self) -> String {
        let ext = match self.file_type {
            HlsFileType::Playlist => Self::M3U8_EXT,
            HlsFileType::Segment => Self::TS_EXT,
        };
        format!(
            "./{}/{}/{}.{}",
            self.app_name, self.stream_name, self.file_name, ext
        )
    }
}

fn response_unauthorized() -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(UNAUTHORIZED.into())
        .unwrap()
}

fn response_not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

async fn response_file(hls_path: &HlsPath) -> Response<Body> {
    let file_path = hls_path.to_file_path();

    if let Ok(file) = File::open(&file_path).await {
        let builder = Response::builder().header("Content-Type", hls_path.file_type.content_type());

        // Serve a file by asynchronously reading it by chunks using tokio-util crate.
        let stream = FramedRead::new(file, BytesCodec::new());
        return builder.body(Body::from_stream(stream)).unwrap();
    }

    response_not_found()
}

async fn handle_connection(State(auth): State<Option<Auth>>, req: Request<Body>) -> Response<Body> {
    let path = req.uri().path();
    let query_string = req.uri().query().map(|s| s.to_string());

    let hls_path = match HlsPath::parse(path) {
        Some(p) => p,
        None => return response_not_found(),
    };

    if let (Some(auth_val), HlsFileType::Playlist) = (auth.as_ref(), &hls_path.file_type) {
        if auth_val
            .authenticate(
                &hls_path.stream_name,
                &query_string.map(SecretCarrier::Query),
                true,
            )
            .is_err()
        {
            return response_unauthorized();
        }
    }

    response_file(&hls_path).await
}

pub async fn run(port: usize, auth: Option<Auth>) -> Result<()> {
    let listen_address = format!("0.0.0.0:{port}");
    let sock_addr: SocketAddr = listen_address.parse().unwrap();

    let listener = TcpListener::bind(sock_addr).await?;

    log::info!("Hls server listening on http://{}", sock_addr);

    let handle_connection = handle_connection.with_state(auth);

    axum::serve(listener, handle_connection.into_make_service()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HlsFileType, HlsPath};

    #[test]
    fn test_hls_path_parse() {
        // Playlist
        let playlist = HlsPath::parse("/live/stream/stream.m3u8").unwrap();
        assert_eq!(playlist.app_name, "live");
        assert_eq!(playlist.stream_name, "stream");
        assert_eq!(playlist.file_name, "stream");
        assert!(matches!(playlist.file_type, HlsFileType::Playlist));
        assert_eq!(playlist.to_file_path(), "./live/stream/stream.m3u8");
        assert_eq!(
            playlist.file_type.content_type(),
            "application/vnd.apple.mpegurl"
        );

        // Segment
        let segment = HlsPath::parse("/live/stream/123.ts").unwrap();
        assert_eq!(segment.app_name, "live");
        assert_eq!(segment.stream_name, "stream");
        assert_eq!(segment.file_name, "123");
        assert!(matches!(segment.file_type, HlsFileType::Segment));
        assert_eq!(segment.to_file_path(), "./live/stream/123.ts");
        assert_eq!(segment.file_type.content_type(), "video/mp2t");

        // Negative
        assert!(HlsPath::parse("").is_none());
        assert!(HlsPath::parse("/invalid").is_none());
        assert!(HlsPath::parse("/too/many/parts/of/path.m3u8").is_none());
        assert!(HlsPath::parse("/live/stream/invalid.mp4").is_none());
        assert!(HlsPath::parse("/live/stream/../../etc/passwd").is_none());
        assert!(HlsPath::parse("/live/stream/...").is_none());
        assert!(HlsPath::parse("/live/stream.m3u8").is_none());
        assert!(HlsPath::parse("/live/stream.ts").is_none());
        assert!(HlsPath::parse("/live/stream/").is_none());
        assert!(HlsPath::parse("/live/stream.m3u8").is_none());
        assert!(HlsPath::parse("/live/stream.ts").is_none());
        assert!(HlsPath::parse("/live/stream/file.").is_none());
        assert!(HlsPath::parse("/live/stream/.m3u8").is_none());
        assert!(HlsPath::parse("/live/stream/file.M3U8").is_none());
        assert!(HlsPath::parse("/live/stream/file.TS").is_none());
    }
}
