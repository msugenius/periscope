use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyTransition {
    Pressed,
    Released,
}

#[derive(Debug, Default)]
pub(crate) struct ShortcutDispatch {
    pressed: HashSet<u32>,
    recording: bool,
}

impl ShortcutDispatch {
    pub(crate) fn transition(&mut self, id: u32, transition: KeyTransition) -> bool {
        match transition {
            KeyTransition::Released => {
                self.pressed.remove(&id);
                false
            }
            KeyTransition::Pressed => !self.recording && self.pressed.insert(id),
        }
    }

    pub(crate) fn set_recording(&mut self, recording: bool) {
        self.recording = recording;
    }

    pub(crate) fn is_recording(&self) -> bool {
        self.recording
    }

    pub(crate) fn clear(&mut self) {
        self.pressed.clear();
    }
}
