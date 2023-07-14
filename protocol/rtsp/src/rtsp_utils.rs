use rand::Rng;

macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

pub(crate) use scanf;

#[cfg(test)]
mod tests {

    #[test]
    fn test_scanf() {
        let str_a = "18:23:08";

        if let (Some(a), Some(b), Some(c), _) =
            scanf!(str_a, |c| c == ':' || c == '.', i64, i64, i64, i64)
        {
            println!("a:{} b:{} c:{} ", a, b, c);
        }
    }
}
