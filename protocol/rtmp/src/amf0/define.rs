pub enum Amf0ValueType {
    Number(f64),
    Boolean(bool),
    UTF8String(String),
    Object(UnOrderedMap),
    Null,
    EcmaArray(UnOrderedMap),
    LongUTF8String(String),
    END,
}

pub struct Amf0Object {
    key: String,
    Value: Amf0ValueType,
}

pub struct UnOrderedMap {
    properties: Vec<Amf0Object>,
}

impl UnOrderedMap {
    pub fn new() -> UnOrderedMap {
        UnOrderedMap {
            properties: Vec::new(),
        }
    }
    fn insert(self, key: String, val: Amf0ValueType) -> Option<Amf0ValueType> {
        for i in &self.properties {
            if i.key == key {
                let tmpVal = i.Value;
                i.Value = val;
                return Some(tmpVal);
            }
        }

        let obj = Amf0Object {
            key: key,
            Value: val,
        };
        self.properties.push(obj);

        None
    }
    fn get_by_key(self, key: String) -> Option<Amf0ValueType> {
        for i in self.properties {
            if i.key == key {
                return Some(i.Value);
            }
        }
        None
    }

    // fn get<I>(self, idx: I) -> Option<Amf0Object> {
    //     self.properties[idx]
    // }

    fn len(self) -> usize {
        self.properties.len()
    }
}
