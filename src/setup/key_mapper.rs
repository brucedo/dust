use std::marker::Send;

pub trait KeyMapper {
    fn update_state(&mut self, depressed_keys: &[u32], engaged_toggles: &[u32]);
    fn translate_keypress(&mut self, key_stroke: u32) -> Option<String>;
}
