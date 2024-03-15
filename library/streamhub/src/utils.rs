use rand::Rng;
use serde::{Serialize, Serializer};
use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Copy)]
pub struct Uuid {
    value: [char; 16],
    random_count: RandomDigitCount,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Copy)]
pub enum RandomDigitCount {
    #[default]
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

fn u8_to_enum(digit: u8) -> RandomDigitCount {
    match digit {
        0 => RandomDigitCount::Zero,
        1 => RandomDigitCount::One,
        2 => RandomDigitCount::Two,
        3 => RandomDigitCount::Three,
        4 => RandomDigitCount::Four,
        5 => RandomDigitCount::Five,
        6 => RandomDigitCount::Six,
        _ => RandomDigitCount::Zero,
    }
}

impl Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Uuid {
    pub fn from_str2(uuid: &str) -> Option<Uuid> {
        let length = uuid.len();
        if !(10..=16).contains(&length) {
            return None;
        }

        let random_count = u8_to_enum(length as u8 - 10);
        let mut value: [char; 16] = ['\0'; 16];
        for (i, c) in uuid.chars().enumerate() {
            value[i] = c;
        }

        Some(Uuid {
            value,
            random_count,
        })
    }
    pub fn new(random_digit_count: RandomDigitCount) -> Self {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
        let seconds = match duration {
            Ok(result) => result.as_secs(),
            _ => 0,
        };

        let seconds_str = seconds.to_string();
        let mut value: [char; 16] = ['\0'; 16];
        for (i, c) in seconds_str.chars().enumerate() {
            if i >= 10 {
                break;
            }
            value[i] = c;
        }

        let mut rng = rand::thread_rng();
        let random_size = random_digit_count as usize;
        for i in 0..random_size {
            let number = rng.gen_range(0..=9);
            if let Some(c) = std::char::from_digit(number, 10) {
                value[10 + i] = c;
            }
        }

        // let count = random_digit_count as u8;
        // let mut rng = rand::thread_rng();
        // let random_digit_str: String = (0..count)
        //     .map(|_| rng.gen_range(0..=9).to_string())
        //     .collect();

        // let seconds_str = format!("{}", seconds);
        // let random_num_str = if count > 0 {
        //     format!("-{}", random_digit_str)
        // } else {
        //     String::from("")
        // };

        Self {
            value,
            random_count: random_digit_count,
        }
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val: String = self
            .value
            .iter()
            .take(10 + self.random_count as usize)
            .collect();
        write!(f, "{}", &val)
    }
}

#[cfg(test)]
mod tests {
    

    use super::Uuid;

    #[test]
    fn test_uuid() {
        let id = Uuid::new(super::RandomDigitCount::Four);

        let s = id.to_string();

        let serialized = serde_json::to_string(&id).unwrap();

        println!("serialized:{serialized}");

        println!("{s}");

        if let Some(u) = Uuid::from_str2(&s) {
            println!("{:?}", u.to_string());
        }
    }
}
