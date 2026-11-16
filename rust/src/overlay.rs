//! Backend-independent overlay policy. Network replies can only update existing content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Hidden,
    Preparing,
    Passive,
    Interactive,
    Closing,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HideReason {
    Explicit,
    Capture,
    Quit,
    Button,
    Escape,
    FocusLost,
    OtherApplication,
    GameGone,
    GameMinimized,
    GameInactive,
    TrackerFailure,
    ExternalLink,
    WindowClosed,
}
#[derive(Debug, Clone, Copy)]
pub struct DesktopState {
    pub game_exists: bool,
    pub game_visible: bool,
    pub game_active: bool,
    pub other_active: bool,
}

#[derive(Debug)]
pub struct Overlay {
    pub state: State,
    pub generation: u64,
    pub game_active: bool,
    pub rendered: bool,
    pub last_hide: Option<HideReason>,
    focus_epoch: u64,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            state: State::Hidden,
            generation: 0,
            game_active: false,
            rendered: false,
            last_hide: None,
            focus_epoch: 0,
        }
    }
}

impl Overlay {
    pub fn show(&mut self) -> Option<u64> {
        if !self.game_active {
            return None;
        }
        self.generation += 1;
        self.focus_restored();
        self.rendered = false;
        self.state = State::Preparing;
        Some(self.generation)
    }
    pub fn painted(&mut self) {
        if self.state == State::Preparing {
            self.rendered = true;
            self.state = State::Passive;
        }
    }
    pub fn interact(&mut self) -> bool {
        if self.state == State::Passive && self.rendered && self.game_active {
            self.state = State::Interactive;
            true
        } else {
            false
        }
    }
    pub fn hide(&mut self, reason: HideReason) {
        self.focus_restored();
        self.last_hide = Some(reason);
        self.state = State::Closing;
        self.generation += 1;
        self.rendered = false;
        self.state = State::Hidden;
    }
    pub fn fail(&mut self) {
        self.hide(HideReason::TrackerFailure);
        self.state = State::Error;
    }
    pub fn accepts(&self, generation: u64) -> bool {
        generation == self.generation
            && matches!(
                self.state,
                State::Preparing | State::Passive | State::Interactive
            )
    }
    pub fn focus_lost(&mut self) -> Option<u64> {
        if self.state != State::Interactive {
            return None;
        }
        self.focus_epoch = self.focus_epoch.wrapping_add(1);
        Some(self.focus_epoch)
    }
    pub fn focus_restored(&mut self) {
        self.focus_epoch = self.focus_epoch.wrapping_add(1);
    }
    pub fn should_hide_after_focus_loss(
        &self,
        token: u64,
        main_active: bool,
        owned_popup: bool,
    ) -> bool {
        self.state == State::Interactive
            && self.focus_epoch == token
            && !main_active
            && !owned_popup
    }
    /// A return to the game updates availability but never reopens a panel.
    pub fn desktop_changed(&mut self, s: DesktopState) -> Option<HideReason> {
        self.game_active = s.game_active;
        if self.state == State::Hidden {
            return None;
        }
        if s.other_active {
            Some(HideReason::OtherApplication)
        } else if !s.game_exists {
            Some(HideReason::GameGone)
        } else if !s.game_visible {
            Some(HideReason::GameMinimized)
        } else if !s.game_active && self.state != State::Interactive {
            Some(HideReason::GameInactive)
        } else {
            None
        }
    }
    /// An edit queued before a newer search must not cancel that newer search.
    pub fn edited(&mut self, observed_generation: u64) -> bool {
        if !self.accepts(observed_generation) {
            return false;
        }
        self.generation += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focus_requires_first_render_and_explicit_action() {
        let mut o = Overlay {
            game_active: true,
            ..Overlay::default()
        };
        o.show();
        assert!(!o.interact());
        o.painted();
        assert_eq!(o.state, State::Passive);
        assert!(o.interact());
    }
    #[test]
    fn late_network_reply_cannot_reopen_or_activate() {
        let mut o = Overlay {
            game_active: true,
            ..Overlay::default()
        };
        let g = o.show().unwrap();
        o.painted();
        o.hide(HideReason::Explicit);
        assert!(!o.accepts(g));
        assert!(!o.interact());
    }
    #[test]
    fn cannot_show_over_another_application() {
        assert_eq!(Overlay::default().show(), None);
    }
    #[test]
    fn failure_revokes_input_and_stale_replies() {
        let mut o = Overlay {
            game_active: true,
            ..Overlay::default()
        };
        let g = o.show().unwrap();
        o.painted();
        o.interact();
        o.fail();
        assert!(!o.rendered);
        assert!(!o.accepts(g));
        assert!(!o.interact());
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;
    #[test]
    fn an_older_queued_edit_cannot_cancel_a_newer_search() {
        let mut overlay = Overlay {
            game_active: true,
            ..Default::default()
        };
        let frame = overlay.show().unwrap();
        overlay.painted();
        assert!(overlay.edited(frame));
        let edited = overlay.generation;
        assert!(!overlay.edited(frame));
        // Search reads the latest widget values before its queued edit arrives.
        overlay.generation += 1;
        let search = overlay.generation;
        assert!(!overlay.edited(edited));
        assert!(overlay.accepts(search));
        overlay.hide(HideReason::Explicit);
        assert!(!overlay.edited(overlay.generation));
    }
}

#[cfg(test)]
mod desktop_tests {
    use super::*;
    fn game() -> DesktopState {
        DesktopState {
            game_exists: true,
            game_visible: true,
            game_active: true,
            other_active: false,
        }
    }
    fn visible() -> Overlay {
        let mut o = Overlay {
            game_active: true,
            ..Default::default()
        };
        o.show();
        o.painted();
        o
    }
    #[test]
    fn minimizing_or_closing_invalidates_even_an_interactive_panel_and_return_never_reopens() {
        for (status, reason) in [
            (
                DesktopState {
                    game_visible: false,
                    game_active: false,
                    ..game()
                },
                HideReason::GameMinimized,
            ),
            (
                DesktopState {
                    game_exists: false,
                    game_visible: false,
                    game_active: false,
                    ..game()
                },
                HideReason::GameGone,
            ),
        ] {
            let mut o = visible();
            o.interact();
            let generation = o.generation;
            assert_eq!(o.desktop_changed(status), Some(reason));
            o.hide(reason);
            assert!(!o.accepts(generation));
            assert_eq!(o.last_hide, Some(reason));
            assert_eq!(o.desktop_changed(game()), None);
            assert_eq!(o.state, State::Hidden);
        }
    }
    #[test]
    fn explicit_interaction_can_own_focus_but_another_app_always_wins() {
        let inactive = DesktopState {
            game_active: false,
            ..game()
        };
        let mut passive = visible();
        assert_eq!(
            passive.desktop_changed(inactive),
            Some(HideReason::GameInactive)
        );
        let mut interactive = visible();
        interactive.interact();
        assert_eq!(interactive.desktop_changed(inactive), None);
        assert_eq!(
            interactive.desktop_changed(DesktopState {
                other_active: true,
                ..inactive
            }),
            Some(HideReason::OtherApplication)
        );
    }
}

#[cfg(test)]
mod popup_focus_tests {
    use super::*;
    fn interactive() -> Overlay {
        let mut o = Overlay {
            game_active: true,
            ..Default::default()
        };
        o.show();
        o.painted();
        o.interact();
        o
    }
    #[test]
    fn an_owned_popup_or_restored_focus_never_hides_the_main_panel() {
        let mut o = interactive();
        let token = o.focus_lost().unwrap();
        assert!(!o.should_hide_after_focus_loss(token, false, true));
        assert!(!o.should_hide_after_focus_loss(token, true, false));
        o.focus_restored();
        assert!(!o.should_hide_after_focus_loss(token, false, false));
    }
    #[test]
    fn a_query_edit_cannot_cancel_real_focus_loss_but_a_new_panel_cancels_old_checks() {
        let mut o = interactive();
        let token = o.focus_lost().unwrap();
        o.edited(o.generation);
        assert!(o.should_hide_after_focus_loss(token, false, false));
        o.hide(HideReason::Explicit);
        o.show();
        o.painted();
        o.interact();
        assert!(!o.should_hide_after_focus_loss(token, false, false));
    }
}
