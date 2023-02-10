use {
    //https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
    anyhow::Result,
    clap::{value_parser, Arg, ArgGroup, Command},
    std::env,
    tokio,
    tokio::signal,
    xiu::config::{config, config::Config},
    xiu::service::Service,
};

#[tokio::main]
async fn main() -> Result<()> {
    let log_levels = vec!["trace", "debug", "info", "warn", "error", "debug"];

    let mut cmd = Command::new("XIU")
        .bin_name("xiu")
        .version("0.3.0")
        .author("HarlanC <harlanc@foxmail.com>")
        .about("A live media server, hope you love it!!!")
        .arg(
            Arg::new("config_file_path")
                .long("config")
                .short('c')
                .value_name("path")
                .help("Specify the xiu server configuration file path.")
                .value_parser(value_parser!(String))
                .conflicts_with_all(["rtmp", "httpflv", "hls", "log"]),
        )
        .arg(
            Arg::new("rtmp")
                .long("rtmp")
                .short('r')
                .value_name("port")
                .help("Specify the RTMP listening port.")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("httpflv")
                .long("httpflv")
                .short('f')
                .value_name("port")
                .help("Specify the HTTP-FLV listening port.")
                .value_parser(value_parser!(usize))
                .conflicts_with("config_file_path"),
        )
        .arg(
            Arg::new("hls")
                .long("hls")
                .short('s')
                .value_name("port")
                .help("Specify the HLS listening port.")
                .value_parser(value_parser!(usize))
                .conflicts_with("config_file_path"),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .short('l')
                .value_name("level")
                .help("Specify the log level.")
                .value_parser(log_levels)
                .conflicts_with("config_file_path"),
        )
        .group(
            ArgGroup::new("vers")
                .args(["config_file_path", "rtmp"])
                .required(true),
        );

    let args: Vec<String> = env::args().collect();
    if 1 == args.len() {
        cmd.print_help()?;
        return Ok(());
    }

    let matches = cmd.clone().get_matches();

    let config = if let Some(path) = matches.get_one::<String>("config_file_path") {
        let config = config::load(path);
        match config {
            Ok(val) => val,
            Err(err) => {
                println!("{}: {}", path, err);
                return Ok(());
            }
        }
    } else {
        let rtmp_port = match matches.get_one::<usize>("rtmp") {
            Some(val) => *val,
            None => 0,
        };
        let httpflv_port = match matches.get_one::<usize>("httpflv") {
            Some(val) => *val,
            None => 0,
        };
        let hls_port = match matches.get_one::<usize>("hls") {
            Some(val) => *val,
            None => 0,
        };
        let log_level = match matches.get_one::<String>("log") {
            Some(val) => val.clone(),
            None => String::from("info"),
        };
        Config::new(rtmp_port, httpflv_port, hls_port, log_level)
    };

    /*set log level*/
    if let Some(log_config_value) = &config.log {
        env::set_var("RUST_LOG", log_config_value.level.clone());
    } else {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    /*run the service*/
    let mut serivce = Service::new(config);
    serivce.run().await?;

    // log::info!("log info...");
    // log::warn!("log warn...");
    // log::error!("log err...");
    // log::trace!("log trace...");
    // log::debug!("log debug...");

    signal::ctrl_c().await?;
    Ok(())
}
