use xkbcommon::xkb;

use super::key_mapper::KeyMapper;

pub struct XcbKeyMapper {
    state: xkb::State,
}

impl KeyMapper for XcbKeyMapper {
    fn update_state(&mut self, _depressed_keys: &[u32], _engaged_toggles: &[u32]) {}
    fn translate_keypress(&mut self, _key_stroke: u32) -> Option<String> {
        Some(String::from("Not yet implemented"))
    }
}

pub fn new(state: xkb::State) -> XcbKeyMapper {
    XcbKeyMapper { state }
}
