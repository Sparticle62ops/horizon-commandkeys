//! 1:1 Rust port of the config/profile editor model: the list orderings,
//! copy, add and remove operations the screens drove, shared by the future
//! screen host (the screens themselves are a host seam, no widget hook yet).

use crate::config::{Config, Profile};

/// java: ConfigScreen's profile list model: select, move, copy, remove.
pub struct ProfileListModel<'a> {
    pub config: &'a mut Config,
    /// The selected profile index.
    pub selected: usize,
}

impl<'a> ProfileListModel<'a> {
    /// java: the copy button: a custom copy at the end of the list.
    pub fn copy_selected(&mut self) {
        let source = self.selected;
        self.config.add_copy_profile(source);
    }

    /// java: the remove button, with the default-index clamp upstream's
    /// validate applies.
    pub fn remove_selected(&mut self) {
        let index = self.selected;
        if index < self.config.profiles.len() {
            self.config.remove_profile(index);
            self.selected = self.selected.min(self.config.profiles.len().saturating_sub(1));
        }
    }

    /// java: activateProfile(index).
    pub fn activate(&mut self, index: i32) {
        self.config.activate_profile(index);
    }
}

/// java: ProfileEditorScreen's link editor model: add replaces the link in
/// whatever profile held it, remove frees it.
pub struct LinkListModel<'a> {
    pub profiles: Vec<&'a mut Profile>,
}

impl<'a> LinkListModel<'a> {
    /// java: Profile#forceAddLink semantics at the list level: a link can
    /// belong to one profile, so assigning it here removes it there.
    pub fn assign_link(&mut self, profile_index: usize, link: String) {
        for (i, profile) in self.profiles.iter_mut().enumerate() {
            if i == profile_index {
                profile.force_add_link(link.clone());
            } else {
                profile.links.retain(|l| l != &link);
            }
        }
    }

    /// java: the remove-link button.
    pub fn remove_link(&mut self, profile_index: usize, link: &str) {
        if let Some(profile) = self.profiles.get_mut(profile_index) {
            profile.remove_link(link);
        }
    }
}

/// java: MacroScreen's message list model: add, set, move, remove, with the
/// cleanup pass the editor runs on change.
pub struct MessageListModel {
    /// The active profile's macro index.
    pub macro_index: usize,
}

#[allow(dead_code)]
impl MessageListModel {
    /// java: moveMessage(sourceIndex, destIndex) through the profile.
    pub fn move_message(
        &self,
        profile: &mut Profile,
        source_index: usize,
        dest_index: usize,
    ) -> bool {
        if let Some(m) = profile.macros.get_mut(self.macro_index) {
            m.move_message(source_index, dest_index)
        } else {
            false
        }
    }

    /// java: the add button: an empty message, then cleanup.
    pub fn add_message(&self, profile: &mut Profile) {
        if let Some(m) = profile.macros.get_mut(self.macro_index) {
            m.messages.push(crate::config::Message::default());
            m.cleanup_messages();
        }
    }

    /// java: the remove button: drop and cleanup.
    pub fn remove_message(&self, profile: &mut Profile, index: usize) {
        if let Some(m) = profile.macros.get_mut(self.macro_index) {
            if index < m.messages.len() {
                m.messages.remove(index);
                m.cleanup_messages();
            }
        }
    }
}
