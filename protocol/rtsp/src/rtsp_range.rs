use crate::global_trait::Marshal;

use super::global_trait::Unmarshal;
use super::rtsp_utils;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum RtspRangeType {
    #[default]
    NPT,
    CLOCK,
}

#[derive(Debug, Clone, Default)]
pub struct RtspRange {
    range_type: RtspRangeType,
    begin: i64,
    end: Option<i64>,
}

impl Unmarshal for RtspRange {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut rtsp_range = RtspRange::default();

        let kv: Vec<&str> = raw_data.splitn(2, '=').collect();
        if kv.len() < 2 {
            return None;
        }

        match kv[0] {
            "clock" => {
                rtsp_range.range_type = RtspRangeType::CLOCK;
                let ranges: Vec<&str> = kv[1].split('-').collect();

                let get_clock_time = |range_time: &str| -> i64 {
                    let datetime =
                        match chrono::NaiveDateTime::parse_from_str(range_time, "%Y%m%dT%H%M%SZ") {
                            Ok(dt) => dt,
                            Err(err) => {
                                println!("get_clock_time error: {err}");
                                return -1;
                            }
                        };
                    datetime.and_utc().timestamp()
                };

                rtsp_range.begin = get_clock_time(ranges[0]);
                if ranges.len() > 1 {
                    rtsp_range.end = Some(get_clock_time(ranges[1]));
                }
            }
            "npt" => {
                rtsp_range.range_type = RtspRangeType::NPT;
                let ranges: Vec<&str> = kv[1].split('-').collect();

                let get_npt_time = |range_time: &str| -> i64 {
                    if let (Some(hour), Some(minute), Some(second), mill) =
                        rtsp_utils::scanf!(range_time, |c| c == ':' || c == '.', i64, i64, i64, i64)
                    {
                        let mut result = (hour * 3600 + minute * 60 + second) * 1000;
                        if let Some(m) = mill {
                            result += m;
                        }
                        result
                    } else {
                        0
                    }
                };

                match ranges[0] {
                    "now" => {
                        rtsp_range.begin = 0;
                    }
                    _ => {
                        rtsp_range.begin = get_npt_time(ranges[0]);
                    }
                }

                if ranges.len() == 2 && !ranges[1].is_empty() {
                    rtsp_range.end = Some(get_npt_time(ranges[1]));
                }
            }
            _ => {
                log::info!("{} not parsed..", kv[0]);
            }
        }

        Some(rtsp_range)
    }
}

impl Marshal for RtspRange {
    fn marshal(&self) -> String {
        String::default()
    }
}

#[cfg(test)]
mod tests {

    use super::RtspRange;
    use crate::global_trait::Unmarshal;

    #[test]
    fn test_parse_transport() {
        //a=range:
        //a=range:npt=now-
        //a=range:npt=0-
        let parser = RtspRange::unmarshal("clock=20220520T064812Z-20230520T064816Z").unwrap();

        println!(" parser: {parser:?}");

        let parser1 = RtspRange::unmarshal("npt=now-").unwrap();

        println!(" parser1: {:?}, {}", parser1, parser1.end.is_none());

        let parser2 = RtspRange::unmarshal("npt=0-").unwrap();
        println!(" parser2: {:?}, {}", parser2, parser2.end.is_none());
    }
}
