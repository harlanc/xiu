mod amf0_markers {
    pub const NUMBER: u8 = 0x00;
    pub const BOOLEAN: u8 = 0x01;
    pub const STRING: u8 = 0x02;
    pub const OBJECT: u8 = 0x03;
    pub const NULL: u8 = 0x05;
    pub const ECMA_ARRAY: u8 = 0x08;
    pub const OBJECT_END: u8 = 0x09;
    pub const LONG_STRING: u8 = 0x0c;
}

enum Amf0ValueType {
    Number(f64),
    Boolean(bool),
    UTF8String(String),
    Object(UnOrderedMap),
    Null,
    EcmaArray(UnOrderedMap),
    LongUTF8String(String),
}

struct Amf0Object {
    key: String,
    Value: Amf0ValueType,
}

struct UnOrderedMap {
    properties: Vec<Amf0Object>,
}

impl UnOrderedMap {
    pub fn new() -> UnOrderedMap {
        UnOrderedMap {
            properties: Vec::new(),
        }
    }
    fn insert(self, key: String, val: Amf0ValueType) -> Option(Amf0ValueType) {
        for i in &self.properties {
            if i.key == key {
                let tmpVal = i.Value;
                i.Value = val;
                return Option(tmpVal);
            }
        }

        let obj = Amf0Object {
            key: key,
            Value: val,
        };
        self.properties.push(obj);

        Option(None)
    }
    fn get_by_key(self, key: String) -> Option(Amf0ValueType) {
        for i in self.properties {
            if i.key == key {
                return Option(i.key);
            }
        }
        Option(None)
    }

    fn get<I>(self, idx: I) -> Option(Amf0Object) {
        self.properties.get(idx)
    }

    fn len(self) -> usize {
        self.properties.len()
    }
}
