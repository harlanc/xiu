use crate::channels::define::ChannelData;

#[derive(Clone)]
pub struct Gop {
    datas: Vec<ChannelData>,
}

impl Default for Gop {
    fn default() -> Self {
        Self::new()
    }
}

impl Gop {
    pub fn new() -> Self {
        Self { datas: Vec::new() }
    }

    pub fn save_gop_data(&mut self, data: ChannelData, is_key_frame: bool) {
        if is_key_frame {
            self.datas.clear();
        }

        self.datas.push(data);
    }

    pub fn get_gop_data(&self) -> Vec<ChannelData> {
        self.datas.clone()
    }

    pub fn len(&self) -> usize {
        self.datas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
